#!/bin/bash

echo "=== Testing Capture Groups Extraction ==="
echo ""

echo "Test 1: Basic capture group"
echo "OS Version: (macOS 14.0)"
echo ""

echo "Test 2: Multiple capture groups - should get first one"
echo "User: (john) Group: (admin) Role: (developer)"
echo ""

echo "Test 3: Nested parentheses - should get first outer group"
echo "Path: (/usr/local/bin/(app))"
echo ""

echo "Test 4: No capture groups - should warn and use full match"
echo "Status: running"
echo ""

echo "Test 5: Empty capture group"
echo "Empty: ()"
echo ""

echo "Test 6: Complex nested structure"
echo "Config: (main: (value1), sub: (value2))"
echo ""

echo "=== End of Tests ===" 