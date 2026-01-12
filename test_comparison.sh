#!/bin/bash

# Automated Video Comparison Testing Script
# Runs visual regression tests for all basic_* and fx_* examples
# Checks against thresholds defined in comparison_config.toml
# Supports sequential execution with batch result reporting

# Ensure we are in the correct directory
# Check for bevy_alight_motion specific file to determine location
if [ -f "assets/am" ] || [ -d "assets/am" ]; then
    # In bevy_alight_motion dir
    BASE_DIR="."
    ASSETS_DIR="assets/am"
elif [ -d "crates/bevy_alight_motion/assets/am" ]; then
    # In workspace root
    BASE_DIR="crates/bevy_alight_motion"
    ASSETS_DIR="crates/bevy_alight_motion/assets/am"
else
    echo "Error: Cannot find assets/am directory for bevy_alight_motion"
    exit 1
fi

echo "Building player example..."
cargo build -p bevy_alight_motion --example player --features video-comparison --release

# Get all basic_*.amproj, fx_*.amproj, and complex_*.amproj files
# Filter only basic_, fx_, and complex_ examples
EXAMPLES=$(ls "$ASSETS_DIR"/*.amproj 2>/dev/null | sed 's|.*/||; s|\.amproj$||' | grep -E '^(basic_|fx_|complex_)' | sort)

# Create temporary directory for results
RESULTS_DIR=$(mktemp -d)
trap "rm -rf $RESULTS_DIR" EXIT

echo ""
echo "Starting Comparison Tests..."
echo "========================================"
echo "Examples to test: $(echo "$EXAMPLES" | wc -l)"
echo ""

# Run tests - sequential for reliability with GUI apps
# GUI apps can conflict when run in parallel on X11
run_single_test() {
    local example=$1
    local result_file="$RESULTS_DIR/${example}.result"
    
    # Run the comparison test
    OUTPUT=$(cargo run --release -p bevy_alight_motion --example player --features video-comparison -- "$example" 2>&1)
    
    # Determine result
    if echo "$OUTPUT" | grep -q "RESULT: PASS"; then
        echo "PASS|$example|" > "$result_file"
        echo "✅ PASS: $example"
    elif echo "$OUTPUT" | grep -q "RESULT: SKIP"; then
        echo "SKIP|$example|" > "$result_file"
        echo "⚠️  SKIP: $example"
    else
        # Extract failure details
        avg_sim=$(echo "$OUTPUT" | grep "Average Similarity" | head -1)
        echo "FAIL|$example|$avg_sim" > "$result_file"
        echo "❌ FAIL: $example"
    fi
}

# Run tests sequentially (GUI apps don't parallelize well on X11)
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
        IFS='|' read -r status name details < "$result_file"
        if [ "$status" == "PASS" ]; then
            printf "%-40s | \033[0;32m✅ PASS\033[0m\n" "$name"
            PASSED_COUNT=$((PASSED_COUNT + 1))
        elif [ "$status" == "SKIP" ]; then
            printf "%-40s | \033[0;33m⚠️ SKIP\033[0m\n" "$name"
            SKIPPED_COUNT=$((SKIPPED_COUNT + 1))
        else
            printf "%-40s | \033[0;31m❌ FAIL\033[0m\n" "$name"
            if [ -n "$details" ]; then
                printf "%-40s   %s\n" "" "$details"
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