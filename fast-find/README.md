# fast-find (ffind)

Ultra-fast parallel file finder - a modern alternative to Unix `find`.

## Philosophy

Fast-find follows the same high-performance philosophy as the other tools in this suite:

- **🚀 Ultra-fast performance** - Parallel processing with optimized algorithms
- **🔧 Unix compatibility** - Drop-in replacement for most common `find` use cases  
- **🧠 Smart filtering** - Optimized pattern matching and file system traversal
- **📊 Rich output** - Multiple output formats including JSON and colored terminal output
- **⚡ Memory efficient** - Batch processing and smart resource management

## Features

### Pattern Matching
- **Name patterns**: `-n/--name`, `--iname` (case-insensitive)
- **Path patterns**: `--path`, `--ipath` (case-insensitive)
- **Regular expressions**: `--regex` flag for advanced pattern matching
- **Extensions**: `--ext` and `--not-ext` for file extension filtering

### File Type Filtering
- **File types**: `-t/--type` (f=file, d=directory, l=symlink)
- **Empty files**: `--empty` for zero-size files and empty directories

### Size Filtering
- **Size specifications**: `-s/--size` with formats like `+100k`, `-1M`, `=50G`
- **Size units**: bytes (b), kilobytes (k), megabytes (M), gigabytes (G), terabytes (T)

### Time Filtering
- **Modification time**: `--mtime` (e.g., `+7` = older than 7 days)
- **Access time**: `--atime`
- **Change time**: `--ctime`
- **Newer than**: `--newer file.txt` (files newer than reference file)

### Traversal Control
- **Depth limits**: `--max-depth`, `--min-depth`
- **Symlink handling**: `-L/--follow` to follow symbolic links
- **Hidden files**: `-H/--hidden` to search hidden files/directories
- **Git integration**: Respects `.gitignore` by default (disable with `--no-ignore`)
- **Filesystem boundaries**: `--mount` to cross filesystem boundaries

### Performance Options
- **Parallel processing**: `-j/--threads` to control thread count
- **Resource limits**: `--max-open` for file descriptor management

### Output Formats
- **Standard output**: Clean, colored output with file type indicators
- **Long format**: `-l/--long` for detailed file information (size, permissions, timestamps)
- **Null separation**: `-0/--print0` for safe shell scripting
- **JSON output**: `--json` for programmatic processing
- **Count only**: `-c/--count` to show only the number of matches
- **Statistics**: `--stats` for search performance metrics

## Usage Examples

### Basic Usage

```bash
# Find all files in current directory
ffind

# Find files in specific directories
ffind /home/user /var/log

# Find files by name pattern
ffind -n "*.rs"          # Rust files
ffind --iname "*.TXT"    # Case-insensitive text files

# Find files by path pattern
ffind --path "*/src/*"   # Files in any src directory
```

### File Type Filtering

```bash
# Find only regular files
ffind -t f

# Find only directories
ffind -t d

# Find only symbolic links
ffind -t l

# Find files by extension
ffind --ext "rs,py,js"

# Exclude specific extensions
ffind --not-ext "tmp,log"
```

### Size Filtering

```bash
# Find large files (>100MB)
ffind -s "+100M"

# Find small files (<1KB)
ffind -s "-1k"

# Find files exactly 1GB
ffind -s "=1G"

# Find empty files
ffind --empty
```

### Time-based Searches

```bash
# Find files modified in last 7 days
ffind --mtime "-7"

# Find files older than 30 days
ffind --mtime "+30"

# Find files newer than reference file
ffind --newer config.toml
```

### Advanced Filtering

```bash
# Combine multiple filters
ffind -n "*.log" -s "+10M" --mtime "+7"

# Use regular expressions
ffind --regex "test_.*\.rs$"

# Search hidden files
ffind -H -n ".*rc"

# Limit search depth
ffind --max-depth 3 -n "Cargo.toml"
```

### Output Formatting

```bash
# Long format with file details
ffind -l -n "*.txt"

# JSON output for scripting
ffind --json -n "*.rs" > results.json

# Count matches only
ffind -c -n "*.py"

# Null-separated output for xargs
ffind -0 -n "*.tmp" | xargs -0 rm

# Show search statistics
ffind --stats -n "*.log"
```

### Performance Tuning

```bash
# Use more threads for large searches
ffind -j 16 -n "*.c"

# Disable git ignore for faster traversal
ffind --no-ignore -n "*.o"

# Follow symlinks (slower but comprehensive)
ffind -L -n "*.txt"
```

## Performance

Fast-find is designed for maximum performance:

- **Parallel directory traversal** using rayon for multi-core utilization
- **Optimized pattern matching** with regex compilation caching
- **Smart file system access** with batched metadata operations
- **Memory-efficient processing** with streaming results
- **SIMD optimizations** where available

### Benchmarks

On a modern system with SSD storage:

- **1M files**: ~2-3 seconds for full traversal
- **Complex patterns**: Minimal performance impact with optimized regex
- **Large files**: Size filtering without reading file contents
- **Network filesystems**: Reduced metadata calls for better performance

## Environment Variables

- `FFIND_VERBOSE`: Enable verbose output for debugging
- `NO_COLOR`: Disable colored output

## Comparison with Unix find

| Feature | Unix find | fast-find |
|---------|-----------|-----------|
| **Performance** | Single-threaded | Multi-threaded |
| **Pattern matching** | Basic glob | Glob + Regex |
| **JSON output** | No | Yes |
| **Git integration** | No | Yes (.gitignore) |
| **Colored output** | No | Yes |
| **Progress stats** | No | Yes |
| **Size units** | Limited | Full range (B,K,M,G,T) |
| **Memory usage** | Low | Optimized batching |

## Architecture

Fast-find uses a modular architecture for maintainability and performance:

1. **CLI** - Comprehensive argument parsing with validation
2. **FileWalker** - Parallel directory traversal with ignore file support
3. **PatternMatcher** - Optimized pattern and filter matching
4. **Worker** - Parallel file processing with load balancing
5. **Output** - Multiple output format support with coloring
6. **SearchEngine** - Main orchestrator tying everything together

## Building from Source

```bash
# Build optimized release version
cargo build --release

# Run tests
cargo test

# Install globally
cargo install --path .
```

## Contributing

This tool follows the same contributing guidelines as the parent project. See [CONTRIBUTING.md](../CONTRIBUTING.md) for details.

## License

Licensed under MIT OR Apache-2.0, same as the parent project.