use anyhow::Result;
use std::time::Instant;

use crate::cli::Args;
use crate::file_walker::{FileWalker, WalkStats};
use crate::output::{OutputFormatter, SearchStats};
use crate::pattern_matcher::PatternMatcher;
use crate::worker::{BatchProcessor, ProcessingStats};

pub struct SearchEngine {
    args: Args,
    pattern_matcher: PatternMatcher,
    file_walker: FileWalker,
    output_formatter: OutputFormatter,
    batch_processor: BatchProcessor,
}

impl SearchEngine {
    pub fn new(args: Args) -> Result<Self> {
        // Validate arguments first
        args.validate().map_err(|e| anyhow::anyhow!("Invalid arguments: {}", e))?;

        // Initialize pattern matcher
        let pattern_matcher = PatternMatcher::new(&args)?;

        // Initialize file walker
        let file_walker = FileWalker::new(args.clone());

        // Initialize output formatter
        let output_formatter = OutputFormatter::new(
            !args.no_color,
            args.long_format,
            args.print0,
            args.json_output,
        );

        // Initialize batch processor
        let batch_processor = BatchProcessor::new(
            pattern_matcher.clone(),
            args.get_threads(),
            Some(2000), // Batch size for memory efficiency
        );

        Ok(Self {
            args,
            pattern_matcher,
            file_walker,
            output_formatter,
            batch_processor,
        })
    }

    pub fn run(&self) -> Result<()> {
        let start_time = Instant::now();

        // Phase 1: Walk the file system
        if std::env::var("FFIND_VERBOSE").is_ok() {
            eprintln!("Starting filesystem walk...");
        }
        
        let walk_results = self.file_walker.walk()?;
        let walk_stats = self.file_walker.get_stats();
        
        if std::env::var("FFIND_VERBOSE").is_ok() {
            eprintln!("Walk completed: {} entries found", walk_results.len());
        }

        // Phase 2: Process and filter files
        if std::env::var("FFIND_VERBOSE").is_ok() {
            eprintln!("Starting file processing...");
        }
        
        let processing_results = self.batch_processor.process_in_batches(walk_results)?;
        let processing_stats = self.batch_processor.get_stats(start_time.elapsed());

        if std::env::var("FFIND_VERBOSE").is_ok() {
            eprintln!("Processing completed: {} matches found", processing_results.len());
        }

        // Phase 3: Output results
        if self.args.count_only {
            self.output_count_only(processing_results.len())?;
        } else if self.args.json_output {
            self.output_json(processing_results, &walk_stats, &processing_stats)?;
        } else {
            self.output_normal(processing_results)?;
        }

        // Phase 4: Show statistics if requested
        if self.args.show_stats {
            self.show_statistics(&walk_stats, &processing_stats)?;
        }

        Ok(())
    }

    fn output_count_only(&self, count: usize) -> Result<()> {
        println!("{}", self.output_formatter.format_count(count));
        Ok(())
    }

    fn output_json(&self, results: Vec<crate::worker::ProcessingResult>, walk_stats: &WalkStats, processing_stats: &ProcessingStats) -> Result<()> {
        let file_infos: Vec<_> = results.into_iter().map(|r| r.file_info).collect();
        
        let search_stats = SearchStats {
            total_found: file_infos.len(),
            files_visited: walk_stats.files_visited,
            dirs_visited: walk_stats.dirs_visited,
            processing_time_ms: processing_stats.processing_time_ms,
        };

        let json_output = self.output_formatter.format_json_results(file_infos, search_stats)?;
        println!("{}", json_output);
        Ok(())
    }

    fn output_normal(&self, results: Vec<crate::worker::ProcessingResult>) -> Result<()> {
        for result in results {
            let path = std::path::Path::new(&result.file_info.path);
            
            // Get fresh metadata for accurate output formatting
            let metadata = std::fs::metadata(path).ok();
            
            let formatted_output = self.output_formatter.format_path(
                path,
                metadata.as_ref(),
                result.file_info.depth,
            )?;
            
            if !formatted_output.is_empty() {
                print!("{}", formatted_output);
                if !self.args.print0 {
                    println!();
                }
            }
        }
        Ok(())
    }

    fn show_statistics(&self, walk_stats: &WalkStats, processing_stats: &ProcessingStats) -> Result<()> {
        let search_stats = SearchStats {
            total_found: processing_stats.total_matched,
            files_visited: walk_stats.files_visited,
            dirs_visited: walk_stats.dirs_visited,
            processing_time_ms: processing_stats.processing_time_ms,
        };

        let stats_output = self.output_formatter.format_stats(&search_stats);
        
        if self.args.json_output {
            println!("{}", stats_output);
        } else {
            eprintln!("{}", stats_output);
            eprintln!("  Processing throughput: {:.1} entries/sec", processing_stats.throughput_per_second);
            
            // Additional performance metrics
            if processing_stats.processing_time_ms > 0 {
                let mb_per_sec = (walk_stats.total_entries() as f64 * 1000.0) / processing_stats.processing_time_ms as f64;
                eprintln!("  Entry processing rate: {:.0} entries/ms", mb_per_sec);
            }
        }
        
        Ok(())
    }
}

// Helper function to validate search patterns
pub fn validate_search_pattern(pattern: &str, use_regex: bool) -> Result<()> {
    if use_regex {
        regex::Regex::new(pattern)
            .map_err(|e| anyhow::anyhow!("Invalid regex pattern '{}': {}", pattern, e))?;
    } else {
        // Basic glob pattern validation
        if pattern.is_empty() {
            return Err(anyhow::anyhow!("Empty search pattern"));
        }
        
        // Check for potentially problematic patterns
        if pattern.contains("**") && pattern.matches("**").count() > 1 {
            return Err(anyhow::anyhow!("Multiple '**' patterns can cause performance issues"));
        }
    }
    
    Ok(())
}

// Helper function to estimate search complexity
pub fn estimate_search_complexity(args: &Args) -> SearchComplexity {
    let mut complexity = SearchComplexity::Low;
    
    // Pattern complexity
    if args.has_pattern_filters() {
        complexity = complexity.max(SearchComplexity::Medium);
        
        if args.use_regex {
            complexity = complexity.max(SearchComplexity::High);
        }
    }
    
    // Size and time filters add complexity
    if args.has_size_filters() || args.has_time_filters() {
        complexity = complexity.max(SearchComplexity::Medium);
    }
    
    // Deep searches are more complex
    if args.max_depth.is_none() || args.max_depth.unwrap_or(0) > 10 {
        complexity = complexity.max(SearchComplexity::Medium);
    }
    
    // Following symlinks increases complexity
    if args.follow_symlinks {
        complexity = complexity.max(SearchComplexity::High);
    }
    
    complexity
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SearchComplexity {
    Low,
    Medium,
    High,
}

impl SearchComplexity {
    pub fn recommended_thread_count(&self, available_cores: usize) -> usize {
        match self {
            SearchComplexity::Low => available_cores.min(4),
            SearchComplexity::Medium => available_cores,
            SearchComplexity::High => available_cores * 2, // Hyperthreading can help with I/O-bound tasks
        }
    }
    
    pub fn recommended_batch_size(&self) -> usize {
        match self {
            SearchComplexity::Low => 5000,
            SearchComplexity::Medium => 2000,
            SearchComplexity::High => 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_search_engine_creation() {
        let args = Args::default();
        let engine = SearchEngine::new(args);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_validate_search_pattern() {
        assert!(validate_search_pattern("*.txt", false).is_ok());
        assert!(validate_search_pattern("test.*", true).is_ok());
        assert!(validate_search_pattern("[invalid", true).is_err());
        assert!(validate_search_pattern("", false).is_err());
    }

    #[test]
    fn test_search_complexity() {
        let mut args = Args::default();
        assert_eq!(estimate_search_complexity(&args), SearchComplexity::Low);
        
        args.name = Some("*.rs".to_string());
        assert_eq!(estimate_search_complexity(&args), SearchComplexity::Medium);
        
        args.use_regex = true;
        assert_eq!(estimate_search_complexity(&args), SearchComplexity::High);
        
        args.follow_symlinks = true;
        assert_eq!(estimate_search_complexity(&args), SearchComplexity::High);
    }

    #[test]
    fn test_complexity_recommendations() {
        let low = SearchComplexity::Low;
        let medium = SearchComplexity::Medium;
        let high = SearchComplexity::High;
        
        assert_eq!(low.recommended_thread_count(8), 4);
        assert_eq!(medium.recommended_thread_count(8), 8);
        assert_eq!(high.recommended_thread_count(8), 16);
        
        assert_eq!(low.recommended_batch_size(), 5000);
        assert_eq!(medium.recommended_batch_size(), 2000);
        assert_eq!(high.recommended_batch_size(), 1000);
    }

    #[test]
    fn test_search_engine_with_simple_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let args = Args {
            paths: vec![temp_dir.path().to_path_buf()],
            name: Some("*.txt".to_string()),
            ..Args::default()
        };

        let engine = SearchEngine::new(args).unwrap();
        // Just test that it doesn't crash
        let result = engine.run();
        assert!(result.is_ok());
    }
}