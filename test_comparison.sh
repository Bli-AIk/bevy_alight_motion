#!/bin/bash

# Automated Video Comparison Testing Script
# Runs visual regression tests for all basic_* and fx_* examples
# Checks against thresholds defined in comparison_config.toml

# Ensure we are in the project root (where Cargo.toml is)
# This handles being run from crates/bevy_alight_motion/ or root
if [ -f "Cargo.toml" ]; then
    # In bevy_alight_motion dir
    BASE_DIR="."
    ASSETS_DIR="assets/am"
elif [ -f "crates/bevy_alight_motion/Cargo.toml" ]; then
    # In workspace root
    BASE_DIR="crates/bevy_alight_motion"
    ASSETS_DIR="crates/bevy_alight_motion/assets/am"
else
    echo "Error: Cannot find Cargo.toml for bevy_alight_motion"
    exit 1
fi

echo "Building player example..."
cargo build -p bevy_alight_motion --example player --features video-comparison

# Get all basic_*.amproj, fx_*.amproj, and complex_*.amproj files
# Use find to robustly get files, then basename to strip path and extension
# Filter only basic_, fx_, and complex_ examples
EXAMPLES=$(find "$ASSETS_DIR" -name "*.amproj" -print0 | xargs -0 -n 1 basename | sed 's/\.amproj//' | grep -E '^(basic_|fx_|complex_)' | sort)

PASSED_COUNT=0
FAILED_COUNT=0
FAILED_EXAMPLES=""

echo ""
echo "Starting Comparison Tests..."
echo "----------------------------------------"

for example in $EXAMPLES; do
    echo "Running Test: $example"
    
    # Run the comparison test (it will auto-exit when finished)
    # Capture output to log file to keep console clean, but print result status
    
    OUTPUT=$(cargo run -p bevy_alight_motion --example player --features video-comparison -- "$example" 2>&1)
    EXIT_CODE=$?
    
    # Check for specific success string in output just in case exit code is unreliable
    if echo "$OUTPUT" | grep -q "RESULT: PASS"; then
        EXIT_CODE=0
    fi
    
    if [ $EXIT_CODE -eq 0 ]; then
        echo "✅ PASS: $example"
        PASSED_COUNT=$((PASSED_COUNT + 1))
    else
        echo "❌ FAIL: $example"
        FAILED_COUNT=$((FAILED_COUNT + 1))
        FAILED_EXAMPLES="${FAILED_EXAMPLES}\n  - ${example}"
        
        # Print the last few lines of output to show failure reason
        echo "  Failure Details:"
        echo "$OUTPUT" | grep "Similarity" | tail -n 5 | sed 's/^/    /'
        echo "$OUTPUT" | grep "Average Similarity" | sed 's/^/    /'
    fi
    echo "----------------------------------------"
done

echo ""
echo "========================================"
echo "TEST SUMMARY"
echo "========================================"
echo "Passed: $PASSED_COUNT"
echo "Failed: $FAILED_COUNT"

if [ $FAILED_COUNT -gt 0 ]; then
    echo -e "Failed Examples:$FAILED_EXAMPLES"
    exit 1
else
    echo "🎉 All tests passed!"
    exit 0
fi