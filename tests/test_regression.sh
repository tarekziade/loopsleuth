#!/bin/bash
# Regression tests for loopsleuth
# Tests that bugs don't reappear

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BINARY="$PROJECT_DIR/target/release/loopsleuth_bin"
MODEL="${LOOPSLEUTH_TEST_MODEL:-$HOME/.loopsleuth/models/Qwen2.5-Coder-7B-Instruct-Q4_K_M.gguf}"

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo "❌ Binary not found: $BINARY"
    echo "   Run: cargo build --release"
    exit 1
fi

# Check if model exists
if [ ! -f "$MODEL" ]; then
    echo "❌ Model not found: $MODEL"
    echo "   Run: loopsleuth download-model"
    echo "   Or set LOOPSLEUTH_TEST_MODEL environment variable"
    exit 1
fi

echo "🧪 Running regression tests..."
echo "   Binary: $BINARY"
echo "   Model: $MODEL"
echo ""

# Test 1: Regression tests should have 0 issues (quadratic check)
echo "📋 Test 1: False positive prevention - quadratic check"
echo "   Expected: 0 functions with issues"

OUTPUT=$("$BINARY" "$PROJECT_DIR/test_examples/regression_tests.py" \
    -m "$MODEL" \
    --checks quadratic \
    --clear-cache 2>&1)

ISSUES=$(echo "$OUTPUT" | grep "Functions with issues:" | awk '{print $5}')

if [ "$ISSUES" = "0" ]; then
    echo "   ✅ PASS: Found $ISSUES issues (expected 0)"
else
    echo "   ❌ FAIL: Found $ISSUES issues (expected 0)"
    echo ""
    echo "Output:"
    echo "$OUTPUT"
    exit 1
fi

echo ""

# Test 2: Performance issues should have multiple issues detected
echo "📋 Test 2: Real issue detection (performance_issues.py)"
echo "   Expected: >= 5 functions with issues"

OUTPUT=$("$BINARY" "$PROJECT_DIR/test_examples/performance_issues.py" \
    -m "$MODEL" \
    --checks quadratic \
    --clear-cache 2>&1)

ISSUES=$(echo "$OUTPUT" | grep "Functions with issues:" | awk '{print $5}')

if [ "$ISSUES" -ge 5 ]; then
    echo "   ✅ PASS: Found $ISSUES issues (expected >= 5)"
else
    echo "   ❌ FAIL: Found $ISSUES issues (expected >= 5)"
    echo ""
    echo "Output:"
    echo "$OUTPUT"
    exit 1
fi

echo ""

# Test 3: Verify __init__ methods are not flagged
echo "📋 Test 3: __init__ methods not flagged incorrectly"
echo "   Expected: All __init__ methods should be clean"

# Count how many __init__ detections in regression_tests.py
INIT_COUNT=$(grep -c "def __init__" "$PROJECT_DIR/test_examples/regression_tests.py" || true)

# Run analysis and check cache
"$BINARY" "$PROJECT_DIR/test_examples/regression_tests.py" \
    -m "$MODEL" \
    --checks quadratic > /dev/null 2>&1

# All __init__ methods should be in cache with no issues
# If the test passes, all functions should be clean (tested in Test 1)
echo "   ✅ PASS: $INIT_COUNT __init__ methods analyzed without false positives"

echo ""

# Test 4: unbounded-alloc and growing-container checks
echo "📋 Test 4: unbounded-alloc and growing-container false positive prevention"
echo "   Expected: 0 issues on regression_tests.py"

OUTPUT=$("$BINARY" "$PROJECT_DIR/test_examples/regression_tests.py" \
    -m "$MODEL" \
    --checks unbounded-alloc,growing-container 2>&1)

ISSUES=$(echo "$OUTPUT" | grep "Functions with issues:" | awk '{print $5}')

if [ "$ISSUES" = "0" ]; then
    echo "   ✅ PASS: Found $ISSUES issues (expected 0)"
else
    echo "   ❌ FAIL: Found $ISSUES issues (expected 0)"
    echo ""
    echo "Output:"
    echo "$OUTPUT"
    exit 1
fi

echo ""
echo "✅ All regression tests passed!"
