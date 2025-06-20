# 🚀 Installation Guide

## 📋 Prerequisites

### 🦀 Rust Toolchain
```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 🔧 System Dependencies

#### Ubuntu/Debian:
```bash
sudo apt update
sudo apt install build-essential gcc
```

#### Fedora/RHEL/CentOS:
```bash
sudo dnf install gcc gcc-c++ make
# or for older versions:
# sudo yum install gcc gcc-c++ make
```

#### Arch Linux:
```bash
sudo pacman -S gcc make
```

#### macOS:
```bash
# Install Xcode Command Line Tools
xcode-select --install
```

## ⚡ Quick Installation

```bash
# Clone the repository
git clone https://github.com/your-username/rust-search-tools-for-linux
cd rust-search-tools-for-linux

# Build all tools with maximum optimizations
cargo build --release

# Install to system PATH
sudo cp target/release/{fgrep,ffind,fls,fdu} /usr/local/bin/

# Verify installation
fgrep --version
```

## 🎯 Build Individual Tools

```bash
# Build only fast-grep
cargo build --release --bin fgrep

# Build only fast-find  
cargo build --release --bin ffind

# Build only fast-ls
cargo build --release --bin fls

# Build only fast-du
cargo build --release --bin fdu
```

## 🧪 Development Setup

```bash
# Install development dependencies
cargo install cargo-watch cargo-criterion

# Run tests
cargo test --workspace

# Format code
cargo fmt --all

# Run linter
cargo clippy --all-targets --all-features

# Run benchmarks (requires nightly Rust)
cargo +nightly bench
```

## 🐳 Docker Installation

```bash
# Build Docker image
docker build -t rust-search-tools .

# Run in container
docker run --rm -v $(pwd):/workspace rust-search-tools fgrep "pattern" /workspace
```

## 📦 Package Managers

### 🍺 Homebrew (macOS/Linux)
```bash
# Coming soon!
brew install rust-search-tools
```

### 📦 Cargo
```bash
# Install from crates.io (coming soon!)
cargo install fast-grep fast-find fast-ls fast-du
```

## 🔧 Troubleshooting

### Missing C Compiler
If you get "linker `cc` not found" error:
- Install build tools for your platform (see Prerequisites)
- On Windows: Install Visual Studio Build Tools

### Compilation Errors
- Ensure you have Rust 1.70+ (`rustc --version`)
- Update dependencies: `cargo update`
- Clean build: `cargo clean && cargo build --release`

### Performance Issues
- Ensure release build: `--release` flag
- Check CPU architecture: `rustc --print target-list`
- Verify SIMD support: `cat /proc/cpuinfo | grep -E "(sse|avx)"`

## 🚀 Post-Installation

### Shell Completion
```bash
# Generate completions for your shell
fgrep --generate-completion bash > ~/.bash_completion.d/fgrep
fgrep --generate-completion zsh > ~/.zsh/completions/_fgrep
```

### Environment Variables
```bash
# Optional: Set default thread count
export RUST_SEARCH_THREADS=16

# Optional: Enable verbose output for debugging
export RUST_SEARCH_VERBOSE=1
```

### Performance Tuning
```bash
# For maximum performance on dedicated search machines
export RUST_SEARCH_SIMD=avx2
export RUST_SEARCH_MEMORY_STRATEGY=aggressive
```

## ✅ Verification

Test your installation:
```bash
# Create test data
echo "function test() { return 42; }" > test.js

# Test fast-grep
fgrep "function" test.js

# Expected output: colored highlighting of "function"
```

🎉 **You're ready to experience the fastest search tools on Linux!**