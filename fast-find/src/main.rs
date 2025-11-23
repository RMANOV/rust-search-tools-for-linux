use anyhow::Result;
use clap::Parser;

mod cli;
mod search;
mod file_walker;
mod pattern_matcher;
mod output;
mod worker;

#[cfg(test)]
mod tests;

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
//    - Search patterns (name, iname, path, ipath, type, size, time)
//    - Path traversal options (depth, hidden files, follow symlinks)
//    - Output formatting (print, print0, json, colored)
//
// 2. SearchEngine (search.rs) - Main orchestrator
//    - Coordinates file discovery and filtering
//    - Manages worker pool for parallel processing
//    - Handles results aggregation and output
//
// 3. FileWalker (file_walker.rs) - Smart directory traversal
//    - Parallel directory walking with rayon
//    - Respects .gitignore and file type filters
//    - Handles symlinks and permissions
//
// 4. PatternMatcher (pattern_matcher.rs) - Optimized matching
//    - Glob pattern matching for names/paths
//    - Regex support for complex patterns
//    - Size, time, and permission filters
//
// 5. Output (output.rs) - Results formatting
//    - Different output formats (print, print0, json)
//    - Colored output with file type indicators
//    - Statistics and performance metrics
//
// 6. Worker (worker.rs) - Parallel processing
//    - File evaluation in parallel
//    - Load balancing across CPU cores
//    - Result collection and ordering