#!/usr/bin/env bash
#
# Tracy capture script — runs player with Tracy profiling and captures a .tracy trace file.
#
# Usage:
#   scripts/tracy_capture.sh <project_name> [options]
#
# Examples:
#   scripts/tracy_capture.sh revenge/revenge
#   scripts/tracy_capture.sh revenge/revenge --play-once
#   scripts/tracy_capture.sh revenge/revenge --play-once --analyze
#   scripts/tracy_capture.sh basic/shape/shape --duration 5
#
# Options:
#   --play-once       Play animation once then exit (uses frame-test play-once mode)
#   --duration SECS   Capture duration in seconds (default: 10, ignored with --play-once)
#   --analyze         Run tracy-csvexport after capture and print hotspot analysis
#   --output FILE     Output .tracy file path (default: tracy_<project>.tracy)
#   --debug           Use debug_tracy feature (includes ECS system names)
#   --headless        Run in headless mode (no window)
#   --release         Build in release mode (default: release)
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$BASE_DIR"

# Defaults
PROJECT=""
PLAY_ONCE=false
DURATION=10
ANALYZE=false
OUTPUT=""
TRACY_FEATURE="trace_tracy"
HEADLESS=true
RELEASE=true

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --play-once)   PLAY_ONCE=true; shift ;;
        --duration)    DURATION="$2"; shift 2 ;;
        --analyze)     ANALYZE=true; shift ;;
        --output)      OUTPUT="$2"; shift 2 ;;
        --debug)       TRACY_FEATURE="debug_tracy"; shift ;;
        --headless)    HEADLESS=true; shift ;;
        --no-headless) HEADLESS=false; shift ;;
        --release)     RELEASE=true; shift ;;
        --no-release)  RELEASE=false; shift ;;
        -*)            echo "Unknown option: $1"; exit 1 ;;
        *)             PROJECT="$1"; shift ;;
    esac
done

if [ -z "$PROJECT" ]; then
    echo "Usage: scripts/tracy_capture.sh <project_name> [options]"
    echo "  e.g.: scripts/tracy_capture.sh revenge/revenge --play-once --analyze"
    exit 1
fi

# Sanitize project name for filename
SAFE_NAME=$(echo "$PROJECT" | tr '/' '_')
if [ -z "$OUTPUT" ]; then
    OUTPUT="tracy_${SAFE_NAME}.tracy"
fi

# Build features
FEATURES="${TRACY_FEATURE}"
if [ "$HEADLESS" = true ]; then
    FEATURES="${FEATURES},headless-render"
fi
if [ "$PLAY_ONCE" = true ]; then
    FEATURES="${FEATURES},frame-test"
fi

PROFILE_FLAG=""
if [ "$RELEASE" = true ]; then
    PROFILE_FLAG="--release"
fi

echo "========================================"
echo "Tracy Capture: $PROJECT"
echo "========================================"
echo "Features: $FEATURES"
echo "Output:   $OUTPUT"
echo "Mode:     $([ "$PLAY_ONCE" = true ] && echo 'play-once' || echo "timed (${DURATION}s)")"
echo ""

# Build player
echo "[1/4] Building player with Tracy..."
cargo build -p bevy_alight_motion --example player --features "$FEATURES" $PROFILE_FLAG 2>&1 | tail -3

# Determine binary path
if [ "$RELEASE" = true ]; then
    PLAYER_BIN="target/release/examples/player"
else
    PLAYER_BIN="target/debug/examples/player"
fi

if [ ! -f "$PLAYER_BIN" ]; then
    echo "ERROR: Player binary not found at $PLAYER_BIN"
    exit 1
fi

# Start tracy-capture in background
echo "[2/4] Starting tracy-capture..."
tracy-capture -o "$OUTPUT" -f &
CAPTURE_PID=$!
# Give tracy-capture time to start listening
sleep 1

# Run player
echo "[3/4] Running player..."
PLAYER_ARGS="$PROJECT"
if [ "$HEADLESS" = true ]; then
    PLAYER_ARGS="--headless $PLAYER_ARGS"
fi

# Set play-once mode via environment variable
EXTRA_ENV=""
if [ "$PLAY_ONCE" = true ]; then
    export AM_FRAME_TEST_PLAY_ONCE=1
fi

if [ "$PLAY_ONCE" = true ]; then
    # In play-once mode, player exits after one animation playthrough
    CARGO_MANIFEST_DIR="$BASE_DIR" "$PLAYER_BIN" $PLAYER_ARGS 2>&1 | tail -30 || true
else
    # In timed mode, run for specified duration then kill
    CARGO_MANIFEST_DIR="$BASE_DIR" timeout "${DURATION}s" "$PLAYER_BIN" $PLAYER_ARGS 2>&1 | tail -10 || true
fi

# Wait for tracy-capture to finish saving
echo "[3.5/4] Waiting for tracy-capture to finish..."
sleep 2
# Send SIGINT to tracy-capture to gracefully stop
kill -INT "$CAPTURE_PID" 2>/dev/null || true
wait "$CAPTURE_PID" 2>/dev/null || true

if [ -f "$OUTPUT" ]; then
    FILE_SIZE=$(du -h "$OUTPUT" | cut -f1)
    echo "[4/4] Capture saved: $OUTPUT ($FILE_SIZE)"
else
    echo "[4/4] ERROR: No capture file generated!"
    exit 1
fi

# Optional analysis
if [ "$ANALYZE" = true ]; then
    echo ""
    echo "========================================"
    echo "Tracy Analysis: $PROJECT"
    echo "========================================"
    tracy-csvexport "$OUTPUT" 2>/dev/null | python3 -c "
import sys, csv
from collections import defaultdict

reader = csv.DictReader(sys.stdin)
zones = defaultdict(lambda: {'total_ns': 0, 'count': 0, 'max_ns': 0})

for row in reader:
    name = row.get('name', 'unknown')
    exec_ns = int(row.get('exec_time_ns', 0))
    zones[name]['total_ns'] += exec_ns
    zones[name]['count'] += 1
    zones[name]['max_ns'] = max(zones[name]['max_ns'], exec_ns)

# Sort by total time descending
sorted_zones = sorted(zones.items(), key=lambda x: x[1]['total_ns'], reverse=True)

print(f'Total zones: {sum(z[\"count\"] for z in zones.values())}')
print(f'Unique zone names: {len(zones)}')
print()
print(f'{\"Zone Name\":<60} {\"Total ms\":>10} {\"Count\":>8} {\"Avg ms\":>10} {\"Max ms\":>10}')
print('-' * 100)
for name, data in sorted_zones[:30]:
    total_ms = data['total_ns'] / 1_000_000
    avg_ms = total_ms / data['count'] if data['count'] > 0 else 0
    max_ms = data['max_ns'] / 1_000_000
    print(f'{name:<60} {total_ms:>10.2f} {data[\"count\"]:>8} {avg_ms:>10.3f} {max_ms:>10.2f}')
" || echo "Analysis failed (tracy-csvexport may not support this trace format)"
fi

echo ""
echo "Done! To view the trace in Tracy GUI:"
echo "  tracy $OUTPUT"
