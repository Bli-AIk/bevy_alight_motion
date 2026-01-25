#!/bin/bash

# Automated Video Comparison Testing Script
# Runs visual regression tests for all basic_*, fx_*, and complex_* examples
# Checks against thresholds defined in comparison_config.toml
#
# Features:
# - Parallel frame extraction (CPU-bound ffmpeg work)
# - Sequential rendering tests (GPU doesn't parallelize well)
# - Configurable max parallelism via PARALLEL_JOBS environment variable
# - Batch result reporting with detailed summary
# - Prefix filtering (default: only basic_* examples)
#
# Usage:
#   ./test_comparison.sh              # Run only basic_* tests (default)
#   ./test_comparison.sh --all        # Run all tests (basic_*, fx_*, complex_*)
#   ./test_comparison.sh basic_       # Run only basic_* tests
#   ./test_comparison.sh fx_          # Run only fx_* tests
#   ./test_comparison.sh complex_     # Run only complex_* tests
#   ./test_comparison.sh fx_1         # Run tests matching fx_1*
#   PARALLEL_JOBS=8 ./test_comparison.sh  # More parallel frame extraction jobs
#
# Note: Rendering tests run sequentially to avoid GPU resource contention.
# Frame extraction is parallelized for speed.

# Configuration
PARALLEL_JOBS=${PARALLEL_JOBS:-4}  # Default 4 parallel jobs (for frame extraction)

# Parse command line arguments
FILTER_PATTERN=""
RUN_ALL=false

if [ "$1" == "--all" ]; then
    RUN_ALL=true
elif [ -n "$1" ]; then
    FILTER_PATTERN="$1"
else
    # Default: only run basic_* tests
    FILTER_PATTERN="basic_"
fi

# Ensure we are in the correct directory
if [ -f "assets/am" ] || [ -d "assets/am" ]; then
    BASE_DIR="."
    ASSETS_DIR="assets/am"
    DEBUG_DIR="assets/debug"
elif [ -d "crates/bevy_alight_motion/assets/am" ]; then
    BASE_DIR="crates/bevy_alight_motion"
    ASSETS_DIR="crates/bevy_alight_motion/assets/am"
    DEBUG_DIR="crates/bevy_alight_motion/assets/debug"
else
    echo "Error: Cannot find assets/am directory for bevy_alight_motion"
    exit 1
fi

echo "========================================"
echo "Video Comparison Test Suite"
echo "========================================"
echo "Frame extraction parallelism: $PARALLEL_JOBS"
if [ "$RUN_ALL" = true ]; then
    echo "Mode: Running ALL tests (basic_*, fx_*, complex_*)"
elif [ -n "$FILTER_PATTERN" ]; then
    echo "Mode: Running tests matching '$FILTER_PATTERN*'"
fi
echo ""

# Build player example first
echo "Building player example..."
cargo build -p bevy_alight_motion --example player --features video-comparison --release
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

echo "Using binary: $PLAYER_BIN"

# Get all matching amproj files that have corresponding videos
EXAMPLES=""
for amproj in "$ASSETS_DIR"/*.amproj; do
    name=$(basename "$amproj" .amproj)
    # Check if matches filter pattern (basic_, fx_, complex_)
    if echo "$name" | grep -qE '^(basic_|fx_|complex_)'; then
        # Apply prefix filter if not running all
        if [ "$RUN_ALL" = true ]; then
            # Check if video exists
            if [ -f "${DEBUG_DIR}/${name}.mp4" ]; then
                EXAMPLES="$EXAMPLES $name"
            fi
        elif [ -z "$FILTER_PATTERN" ] || echo "$name" | grep -q "^${FILTER_PATTERN}"; then
            # Check if video exists
            if [ -f "${DEBUG_DIR}/${name}.mp4" ]; then
                EXAMPLES="$EXAMPLES $name"
            fi
        fi
    fi
done
EXAMPLES=$(echo $EXAMPLES | tr ' ' '\n' | sort | tr '\n' ' ')

EXAMPLE_COUNT=$(echo "$EXAMPLES" | wc -w)
echo ""
echo "Found $EXAMPLE_COUNT examples with videos to test"
echo ""

if [ "$EXAMPLE_COUNT" -eq 0 ]; then
    echo "No examples to test!"
    exit 0
fi

# Create temporary directory for results
RESULTS_DIR=$(mktemp -d)
trap "rm -rf $RESULTS_DIR" EXIT

# Phase 1: Pre-extract video frames in parallel (CPU-bound)
echo "========================================"
echo "Phase 1: Extracting video frames (parallel)"
echo "========================================"

FRAME_CACHE_DIR="${DEBUG_DIR}/_video_frames"
mkdir -p "$FRAME_CACHE_DIR"

extract_frames_for_video() {
    local name=$1
    local video_path="${DEBUG_DIR}/${name}.mp4"
    local frame_dir="${FRAME_CACHE_DIR}/${name}"
    local marker_file="${frame_dir}/.extracted"
    
    # Skip if already extracted (cache hit)
    if [ -f "$marker_file" ]; then
        echo "  [CACHE] $name"
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
    
    local frame_count=$(ls "$frame_dir"/*.png 2>/dev/null | wc -l)
    echo "  [DONE] $name ($frame_count frames)"
}

export -f extract_frames_for_video
export DEBUG_DIR FRAME_CACHE_DIR

# Run frame extraction in parallel
echo "Extracting frames with $PARALLEL_JOBS parallel jobs..."
echo "$EXAMPLES" | tr ' ' '\n' | xargs -P "$PARALLEL_JOBS" -I {} bash -c 'extract_frames_for_video "$@"' _ {}

echo ""
echo "Frame extraction complete!"
echo ""

# Phase 2: Run rendering tests (sequential to avoid GPU contention)
echo "========================================"
echo "Phase 2: Running comparison tests (sequential)"
echo "========================================"
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
    local example=$1
    local result_file="$RESULTS_DIR/${example}.result"
    local log_file="$RESULTS_DIR/${example}.log"
    
    # Run directly without virtual framebuffer for consistent results
    # with manual testing. This requires a real display or proper GPU access.
    CARGO_MANIFEST_DIR="$MANIFEST_DIR" \
        "$PLAYER_BIN" "$example" > "$log_file" 2>&1
    
    local exit_code=$?
    
    # Determine result from log
    if grep -q "RESULT: PASS" "$log_file"; then
        echo "PASS|$example|" > "$result_file"
        echo "✅ $example"
    elif grep -q "RESULT: SKIP" "$log_file"; then
        echo "SKIP|$example|" > "$result_file"
        echo "⚠️  $example (SKIP)"
    elif grep -q "RESULT: CANCELLED" "$log_file"; then
        echo "CANCELLED|$example|" > "$result_file"
        echo "⛔ $example (CANCELLED by user)"
    else
        # Extract failure details (both Average Similarity and Per-Frame Pass Rate)
        avg_sim=$(grep "Average Similarity" "$log_file" | head -1)
        frame_rate=$(grep "Per-Frame Pass Rate" "$log_file" | head -1)
        echo "FAIL|$example|$avg_sim|$frame_rate" > "$result_file"
        echo "❌ $example (FAIL)"
    fi
}

# Run tests sequentially (GPU doesn't handle parallel rendering well)
for example in $EXAMPLES; do
    run_single_test "$example"
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
FAILED_EXAMPLES=""

for result_file in "$RESULTS_DIR"/*.result; do
    if [ -f "$result_file" ]; then
        IFS='|' read -r status name avg_details frame_details < "$result_file"
        if [ "$status" == "PASS" ]; then
            printf "%-40s | \033[0;32m✅ PASS\033[0m\n" "$name"
            PASSED_COUNT=$((PASSED_COUNT + 1))
        elif [ "$status" == "SKIP" ]; then
            printf "%-40s | \033[0;33m⚠️ SKIP\033[0m\n" "$name"
            SKIPPED_COUNT=$((SKIPPED_COUNT + 1))
        elif [ "$status" == "CANCELLED" ]; then
            printf "%-40s | \033[0;33m⛔ CANCELLED\033[0m\n" "$name"
            SKIPPED_COUNT=$((SKIPPED_COUNT + 1))
        else
            printf "%-40s | \033[0;31m❌ FAIL\033[0m\n" "$name"
            if [ -n "$avg_details" ]; then
                printf "%-40s   %s\n" "" "$avg_details"
            fi
            if [ -n "$frame_details" ]; then
                printf "%-40s   %s\n" "" "$frame_details"
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
echo "Passed:  $PASSED_COUNT"
echo "Skipped: $SKIPPED_COUNT"
echo "Failed:  $FAILED_COUNT"

if [ $FAILED_COUNT -gt 0 ]; then
    echo -e "Failed Examples:$FAILED_EXAMPLES"
    exit 1
else
    echo "🎉 All tests passed!"
    exit 0
fi