#!/usr/bin/env bash

set -euo pipefail

usage() {
    cat <<'EOF'
Usage:
  scripts/vast/remote_run_comparison.sh --pattern <pattern> [options]

Options:
  --pattern <pattern>      Comparison filter passed to test_comparison.sh
  --workdir <dir>          Remote bundle root. Defaults to current directory.
  --player-bin <path>      Prebuilt player binary. Defaults to <workdir>/bin/player.
  --single                 Forward --single to test_comparison.sh.
  --frame-test             Run frame-test mode instead of video comparison.
  --tracy                  Enable Tracy profiling (implies --frame-test).
                           Starts tracy-capture, runs player, saves .tracy + analysis.
  --no-headless            Do not pass --headless.
  --log-file <path>        Log path. Defaults to logs/remote_comparison_<timestamp>.log
  --max-frames <n>         Export MAX_FRAMES for hypothesis checks.
  --skip-render-probe      Skip the preflight render diagnostics probe.
  -h, --help               Show this message.
EOF
}

require_cmd() {
    local cmd="$1"
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "Missing required command: $cmd" >&2
        exit 1
    fi
}

pattern=""
workdir=""
player_bin=""
single=0
frame_test=0
tracy=0
headless=1
log_file=""
max_frames=""
skip_render_probe=0

while [ $# -gt 0 ]; do
    case "$1" in
        --pattern)
            pattern="${2:-}"
            shift 2
            ;;
        --workdir)
            workdir="${2:-}"
            shift 2
            ;;
        --player-bin)
            player_bin="${2:-}"
            shift 2
            ;;
        --single)
            single=1
            shift
            ;;
        --frame-test)
            frame_test=1
            shift
            ;;
        --tracy)
            tracy=1
            frame_test=1
            shift
            ;;
        --no-headless)
            headless=0
            shift
            ;;
        --log-file)
            log_file="${2:-}"
            shift 2
            ;;
        --max-frames)
            max_frames="${2:-}"
            shift 2
            ;;
        --skip-render-probe)
            skip_render_probe=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

if [ -z "$pattern" ]; then
    echo "--pattern is required" >&2
    usage >&2
    exit 1
fi

if [ -z "$workdir" ]; then
    workdir="$(pwd)"
fi
if [ -z "$player_bin" ]; then
    player_bin="${workdir}/bin/player"
fi
if [ -z "$log_file" ]; then
    log_file="${workdir}/logs/remote_comparison_$(date +%Y%m%d_%H%M%S).log"
fi

require_cmd bash
require_cmd python3
require_cmd ffmpeg
require_cmd ffprobe
require_cmd bc

if [ ! -d "$workdir" ]; then
    echo "Remote workdir does not exist: $workdir" >&2
    exit 1
fi

cd "$workdir"

if [ ! -x "$player_bin" ]; then
    echo "Prebuilt player binary is missing or not executable: $player_bin" >&2
    exit 1
fi
if [ ! -f "./test_comparison.sh" ]; then
    echo "test_comparison.sh is missing in $workdir" >&2
    exit 1
fi
if [ ! -f "./comparison_config.toml" ]; then
    echo "comparison_config.toml is missing in $workdir" >&2
    exit 1
fi
if [ ! -d "./assets/projects" ]; then
    echo "assets/projects is missing in $workdir" >&2
    exit 1
fi

mkdir -p "$(dirname "$log_file")"

export AM_SKIP_BUILD=1
export AM_PLAYER_BIN="$player_bin"
export CARGO_MANIFEST_DIR="$workdir"
export AM_RENDER_DIAGNOSTICS="${AM_RENDER_DIAGNOSTICS:-1}"
if [ -n "$max_frames" ]; then
    export MAX_FRAMES="$max_frames"
fi

cmd=(bash ./test_comparison.sh "$pattern")
if [ "$single" -eq 1 ]; then
    cmd+=(--single)
fi
if [ "$frame_test" -eq 1 ]; then
    cmd+=(--frame-test)
fi
if [ "$headless" -eq 1 ]; then
    cmd+=(--headless)
fi

{
    echo "[remote] workdir=$workdir"
    echo "[remote] player_bin=$player_bin"
    echo "[remote] pattern=$pattern"
    echo "[remote] uname=$(uname -a)"
    echo "[remote] WGPU_BACKEND=${WGPU_BACKEND:-<unset>}"
    echo "[remote] WGPU_FORCE_FALLBACK_ADAPTER=${WGPU_FORCE_FALLBACK_ADAPTER:-<unset>}"
    echo "[remote] WGPU_ADAPTER_NAME=${WGPU_ADAPTER_NAME:-<unset>}"
    echo "[remote] WGPU_SETTINGS_PRIO=${WGPU_SETTINGS_PRIO:-<unset>}"
    echo "[remote] VK_ICD_FILENAMES=${VK_ICD_FILENAMES:-<unset>}"
    echo "[remote] MESA_VK_DEVICE_SELECT=${MESA_VK_DEVICE_SELECT:-<unset>}"
    echo "[remote] DRI_PRIME=${DRI_PRIME:-<unset>}"
    printf '[remote] command='
    printf '%q ' "${cmd[@]}"
    echo
} | tee "$log_file"

if command -v vulkaninfo >/dev/null 2>&1; then
    {
        echo "[remote] vulkaninfo --summary"
        vulkaninfo --summary 2>&1 || true
    } | tee -a "$log_file"
fi

# Abort if no discrete/NVIDIA GPU is detected (only llvmpipe/CPU present).
# Running on software rendering wastes Vast.ai budget and produces unreliable results.
if command -v vulkaninfo >/dev/null 2>&1; then
    gpu_devices="$(vulkaninfo --summary 2>&1 | grep -i 'deviceName' || true)"
    has_hw_gpu=0
    while IFS= read -r line; do
        case "$line" in
            *llvmpipe*|*lavapipe*|*swiftshader*|*SwiftShader*) ;;
            *deviceName*) has_hw_gpu=1 ;;
        esac
    done <<< "$gpu_devices"
    if [ -n "$gpu_devices" ] && [ "$has_hw_gpu" -eq 0 ]; then
        echo "[remote] FATAL: No hardware GPU detected by Vulkan. Only software renderers found:" | tee -a "$log_file"
        echo "$gpu_devices" | tee -a "$log_file"
        echo "[remote] Aborting to avoid wasting Vast.ai budget on software rendering." | tee -a "$log_file"
        exit 78
    fi
fi

if [ "$skip_render_probe" -ne 1 ]; then
    probe_cmd=("$player_bin" "$pattern")
    if [ "$headless" -eq 1 ]; then
        probe_cmd+=(--headless)
    fi

    {
        echo "[remote] render probe start"
        printf '[remote] render probe command='
        printf '%q ' "${probe_cmd[@]}"
        echo
    } | tee -a "$log_file"

    AM_RENDER_DIAGNOSTICS=1 \
    AM_RENDER_DIAGNOSTICS_ONLY=1 \
    CARGO_MANIFEST_DIR="$workdir" \
        "${probe_cmd[@]}" 2>&1 | tee -a "$log_file"
fi

# Start Tracy capture if --tracy is enabled
tracy_capture_pid=""
tracy_output=""
if [ "$tracy" -eq 1 ]; then
    tracy_capture_bin="${workdir}/bin/tracy-capture"
    tracy_csvexport_bin="${workdir}/bin/tracy-csvexport"
    if [ ! -x "$tracy_capture_bin" ]; then
        echo "[remote] WARNING: tracy-capture not found at $tracy_capture_bin, skipping Tracy capture" | tee -a "$log_file"
        tracy=0
    else
        safe_pattern=$(echo "$pattern" | tr '/' '_')
        tracy_output="${workdir}/tracy_${safe_pattern}.tracy"
        echo "[remote] Starting tracy-capture -> $tracy_output" | tee -a "$log_file"
        "$tracy_capture_bin" -o "$tracy_output" -f &
        tracy_capture_pid=$!
        sleep 2
        echo "[remote] tracy-capture started (pid=$tracy_capture_pid)" | tee -a "$log_file"
    fi
fi

"${cmd[@]}" 2>&1 | tee -a "$log_file"
status=${PIPESTATUS[0]}

# Stop Tracy capture and run analysis
if [ -n "$tracy_capture_pid" ]; then
    echo "[remote] Stopping tracy-capture..." | tee -a "$log_file"
    sleep 2
    kill -INT "$tracy_capture_pid" 2>/dev/null || true
    wait "$tracy_capture_pid" 2>/dev/null || true

    if [ -f "$tracy_output" ]; then
        tracy_size=$(du -h "$tracy_output" | cut -f1)
        echo "[remote] Tracy capture saved: $tracy_output ($tracy_size)" | tee -a "$log_file"

        # Run analysis with tracy-csvexport
        if [ -x "$tracy_csvexport_bin" ]; then
            echo "[remote] Running Tracy analysis..." | tee -a "$log_file"
            "$tracy_csvexport_bin" "$tracy_output" 2>/dev/null | python3 -c "
import sys, re

lines = sys.stdin.readlines()
# CSV fields: name,src_file,src_line,total_ns,total_perc,counts,mean_ns,min_ns,max_ns,std_ns
# Names may contain commas (generic types), so parse from the right
pattern = re.compile(r'^(.*?),(.*?),(\d+),(\d+),([0-9.]+),(\d+),([0-9.]+),(\d+),(\d+),([0-9.]+)$')

zones = []
for line in lines[1:]:
    m = pattern.match(line.strip())
    if not m:
        continue
    name = m.group(1)
    total_ns = int(m.group(4))
    counts = int(m.group(6))
    mean_ns = float(m.group(7))
    max_ns = int(m.group(9))
    zones.append((name, total_ns, counts, mean_ns, max_ns))

zones.sort(key=lambda z: z[1], reverse=True)

print(f'Total unique zones: {len(zones)}')
print(f'Total time: {sum(z[1] for z in zones) / 1_000_000_000:.2f} s')
print()
print(f'{\"Zone Name\":<70} {\"Total ms\":>10} {\"Count\":>8} {\"Avg ms\":>10} {\"Max ms\":>10}')
print('=' * 110)
for name, total_ns, counts, mean_ns, max_ns in zones[:50]:
    total_ms = total_ns / 1_000_000
    avg_ms = mean_ns / 1_000_000
    max_ms = max_ns / 1_000_000
    print(f'{name:<70} {total_ms:>10.2f} {counts:>8} {avg_ms:>10.3f} {max_ms:>10.2f}')
" 2>&1 | tee "${workdir}/tracy_analysis.txt" | tee -a "$log_file"
        fi
    else
        echo "[remote] WARNING: Tracy capture file not found at $tracy_output" | tee -a "$log_file"
    fi
fi

python3 - "$pattern" "$single" <<'PY' | tee -a "$log_file"
import json
import pathlib
import sys

pattern = sys.argv[1]
single = sys.argv[2] == "1"
path = pathlib.Path("test_results.json")

if not path.exists():
    print("[remote-summary] test_results.json missing")
    raise SystemExit(0)

try:
    data = json.loads(path.read_text())
except json.JSONDecodeError as exc:
    print(f"[remote-summary] invalid test_results.json: {exc}")
    raise SystemExit(0)

results = data.get("results", {})
if single:
    matched = {name: info for name, info in results.items() if name == pattern}
else:
    matched = {name: info for name, info in results.items() if name.startswith(pattern)}

if not matched:
    print("[remote-summary] no matching results yet")
    raise SystemExit(0)

counts = {"pass": 0, "warning": 0, "skip": 0, "cancelled": 0, "fail": 0, "other": 0}
for info in matched.values():
    status = info.get("status", "other")
    counts[status if status in counts else "other"] += 1

print(
    "[remote-summary] matched={} pass={} warning={} skip={} cancelled={} fail={}".format(
        len(matched),
        counts["pass"],
        counts["warning"],
        counts["skip"],
        counts["cancelled"],
        counts["fail"],
    )
)
PY

exit "$status"
