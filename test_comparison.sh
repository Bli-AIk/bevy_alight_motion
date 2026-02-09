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
    # Default: only run basic/* tests
    FILTER_PATTERN="basic/"
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
# New structure: assets/projects/{category}/{subcategory}/name.amproj + name.mp4
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
    
    # Apply category filter (basic/, effects/, complex/)
    if [ "$RUN_ALL" = true ]; then
        if [ -f "$video_path" ]; then
            EXAMPLES="$EXAMPLES $test_id"
        fi
    elif [ -n "$FILTER_PATTERN" ]; then
        # Filter can be category (basic, effects, complex) or subcategory
        if echo "$test_id" | grep -q "^${FILTER_PATTERN}"; then
            if [ -f "$video_path" ]; then
                EXAMPLES="$EXAMPLES $test_id"
            fi
        fi
    else
        # Default: only run basic/* tests
        if echo "$test_id" | grep -q "^basic/"; then
            if [ -f "$video_path" ]; then
                EXAMPLES="$EXAMPLES $test_id"
            fi
        fi
    fi
done < <(find "$PROJECTS_DIR" -name "*.amproj" -type f | sort)
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

FRAME_CACHE_DIR="${PROJECTS_DIR}/_video_frames"
mkdir -p "$FRAME_CACHE_DIR"

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
    
    local frame_count=$(ls "$frame_dir"/*.png 2>/dev/null | wc -l)
    echo "  [DONE] $test_id ($frame_count frames)"
}

export -f extract_frames_for_video
export PROJECTS_DIR FRAME_CACHE_DIR

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
    local test_id=$1
    # Create result file with flattened name (replace / with _)
    local flat_name=$(echo "$test_id" | tr '/' '_')
    local result_file="$RESULTS_DIR/${flat_name}.result"
    local log_file="$RESULTS_DIR/${flat_name}.log"
    
    # Run directly without virtual framebuffer for consistent results
    # with manual testing. This requires a real display or proper GPU access.
    CARGO_MANIFEST_DIR="$MANIFEST_DIR" \
        "$PLAYER_BIN" "$test_id" > "$log_file" 2>&1
    
    local exit_code=$?
    
    # Determine result from log
    if grep -q "RESULT: PASS" "$log_file"; then
        echo "PASS|$test_id|" > "$result_file"
        echo "✅ $test_id"
    elif grep -q "RESULT: SKIP" "$log_file"; then
        echo "SKIP|$test_id|" > "$result_file"
        echo "⚠️  $test_id (SKIP)"
    elif grep -q "RESULT: CANCELLED" "$log_file"; then
        echo "CANCELLED|$test_id|" > "$result_file"
        echo "⛔ $test_id (CANCELLED by user)"
    else
        # Extract failure details (both Average Similarity and Per-Frame Pass Rate)
        avg_sim=$(grep "Average Similarity" "$log_file" | head -1)
        frame_rate=$(grep "Per-Frame Pass Rate" "$log_file" | head -1)
        echo "FAIL|$test_id|$avg_sim|$frame_rate" > "$result_file"
        echo "❌ $test_id (FAIL)"
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

# Generate JSON results file (merge with existing results)
JSON_OUTPUT="${BASE_DIR}/test_results.json"

# Build new results as JSON entries
NEW_RESULTS=""
for result_file in "$RESULTS_DIR"/*.result; do
    if [ -f "$result_file" ]; then
        IFS='|' read -r status name avg_details frame_details < "$result_file"
        
        # Extract similarity value from avg_details if present
        avg_sim=""
        if [ -n "$avg_details" ]; then
            avg_sim=$(echo "$avg_details" | grep -oP '\d+\.\d+' | head -1)
        fi
        
        # Convert status to lowercase
        status_lower=$(echo "$status" | tr '[:upper:]' '[:lower:]')
        
        # Build JSON entry
        entry="\"$name\": { \"status\": \"$status_lower\""
        if [ -n "$avg_sim" ]; then
            entry="$entry, \"avg_similarity\": $avg_sim"
        fi
        entry="$entry }"
        
        if [ -n "$NEW_RESULTS" ]; then
            NEW_RESULTS="$NEW_RESULTS, $entry"
        else
            NEW_RESULTS="$entry"
        fi
    fi
done

# Merge with existing results using Python (preserves old results, updates with new)
python3 << EOF
import json
import os
from datetime import datetime

json_output = "$JSON_OUTPUT"
new_results_json = '''{$NEW_RESULTS}'''

# Parse new results
try:
    new_results = json.loads(new_results_json) if new_results_json.strip() and new_results_json.strip() != '{}' else {}
except json.JSONDecodeError:
    new_results = {}

# Load existing results if file exists
existing_results = {}
if os.path.exists(json_output):
    try:
        with open(json_output, 'r') as f:
            existing_data = json.load(f)
            existing_results = existing_data.get('results', {})
    except (json.JSONDecodeError, IOError):
        pass  # Start fresh if file is corrupted

# Merge: existing results + new results (new overwrites old for same keys)
merged_results = {**existing_results, **new_results}

# Recalculate summary from merged results
passed = sum(1 for r in merged_results.values() if r.get('status') == 'pass')
failed = sum(1 for r in merged_results.values() if r.get('status') == 'fail')
skipped = sum(1 for r in merged_results.values() if r.get('status') in ('skip', 'cancelled'))

# Build final JSON
output = {
    "timestamp": datetime.now().astimezone().isoformat(),
    "summary": {
        "passed": passed,
        "skipped": skipped,
        "failed": failed
    },
    "results": dict(sorted(merged_results.items()))
}

# Write output
with open(json_output, 'w') as f:
    json.dump(output, f, indent=2)

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