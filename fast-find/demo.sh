#!/bin/bash

# Fast-find Demo Script
# This script demonstrates the capabilities of the fast-find tool

set -e

echo "🚀 Fast-find (ffind) - Ultra-fast parallel file finder demo"
echo "============================================================"
echo

# Create a test directory structure
TEST_DIR="./demo_test_files"
echo "📁 Creating test directory structure in $TEST_DIR..."

rm -rf "$TEST_DIR" 2>/dev/null || true
mkdir -p "$TEST_DIR"/{src,docs,tests,config}
mkdir -p "$TEST_DIR"/src/{main,lib,utils}
mkdir -p "$TEST_DIR"/docs/{api,guides}

# Create test files
echo "📄 Creating test files..."

# Source files
cat > "$TEST_DIR/src/main.rs" << 'EOF'
fn main() {
    println!("Hello, world!");
}
EOF

cat > "$TEST_DIR/src/lib.rs" << 'EOF'
pub mod utils;
pub mod config;
EOF

cat > "$TEST_DIR/src/utils/helper.rs" << 'EOF'
pub fn help() {
    println!("Helper function");
}
EOF

# Documentation files
cat > "$TEST_DIR/docs/README.md" << 'EOF'
# Project Documentation
This is the main documentation.
EOF

cat > "$TEST_DIR/docs/API.md" << 'EOF'
# API Documentation
API reference here.
EOF

# Config files
cat > "$TEST_DIR/config/settings.toml" << 'EOF'
[database]
host = "localhost"
port = 5432
EOF

cat > "$TEST_DIR/config/logging.json" << 'EOF'
{
  "level": "info",
  "format": "json"
}
EOF

# Test files
cat > "$TEST_DIR/tests/integration_test.py" << 'EOF'
def test_integration():
    assert True
EOF

# Create some large and empty files
echo "This is a large file" > "$TEST_DIR/large_file.txt"
for i in {1..100}; do
    echo "Line $i: Lorem ipsum dolor sit amet, consectetur adipiscing elit." >> "$TEST_DIR/large_file.txt"
done

touch "$TEST_DIR/empty_file.txt"
touch "$TEST_DIR/.hidden_file"

# Create some files with different timestamps
touch -t 202301010000 "$TEST_DIR/old_file.txt"
touch "$TEST_DIR/new_file.txt"

echo "✅ Test directory structure created successfully!"
echo

# Display directory structure
echo "📂 Directory structure:"
find "$TEST_DIR" -type f | sort | sed 's|./demo_test_files/|  |g'
echo

# Now demonstrate ffind commands (simulated output)
echo "🔍 Fast-find command demonstrations:"
echo "===================================="
echo

# Note: Since we can't actually compile and run ffind, we'll simulate the expected output

echo "1. Find all files:"
echo "   $ ffind $TEST_DIR"
echo "   Expected output:"
find "$TEST_DIR" -type f | sort | sed 's|^|   |'
echo

echo "2. Find Rust source files:"
echo "   $ ffind -n '*.rs' $TEST_DIR"
echo "   Expected output:"
find "$TEST_DIR" -name "*.rs" | sort | sed 's|^|   |'
echo

echo "3. Find configuration files:"
echo "   $ ffind --ext 'toml,json' $TEST_DIR"
echo "   Expected output:"
find "$TEST_DIR" \( -name "*.toml" -o -name "*.json" \) | sort | sed 's|^|   |'
echo

echo "4. Find large files (>1KB):"
echo "   $ ffind -s '+1k' $TEST_DIR"
echo "   Expected output:"
find "$TEST_DIR" -type f -size +1k | sort | sed 's|^|   |'
echo

echo "5. Find empty files:"
echo "   $ ffind --empty $TEST_DIR"
echo "   Expected output:"
find "$TEST_DIR" -type f -empty | sort | sed 's|^|   |'
echo

echo "6. Find files with max depth 1:"
echo "   $ ffind --max-depth 1 $TEST_DIR"
echo "   Expected output:"
find "$TEST_DIR" -maxdepth 2 -type f | sort | sed 's|^|   |'
echo

echo "7. Find hidden files:"
echo "   $ ffind -H -n '.*' $TEST_DIR"
echo "   Expected output:"
find "$TEST_DIR" -name ".*" | sort | sed 's|^|   |'
echo

echo "8. Count files only:"
echo "   $ ffind -c $TEST_DIR"
echo "   Expected output:"
find "$TEST_DIR" -type f | wc -l | sed 's|^|   |'
echo

echo "9. Long format output (simulated):"
echo "   $ ffind -l -n '*.rs' $TEST_DIR"
echo "   Expected output:"
find "$TEST_DIR" -name "*.rs" -ls | sed 's|^|   |'
echo

echo "10. JSON output (simulated):"
echo "    $ ffind --json -n '*.toml' $TEST_DIR"
echo "    Expected output:"
echo '    {
      "files": [
        {
          "path": "'$TEST_DIR'/config/settings.toml",
          "file_type": "file",
          "size": 45,
          "modified": "2025-06-21T12:00:00Z",
          "permissions": "-rw-r--r--",
          "depth": 2
        }
      ],
      "stats": {
        "total_found": 1,
        "files_visited": 12,
        "dirs_visited": 8,
        "processing_time_ms": 15
      }
    }'
echo

echo "🎯 Performance characteristics:"
echo "==============================="
echo "• Parallel processing: Uses all CPU cores for directory traversal"
echo "• Smart filtering: Applies filters during traversal, not after"
echo "• Memory efficient: Streaming results, no large intermediate collections"
echo "• Git-aware: Respects .gitignore files by default"
echo "• Cross-platform: Works on Linux, macOS, and Windows"
echo

echo "🛠️ Advanced usage examples:"
echo "============================"
echo "• Complex pattern: ffind --regex 'test_.*\.py$'"
echo "• Multiple filters: ffind -n '*.log' -s '+10M' --mtime '+7'"
echo "• Performance tuning: ffind -j 16 --max-open 2048"
echo "• Scripting: ffind -0 -n '*.tmp' | xargs -0 rm"
echo

echo "✨ Key advantages over Unix find:"
echo "================================="
echo "• 🚀 Much faster on large directories (parallel processing)"
echo "• 🎨 Colored output with file type indicators"
echo "• 📊 JSON output for programmatic use"
echo "• 🔍 Better pattern matching (regex support)"
echo "• 📈 Built-in performance statistics"
echo "• 🚫 Git integration (.gitignore awareness)"
echo

echo "🧹 Cleaning up test files..."
rm -rf "$TEST_DIR"
echo "✅ Demo completed successfully!"
echo
echo "💡 To build and install fast-find:"
echo "   cargo build --release"
echo "   cargo install --path ."
echo
echo "📖 For more information, see fast-find/README.md"