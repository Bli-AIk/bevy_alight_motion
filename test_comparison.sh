#!/bin/bash

# Automated Video Comparison Testing Script
# Runs visual regression tests for projects in assets/projects/
# Checks against thresholds defined in comparison_config.toml
#
# Features:
# - Parallel frame extraction (CPU-bound ffmpeg work)
# - Sequential rendering tests (GPU doesn't parallelize well)
# - Configurable max parallelism via PARALLEL_JOBS environment variable
# - Batch result reporting with detailed summary
# - Category filtering (default: basic/ only)
#
# Directory structure:
#   assets/projects/
#   ├── basic/           # Basic functionality tests
#   ├── effects/         # Effect-specific tests
#   ├── complex/         # Complex examples
#   └── showcase/        # Showcase (excluded from tests)
#
# Usage:
#   ./test_comparison.sh              # Run only basic/* tests (default)
#   ./test_comparison.sh --all        # Run all tests
#   ./test_comparison.sh basic/       # Run only basic/* tests
#   ./test_comparison.sh effects/     # Run only effects/* tests
#   ./test_comparison.sh complex/     # Run only complex/* tests
#   ./test_comparison.sh effects/stretch  # Run tests matching effects/stretch*
#   ./test_comparison.sh private/.../target2 --single  # Run only the exact example
#   ./test_comparison.sh --frame-test              # Run FPS benchmark on basic/* tests
#   ./test_comparison.sh --frame-test --all        # Run FPS benchmark on all tests
#   ./test_comparison.sh --frame-test effects/     # Run FPS benchmark on effects/* tests
#   AM_PLAYER_EXTRA_FEATURES=player-brp ./test_comparison.sh private/.../target2 --single --headless
#   PARALLEL_JOBS=8 ./test_comparison.sh  # More parallel frame extraction jobs
#
# Note: Rendering tests run sequentially to avoid GPU resource contention.
# Frame extraction is parallelized for speed.

# Configuration
PARALLEL_JOBS=${PARALLEL_JOBS:-4}  # Default 4 parallel jobs (for frame extraction)
PLAYER_EXTRA_FEATURES=${AM_PLAYER_EXTRA_FEATURES:-}
PLAYER_BIN_OVERRIDE=${AM_PLAYER_BIN:-}
SKIP_BUILD_REQUESTED=${AM_SKIP_BUILD:-0}

is_truthy() {
    case "${1:-}" in
        1|true|TRUE|yes|YES|on|ON)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

# Parse command line arguments
FILTER_PATTERN=""
RUN_ALL=false
FRAME_TEST=false
HEADLESS=false
EXACT_MATCH=false
HAS_EXPLICIT_FILTER=false

# Parse flags that can appear anywhere
for arg in "$@"; do
    if [ "$arg" == "--headless" ]; then
        HEADLESS=true
    elif [ "$arg" == "--single" ]; then
        EXACT_MATCH=true
    elif [ "$arg" == "--frame-test" ]; then
        FRAME_TEST=true
    fi
done
# Remove flags that can appear anywhere from positional args
ARGS=()
for arg in "$@"; do
    if [ "$arg" != "--headless" ] && [ "$arg" != "--single" ] && [ "$arg" != "--frame-test" ]; then
        ARGS+=("$arg")
    fi
done
set -- "${ARGS[@]}"

# Auto-detect headless: if no DISPLAY or WAYLAND_DISPLAY, enable headless
if [ "$HEADLESS" = false ] && [ -z "$DISPLAY" ] && [ -z "$WAYLAND_DISPLAY" ]; then
    echo "No display detected, enabling headless mode automatically."
    HEADLESS=true
fi

# Headless mode uses headless-render feature (no xvfb needed)
# Fallback to xvfb-run if headless-render build fails
HEADLESS_RENDER=false
if [ "$HEADLESS" = true ]; then
    HEADLESS_RENDER=true
fi

if [ "$1" == "--all" ]; then
    RUN_ALL=true
    shift
elif [ -n "$1" ]; then
    FILTER_PATTERN="$1"
    # Strip common path prefixes so users can pass paths like
    # "projects/private/..." or "assets/projects/private/..." and still match.
    FILTER_PATTERN="${FILTER_PATTERN#./}"
    FILTER_PATTERN="${FILTER_PATTERN#assets/projects/}"
    FILTER_PATTERN="${FILTER_PATTERN#projects/}"
    HAS_EXPLICIT_FILTER=true
else
    # Default: only run basic/* tests
    FILTER_PATTERN="basic/"
fi

if [ "$EXACT_MATCH" = true ] && [ "$RUN_ALL" = true ]; then
    echo "Error: --single cannot be combined with --all"
    exit 1
fi

if [ "$EXACT_MATCH" = true ] && [ "$HAS_EXPLICIT_FILTER" = false ]; then
    echo "Error: --single requires an explicit example path"
    exit 1
fi

# Ensure we are in the correct directory
if [ -d "assets/projects" ]; then
    BASE_DIR="."
    PROJECTS_DIR="assets/projects"
elif [ -d "crates/bevy_alight_motion/assets/projects" ]; then
    BASE_DIR="crates/bevy_alight_motion"
    PROJECTS_DIR="crates/bevy_alight_motion/assets/projects"
else
    echo "Error: Cannot find assets/projects directory for bevy_alight_motion"
    exit 1
fi

COMPARISON_CONFIG_PATH="${BASE_DIR}/comparison_config.toml"
FRAME_CACHE_DIR="${PROJECTS_DIR}/_video_frames"
DEBUG_FRAME_CACHE_DIR="${BASE_DIR}/assets/debug/_video_frames"

sort_examples() {
    printf '%s\n' "$1" | tr ' ' '\n' | sed '/^$/d' | sort -u | tr '\n' ' '
}

partition_examples_by_skip() {
    local examples="$1"
    local config_path="$2"

    if [ -z "$examples" ] || [ ! -f "$config_path" ]; then
        printf '%s\n' "$examples" | tr ' ' '\n' | sed '/^$/d' | while IFS= read -r test_id; do
            [ -n "$test_id" ] && printf 'RUN\t%s\n' "$test_id"
        done
        return 0
    fi

    EXAMPLES_INPUT="$examples" python3 - "$config_path" <<'PY'
import os
import sys

try:
    import tomllib
except ImportError:  # pragma: no cover
    import tomli as tomllib

config_path = sys.argv[1]
examples = [line.strip() for line in os.environ.get("EXAMPLES_INPUT", "").split() if line.strip()]

with open(config_path, "rb") as f:
    config = tomllib.load(f)

default = config.get("default", {})
default_skip = bool(default.get("skip", False))
overrides = config.get("overrides", {})

for test_id in examples:
    skip = default_skip
    override = overrides.get(test_id)
    if isinstance(override, dict) and override.get("skip") is not None:
        skip = bool(override["skip"])
    kind = "SKIP" if skip else "RUN"
    print(f"{kind}\t{test_id}")
PY
}

discover_skip_only_examples() {
    local config_path="$1"
    local projects_dir="$2"
    local filter_pattern="$3"
    local exact_match="$4"
    local run_all="$5"

    if [ ! -f "$config_path" ]; then
        return 0
    fi

    python3 - "$config_path" "$projects_dir" "$filter_pattern" "$exact_match" "$run_all" <<'PY'
import sys
from pathlib import Path

try:
    import tomllib
except ImportError:  # pragma: no cover
    import tomli as tomllib

config_path, projects_dir, filter_pattern, exact_match, run_all = sys.argv[1:]
projects_dir = Path(projects_dir)

with open(config_path, "rb") as f:
    config = tomllib.load(f)

overrides = config.get("overrides", {})

def matches_filter(test_id: str) -> bool:
    if run_all == "true":
        return True
    if filter_pattern:
        if exact_match == "true":
            return test_id == filter_pattern
        return test_id.startswith(filter_pattern)
    return test_id.startswith("basic/")

for test_id, override in overrides.items():
    if not isinstance(override, dict) or not override.get("skip"):
        continue
    if not matches_filter(test_id):
        continue
    if (projects_dir / f"{test_id}.amproj").exists():
        print(test_id)
PY
}

cleanup_stale_frame_cache() {
    local cache_root="$1"
    local interval_hours="${FRAME_CACHE_CLEANUP_INTERVAL_HOURS:-24}"
    local max_age_days="${FRAME_CACHE_MAX_AGE_DAYS:-14}"

    if [ "${FRAME_CACHE_DISABLE_CLEANUP:-0}" = "1" ]; then
        return 0
    fi

    mkdir -p "$cache_root"
    local marker_file="${cache_root}/.last_cleanup"
    local now
    now=$(date +%s)
    local interval_secs=$((interval_hours * 3600))
    local max_age_secs=$((max_age_days * 86400))

    if [ -f "$marker_file" ]; then
        local last_cleanup
        last_cleanup=$(stat -c %Y "$marker_file" 2>/dev/null || echo 0)
        if [ $((now - last_cleanup)) -lt "$interval_secs" ]; then
            return 0
        fi
    fi

    echo "Checking stale frame cache under $cache_root ..."
    local pruned=0

    while IFS= read -r cache_dir; do
        [ -z "$cache_dir" ] && continue
        local extracted_marker="${cache_dir}/.extracted"
        local last_used_marker="${cache_dir}/.last_used"

        if [ ! -f "$extracted_marker" ] || [ ! -f "$last_used_marker" ]; then
            rm -rf "$cache_dir"
            pruned=$((pruned + 1))
            continue
        fi

        local last_used
        last_used=$(stat -c %Y "$last_used_marker" 2>/dev/null || echo 0)
        if [ $((now - last_used)) -ge "$max_age_secs" ]; then
            rm -rf "$cache_dir"
            pruned=$((pruned + 1))
        fi
    done < <(find "$cache_root" -mindepth 1 -maxdepth 1 -type d | sort)

    touch "$marker_file"
    echo "  [CACHE-CLEANUP] pruned ${pruned} stale cache director$( [ "$pruned" -eq 1 ] && echo 'y' || echo 'ies' )"
}

remove_example_caches() {
    local test_id="$1"
    local flat_name
    flat_name=$(echo "$test_id" | tr '/' '_')
    local dir_name
    dir_name=$(dirname "$test_id")
    local base_name
    base_name=$(basename "$test_id")

    rm -rf "${FRAME_CACHE_DIR}/${flat_name}"
    rm -rf "${PROJECTS_DIR}/${dir_name}/_video_frames/${base_name}"
    rm -rf "${DEBUG_FRAME_CACHE_DIR}/${base_name}"
}

if [ "$FRAME_TEST" = true ]; then
    echo "========================================"
    echo "Frame Test (FPS Benchmark) Suite"
    echo "========================================"
else
    echo "========================================"
    echo "Video Comparison Test Suite"
    echo "========================================"
    echo "Frame extraction parallelism: $PARALLEL_JOBS"
fi
if [ "$RUN_ALL" = true ]; then
    echo "Mode: Running ALL tests (basic_*, fx_*, complex_*)"
elif [ "$EXACT_MATCH" = true ]; then
    echo "Mode: Running exact test '$FILTER_PATTERN'"
elif [ -n "$FILTER_PATTERN" ]; then
    echo "Mode: Running tests matching '$FILTER_PATTERN*'"
fi
if [ "$HEADLESS" = true ]; then
    if [ "$HEADLESS_RENDER" = true ]; then
        echo "Headless: enabled (headless-render feature, no display server needed)"
    else
        echo "Headless: enabled (software rendering via llvmpipe + Xvfb)"
    fi
fi
echo ""

# Set llvmpipe optimization environment variables for software rendering
if [ "$HEADLESS" = true ]; then
    export LP_NUM_THREADS=${LP_NUM_THREADS:-4}
    export GALLIVM_PERF_LEVEL=${GALLIVM_PERF_LEVEL:-3}
    export MESA_GLTHREAD=${MESA_GLTHREAD:-true}
    export RAYON_NUM_THREADS=${RAYON_NUM_THREADS:-4}
    export MESA_NO_ERROR=${MESA_NO_ERROR:-1}
    export WGPU_BACKEND=${WGPU_BACKEND:-vulkan}
fi

# Build player example first
HEADLESS_FEATURES=""
if [ "$HEADLESS_RENDER" = true ]; then
    HEADLESS_FEATURES=",headless-render"
fi

BUILD_FEATURES=""
if [ "$FRAME_TEST" = true ]; then
    BUILD_FEATURES="frame-test${HEADLESS_FEATURES}"
else
    BUILD_FEATURES="video-comparison${HEADLESS_FEATURES}"
fi

if [ -n "$PLAYER_EXTRA_FEATURES" ]; then
    BUILD_FEATURES="${BUILD_FEATURES},${PLAYER_EXTRA_FEATURES}"
    echo "Extra player features: $PLAYER_EXTRA_FEATURES"
fi

if is_truthy "$SKIP_BUILD_REQUESTED"; then
    if [ -z "$PLAYER_BIN_OVERRIDE" ]; then
        echo "Error: AM_SKIP_BUILD is set but AM_PLAYER_BIN is empty"
        exit 1
    fi
    echo "Skipping build because AM_SKIP_BUILD is enabled."
    PLAYER_BIN="$PLAYER_BIN_OVERRIDE"
else
    echo "Building player example (${BUILD_FEATURES})..."
    cargo build -p bevy_alight_motion --example player --features "$BUILD_FEATURES" --release
    if [ $? -ne 0 ]; then
        echo "Build failed!"
        exit 1
    fi

    # Get binary path
    PLAYER_BIN=$(cargo metadata --format-version=1 2>/dev/null | \
        python3 -c "import sys,json; print(json.load(sys.stdin)['target_directory'])")/release/examples/player

    if [ ! -f "$PLAYER_BIN" ]; then
        # Fallback path detection
        PLAYER_BIN="target/release/examples/player"
    fi

    if [ -n "$PLAYER_BIN_OVERRIDE" ]; then
        PLAYER_BIN="$PLAYER_BIN_OVERRIDE"
    fi
fi

if [ ! -f "$PLAYER_BIN" ]; then
    echo "Player binary not found: $PLAYER_BIN"
    exit 1
fi

echo "Using binary: $PLAYER_BIN"

# GPU cooldown after build to prevent thermal throttling on first tests
if [ "$FRAME_TEST" = "true" ]; then
    echo "Waiting 5s for GPU cooldown after build..."
    sleep 5
fi

# Get all matching amproj files
# In frame-test mode: all amproj files (no video needed)
# In comparison mode: only amproj files with corresponding .mp4 videos
EXAMPLES=""
while IFS= read -r amproj; do
    # Get the path relative to PROJECTS_DIR (e.g., basic/shape/shape.amproj)
    rel_path="${amproj#$PROJECTS_DIR/}"
    # Remove .amproj extension to get the test ID (e.g., basic/shape/shape)
    test_id="${rel_path%.amproj}"
    # Get directory and basename
    dir_path=$(dirname "$amproj")
    base_name=$(basename "$amproj" .amproj)
    video_path="${dir_path}/${base_name}.mp4"
    
    # Skip showcase and _video_frames
    if echo "$test_id" | grep -qE '^(showcase/|_video_frames)'; then
        continue
    fi

    # Check if this test should be included
    has_video=false
    [ -f "$video_path" ] && has_video=true

    # In comparison mode, require video; in frame-test mode, any amproj works
    if [ "$FRAME_TEST" = true ]; then
        should_include=true
    else
        should_include=$has_video
    fi

    if [ "$should_include" = true ]; then
        # Apply category filter
        if [ "$RUN_ALL" = true ]; then
            EXAMPLES="$EXAMPLES $test_id"
        elif [ -n "$FILTER_PATTERN" ]; then
            if [ "$EXACT_MATCH" = true ]; then
                if [ "$test_id" = "$FILTER_PATTERN" ]; then
                    EXAMPLES="$EXAMPLES $test_id"
                fi
            elif echo "$test_id" | grep -q "^${FILTER_PATTERN}"; then
                EXAMPLES="$EXAMPLES $test_id"
            fi
        else
            if echo "$test_id" | grep -q "^basic/"; then
                EXAMPLES="$EXAMPLES $test_id"
            fi
        fi
    fi
done < <(find "$PROJECTS_DIR" \( -name "*.amproj" -type d -print -prune \) -o \( -name "*.amproj" -type f -print \) | sort)

if [ "$FRAME_TEST" != true ]; then
    EXTRA_SKIP_EXAMPLES=$(discover_skip_only_examples \
        "$COMPARISON_CONFIG_PATH" \
        "$PROJECTS_DIR" \
        "$FILTER_PATTERN" \
        "$EXACT_MATCH" \
        "$RUN_ALL")
    EXAMPLES=$(sort_examples "$EXAMPLES $EXTRA_SKIP_EXAMPLES")
else
    EXAMPLES=$(sort_examples "$EXAMPLES")
fi

RUN_EXAMPLES=""
SKIP_EXAMPLES=""
while IFS=$'\t' read -r kind test_id; do
    [ -z "$test_id" ] && continue
    if [ "$kind" = "SKIP" ]; then
        SKIP_EXAMPLES="$SKIP_EXAMPLES $test_id"
    else
        RUN_EXAMPLES="$RUN_EXAMPLES $test_id"
    fi
done < <(partition_examples_by_skip "$EXAMPLES" "$COMPARISON_CONFIG_PATH")

RUN_EXAMPLES=$(sort_examples "$RUN_EXAMPLES")
SKIP_EXAMPLES=$(sort_examples "$SKIP_EXAMPLES")

TOTAL_EXAMPLE_COUNT=$(echo "$EXAMPLES" | wc -w)
RUN_EXAMPLE_COUNT=$(echo "$RUN_EXAMPLES" | wc -w)
SKIP_EXAMPLE_COUNT=$(echo "$SKIP_EXAMPLES" | wc -w)
echo ""
echo "Found $TOTAL_EXAMPLE_COUNT matching examples"
echo "Runnable: $RUN_EXAMPLE_COUNT"
echo "Skipped by config: $SKIP_EXAMPLE_COUNT"
echo ""

if [ "$TOTAL_EXAMPLE_COUNT" -eq 0 ]; then
    echo "No examples to test!"
    exit 0
fi

# Create temporary directory for results
RESULTS_DIR=$(mktemp -d)
trap "rm -rf $RESULTS_DIR" EXIT

# Drop any stale caches for config-skipped examples immediately.
for example in $SKIP_EXAMPLES; do
    remove_example_caches "$example"
done

# Phase 1: Pre-extract video frames in parallel (CPU-bound)
# Skip this phase in frame-test mode (no video comparison needed)
if [ "$FRAME_TEST" != true ]; then
echo "========================================"
echo "Phase 1: Extracting video frames (parallel)"
echo "========================================"

mkdir -p "$FRAME_CACHE_DIR"
cleanup_stale_frame_cache "$FRAME_CACHE_DIR"

extract_frames_for_video() {
    local test_id=$1
    # test_id is like basic/shape/shape, video is at projects/basic/shape/shape.mp4
    local video_path="${PROJECTS_DIR}/${test_id}.mp4"
    # Frame cache uses flattened name (replace / with _)
    local cache_name=$(echo "$test_id" | tr '/' '_')
    local frame_dir="${FRAME_CACHE_DIR}/${cache_name}"
    local marker_file="${frame_dir}/.extracted"
    
    # Skip if already extracted (cache hit)
    if [ -f "$marker_file" ]; then
        touch "${frame_dir}/.last_used"
        echo "  [CACHE] $test_id"
        return 0
    fi
    
    # Get FPS from video
    local fps=$(ffprobe -v error -select_streams v:0 -show_entries stream=r_frame_rate -of default=noprint_wrappers=1:nokey=1 "$video_path" 2>/dev/null)
    if [[ "$fps" == *"/"* ]]; then
        fps=$(echo "scale=2; $fps" | bc)
    fi
    fps=${fps:-12}
    
    # Clean and create frame directory
    rm -rf "$frame_dir"
    mkdir -p "$frame_dir"
    
    # Extract frames
    ffmpeg -i "$video_path" -vf "fps=$fps" -y "${frame_dir}/frame_%06d.png" 2>/dev/null
    
    # Write marker file
    echo "$fps" > "$marker_file"
    touch "${frame_dir}/.last_used"
    
    local frame_count=$(ls "$frame_dir"/*.png 2>/dev/null | wc -l)
    echo "  [DONE] $test_id ($frame_count frames)"
}

export -f extract_frames_for_video
export PROJECTS_DIR FRAME_CACHE_DIR

# Run frame extraction in parallel
echo "Extracting frames with $PARALLEL_JOBS parallel jobs..."
if [ "$RUN_EXAMPLE_COUNT" -gt 0 ]; then
    echo "$RUN_EXAMPLES" | tr ' ' '\n' | xargs -P "$PARALLEL_JOBS" -I {} bash -c 'extract_frames_for_video "$@"' _ {}
else
    echo "No runnable examples require frame extraction."
fi

echo ""
echo "Frame extraction complete!"
echo ""
fi  # end of frame-test skip

# Phase 2: Run rendering tests (sequential to avoid GPU contention)
if [ "$FRAME_TEST" = true ]; then
    echo "========================================"
    echo "Running frame tests (sequential)"
    echo "========================================"
else
    echo "========================================"
    echo "Phase 2: Running comparison tests (sequential)"
    echo "========================================"
fi
echo "Note: Tests run sequentially to avoid GPU resource contention"
echo ""

# Export variables for subprocesses
export PLAYER_BIN RESULTS_DIR
# Pass through MAX_FRAMES if set (for limiting test frames during debugging)
[ -n "$MAX_FRAMES" ] && export MAX_FRAMES
# CARGO_MANIFEST_DIR is required for Bevy asset loading
MANIFEST_DIR="$(cd "$BASE_DIR" && pwd)"
export MANIFEST_DIR

# Function to run a single test
run_single_test() {
    local test_id=$1
    # Create result file with flattened name (replace / with _)
    local flat_name=$(echo "$test_id" | tr '/' '_')
    local result_file="$RESULTS_DIR/${flat_name}.result"
    local log_file="$RESULTS_DIR/${flat_name}.log"
    
    # In headless mode, wrap with xvfb-run and pass --headless to player
    # With headless-render feature, no xvfb needed at all
    local headless_flag=""
    local xvfb_prefix=""
    if [ "$HEADLESS" = true ]; then
        headless_flag="--headless"
        if [ "$HEADLESS_RENDER" != true ]; then
            xvfb_prefix="xvfb-run -a"
        fi
    fi
    CARGO_MANIFEST_DIR="$MANIFEST_DIR" \
        $xvfb_prefix "$PLAYER_BIN" "$test_id" $headless_flag > "$log_file" 2>&1
    
    local exit_code=$?
    local report_path=""
    if grep -q "Report saved to:" "$log_file"; then
        report_path=$(grep "Report saved to:" "$log_file" | tail -1 | sed -E 's/.*Report saved to: "?([^"]+)"?/\1/')
    fi
    
    # Determine result from log
    if grep -q "RESULT: PASS" "$log_file"; then
        echo "PASS|$test_id|||$report_path" > "$result_file"
        echo "✅ $test_id"
    elif grep -q "RESULT: WARNING" "$log_file"; then
        local warning_msg=$(grep "RESULT: WARNING" "$log_file" | head -1)
        echo "WARNING|$test_id|$warning_msg||$report_path" > "$result_file"
        echo "⚠️  $test_id (WARNING)"
    elif grep -q "RESULT: SKIP" "$log_file"; then
        echo "SKIP|$test_id|||$report_path" > "$result_file"
        echo "⚠️  $test_id (SKIP)"
    elif grep -q "RESULT: CANCELLED" "$log_file"; then
        echo "CANCELLED|$test_id|||$report_path" > "$result_file"
        echo "⛔ $test_id (CANCELLED by user)"
    else
        # Extract failure details (both Average Similarity and Per-Frame Pass Rate)
        avg_sim=$(grep "Average Similarity" "$log_file" | head -1)
        frame_rate=$(grep "Per-Frame Pass Rate" "$log_file" | head -1)
        echo "FAIL|$test_id|$avg_sim|$frame_rate|$report_path" > "$result_file"
        echo "❌ $test_id (FAIL)"
        if [ -n "${AM_ECHO_FAIL_LOG_TAIL:-}" ]; then
            local tail_lines="${AM_ECHO_FAIL_LOG_LINES:-120}"
            echo "   ↳ fail-log-tail (${tail_lines} lines)"
            tail -n "$tail_lines" "$log_file" | sed 's/^/      | /'
        fi
    fi

    # Extract frame-test JSON data (for perf_results.json)
    if [ "$FRAME_TEST" = "true" ]; then
        local perf_json_line
        perf_json_line=$(grep '^\[FRAME-TEST-JSON\]' "$log_file" | tail -1 | sed 's/^\[FRAME-TEST-JSON\] //')
        if [ -n "$perf_json_line" ]; then
            echo "$perf_json_line" > "$RESULTS_DIR/${flat_name}.perf_json"
        fi
    fi

    if [ -n "$report_path" ]; then
        echo "   ↳ report: $report_path"
    fi
}

# Function to incrementally write a single test result to JSON
write_result_to_json() {
    local result_file=$1
    local result_key
    if [ "$FRAME_TEST" = "true" ]; then
        result_key="frame_test_results"
    else
        result_key="results"
    fi

    if [ ! -f "$result_file" ]; then
        return
    fi

    IFS='|' read -r status name avg_details frame_details report_path < "$result_file"
    local avg_val=""
    if [ -n "$avg_details" ]; then
        avg_val=$(echo "$avg_details" | grep -oP '\d+\.\d+' | head -1)
    fi
    local status_lower=$(echo "$status" | tr '[:upper:]' '[:lower:]')

    # For frame-test mode, check for rich JSON data and write to perf_results.json
    if [ "$FRAME_TEST" = "true" ]; then
        local flat_name=$(echo "$name" | tr '/' '_')
        local perf_json_file="$RESULTS_DIR/${flat_name}.perf_json"
        local json_output="${BASE_DIR}/perf_results.json"

        python3 << PYEOF
import json, os
from datetime import datetime

json_output = "$json_output"
name = "$name"
status = "$status_lower"
perf_json_file = "$perf_json_file"

existing_data = {}
if os.path.exists(json_output):
    try:
        with open(json_output, 'r') as f:
            existing_data = json.load(f)
    except (json.JSONDecodeError, IOError):
        pass

results = existing_data.get("results", {})

# Try to read rich perf data from [FRAME-TEST-JSON] output
entry = {"status": status}
if os.path.exists(perf_json_file):
    try:
        with open(perf_json_file, 'r') as f:
            perf_data = json.load(f)
        entry.update(perf_data)
        entry["status"] = status  # ensure status from result file takes precedence
    except (json.JSONDecodeError, IOError):
        pass

results[name] = entry
existing_data["results"] = dict(sorted(results.items()))

# Recalculate summary
passed = sum(1 for r in results.values() if r.get('status') == 'pass')
failed = sum(1 for r in results.values() if r.get('status') == 'fail')
warnings = sum(1 for r in results.values() if r.get('status') == 'warning')
skipped = sum(1 for r in results.values() if r.get('status') in ('skip', 'cancelled'))
existing_data["summary"] = {"passed": passed, "warnings": warnings, "skipped": skipped, "failed": failed}
existing_data["timestamp"] = datetime.now().astimezone().isoformat()

with open(json_output, 'w') as f:
    json.dump(existing_data, f, indent=2)
PYEOF
    else
        # Original behavior for comparison tests — write to test_results.json
        local json_output="${BASE_DIR}/test_results.json"

        python3 << PYEOF
import json, os
from datetime import datetime

json_output = "$json_output"
result_key = "$result_key"
name = "$name"
status = "$status_lower"
avg_val = "$avg_val"

existing_data = {}
if os.path.exists(json_output):
    try:
        with open(json_output, 'r') as f:
            existing_data = json.load(f)
    except (json.JSONDecodeError, IOError):
        pass

results = existing_data.get(result_key, {})
entry = {"status": status}
if avg_val:
    entry["avg_similarity"] = float(avg_val)
results[name] = entry

existing_data[result_key] = dict(sorted(results.items()))

# Recalculate summary
summary_key = "summary"
passed = sum(1 for r in results.values() if r.get('status') == 'pass')
failed = sum(1 for r in results.values() if r.get('status') == 'fail')
warnings = sum(1 for r in results.values() if r.get('status') == 'warning')
skipped = sum(1 for r in results.values() if r.get('status') in ('skip', 'cancelled'))
existing_data[summary_key] = {"passed": passed, "warnings": warnings, "skipped": skipped, "failed": failed}
existing_data["timestamp"] = datetime.now().astimezone().isoformat()

with open(json_output, 'w') as f:
    json.dump(existing_data, f, indent=2)
PYEOF
    fi
}

# Run tests sequentially (GPU doesn't handle parallel rendering well)
for example in $SKIP_EXAMPLES; do
    flat_name=$(echo "$example" | tr '/' '_')
    result_file="$RESULTS_DIR/${flat_name}.result"
    echo "SKIP|$example|||" > "$result_file"
    write_result_to_json "$result_file"
    echo "⚠️  $example (SKIP by config)"
done

for example in $RUN_EXAMPLES; do
    run_single_test "$example"
    # Write result to JSON immediately after each test
    local_flat=$(echo "$example" | tr '/' '_')
    write_result_to_json "$RESULTS_DIR/${local_flat}.result"
    # Cooldown between tests to mitigate GPU thermal throttling (especially on iGPUs)
    if [ "$FRAME_TEST" = "true" ]; then
        sleep 3
    fi
done

echo ""
echo "========================================"
echo "DETAILED RESULTS"
echo "========================================"
printf "%-40s | %s\n" "Example Name" "Status"
echo "-----------------------------------------+--------"

PASSED_COUNT=0
FAILED_COUNT=0
SKIPPED_COUNT=0
WARNING_COUNT=0
FAILED_EXAMPLES=""

for result_file in "$RESULTS_DIR"/*.result; do
    if [ -f "$result_file" ]; then
        IFS='|' read -r status name avg_details frame_details report_path < "$result_file"
        if [ "$status" == "PASS" ]; then
            printf "%-40s | \033[0;32m✅ PASS\033[0m\n" "$name"
            if [ -n "$report_path" ]; then
                printf "%-40s   %s\n" "" "Report: $report_path"
            fi
            PASSED_COUNT=$((PASSED_COUNT + 1))
        elif [ "$status" == "WARNING" ]; then
            printf "%-40s | \033[1;33m⚠️ WARNING\033[0m\n" "$name"
            if [ -n "$avg_details" ]; then
                printf "%-40s   %s\n" "" "$avg_details"
            fi
            if [ -n "$report_path" ]; then
                printf "%-40s   %s\n" "" "Report: $report_path"
            fi
            WARNING_COUNT=$((WARNING_COUNT + 1))
        elif [ "$status" == "SKIP" ]; then
            printf "%-40s | \033[0;33m⚠️ SKIP\033[0m\n" "$name"
            if [ -n "$report_path" ]; then
                printf "%-40s   %s\n" "" "Report: $report_path"
            fi
            SKIPPED_COUNT=$((SKIPPED_COUNT + 1))
        elif [ "$status" == "CANCELLED" ]; then
            printf "%-40s | \033[0;33m⛔ CANCELLED\033[0m\n" "$name"
            if [ -n "$report_path" ]; then
                printf "%-40s   %s\n" "" "Report: $report_path"
            fi
            SKIPPED_COUNT=$((SKIPPED_COUNT + 1))
        else
            printf "%-40s | \033[0;31m❌ FAIL\033[0m\n" "$name"
            if [ -n "$avg_details" ]; then
                printf "%-40s   %s\n" "" "$avg_details"
            fi
            if [ -n "$frame_details" ]; then
                printf "%-40s   %s\n" "" "$frame_details"
            fi
            if [ -n "$report_path" ]; then
                printf "%-40s   %s\n" "" "Report: $report_path"
            fi
            FAILED_COUNT=$((FAILED_COUNT + 1))
            FAILED_EXAMPLES="${FAILED_EXAMPLES}\n  - ${name}"
        fi
    fi
done

echo ""
echo "========================================"
echo "TEST SUMMARY"
echo "========================================"
echo "Passed:   $PASSED_COUNT"
echo "Warnings: $WARNING_COUNT"
echo "Skipped:  $SKIPPED_COUNT"
echo "Failed:   $FAILED_COUNT"

# Generate JSON results file (merge with existing results)
if [ "$FRAME_TEST" = "true" ]; then
    JSON_OUTPUT="${BASE_DIR}/perf_results.json"
else
    JSON_OUTPUT="${BASE_DIR}/test_results.json"
fi

# Build new results as JSON entries
NEW_RESULTS=""
for result_file in "$RESULTS_DIR"/*.result; do
    if [ -f "$result_file" ]; then
        IFS='|' read -r status name avg_details frame_details report_path < "$result_file"
        
        # Extract similarity value or FPS value from details if present
        avg_val=""
        if [ -n "$avg_details" ]; then
            avg_val=$(echo "$avg_details" | grep -oP '\d+\.\d+' | head -1)
        fi
        
        # Convert status to lowercase
        status_lower=$(echo "$status" | tr '[:upper:]' '[:lower:]')
        
        # Build JSON entry with test-type-specific fields
        entry="\"$name\": { \"status\": \"$status_lower\""
        if [ -n "$avg_val" ]; then
            if [ "$FRAME_TEST" = "true" ]; then
                entry="$entry, \"avg_fps\": $avg_val"
            else
                entry="$entry, \"avg_similarity\": $avg_val"
            fi
        fi
        entry="$entry }"
        
        if [ -n "$NEW_RESULTS" ]; then
            NEW_RESULTS="$NEW_RESULTS, $entry"
        else
            NEW_RESULTS="$entry"
        fi
    fi
done

# Determine result key based on test type
RESULT_KEY="results"

# Merge with existing results using Python (preserves old results, updates with new)
python3 << EOF
import json
import os
from datetime import datetime

json_output = "$JSON_OUTPUT"
result_key = "$RESULT_KEY"
frame_test = "$FRAME_TEST" == "true"
results_dir = "$RESULTS_DIR"
new_results_json = '''{$NEW_RESULTS}'''

# Parse new results
try:
    new_results = json.loads(new_results_json) if new_results_json.strip() and new_results_json.strip() != '{}' else {}
except json.JSONDecodeError:
    new_results = {}

# For frame-test mode, enrich entries with [FRAME-TEST-JSON] data
if frame_test:
    for name in list(new_results.keys()):
        flat_name = name.replace('/', '_')
        perf_json_path = os.path.join(results_dir, f"{flat_name}.perf_json")
        if os.path.exists(perf_json_path):
            try:
                with open(perf_json_path, 'r') as f:
                    perf_data = json.load(f)
                status = new_results[name].get('status', 'fail')
                new_results[name] = perf_data
                new_results[name]['status'] = status
            except (json.JSONDecodeError, IOError):
                pass

# Load existing data if file exists
existing_data = {}
if os.path.exists(json_output):
    try:
        with open(json_output, 'r') as f:
            existing_data = json.load(f)
    except (json.JSONDecodeError, IOError):
        pass  # Start fresh if file is corrupted

# Get existing results for this result type
existing_results = existing_data.get(result_key, {})

# Merge: existing results + new results (new overwrites old for same keys)
merged_results = {**existing_results, **new_results}

# Recalculate summary for this result type
passed = sum(1 for r in merged_results.values() if r.get('status') == 'pass')
failed = sum(1 for r in merged_results.values() if r.get('status') == 'fail')
warnings = sum(1 for r in merged_results.values() if r.get('status') == 'warning')
skipped = sum(1 for r in merged_results.values() if r.get('status') in ('skip', 'cancelled'))

# Update existing data with new results (preserving other test types)
existing_data[result_key] = dict(sorted(merged_results.items()))
existing_data["summary"] = {
    "passed": passed,
    "warnings": warnings,
    "skipped": skipped,
    "failed": failed
}
existing_data["timestamp"] = datetime.now().astimezone().isoformat()

# Write output
with open(json_output, 'w') as f:
    json.dump(existing_data, f, indent=2)

print(f"Merged {len(new_results)} new results with {len(existing_results)} existing results")
print(f"Total: {len(merged_results)} results ({passed} passed, {failed} failed, {skipped} skipped)")
EOF

echo ""
echo "📄 JSON results saved to: $JSON_OUTPUT"

if [ $FAILED_COUNT -gt 0 ]; then
    echo -e "Failed Examples:$FAILED_EXAMPLES"
    exit 1
else
    echo "🎉 All tests passed!"
    exit 0
fi
