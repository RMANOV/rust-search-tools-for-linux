use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

mod cli;
mod search;
mod file_processor;
mod pattern_matcher;
mod output;
mod worker;
mod errors;

use cli::Args;
use search::SearchEngine;

fn main() -> Result<()> {
    let args = Args::parse();
    
    let search_engine = SearchEngine::new(args)?;
    search_engine.run()
}

// Architecture Overview:
// 
// 1. CLI (cli.rs) - Command line interface using clap
//    - Pattern, paths, options (case-insensitive, regex, etc.)
//    - Output formatting options (colors, line numbers, etc.)
//
// 2. SearchEngine (search.rs) - Main orchestrator
//    - Coordinates file discovery and processing
//    - Manages worker pool for parallel processing
//    - Handles results aggregation
//
// 3. FileProcessor (file_processor.rs) - Smart file reading
//    - Memory mapping for large files (>1MB)
//    - Regular buffered reading for small files
//    - Binary file detection and skipping
//
// 4. PatternMatcher (pattern_matcher.rs) - Optimized matching
//    - Aho-Corasick for literal string searches
//    - Regex engine for pattern matching
//    - SIMD-optimized byte searching with memchr
//
// 5. Output (output.rs) - Results formatting
//    - Colored output with line numbers
//    - Context lines (before/after)
//    - JSON output for programmatic use
//
// 6. Worker (worker.rs) - Parallel processing
//    - File queue management
//    - Result collection and ordering
//    - Load balancing across CPU cores