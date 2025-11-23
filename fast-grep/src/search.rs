use anyhow::Result;
use ignore::WalkBuilder;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use crate::cli::Args;
use crate::file_processor::FileProcessor;
use crate::output::OutputFormatter;
use crate::pattern_matcher::PatternMatcher;
use crate::worker::{SearchStats, WorkerPool};

pub struct SearchEngine {
    args: Args,
    pattern_matcher: PatternMatcher,
    file_processor: FileProcessor,
    output_formatter: OutputFormatter,
    worker_pool: WorkerPool,
}

impl SearchEngine {
    pub fn new(args: Args) -> Result<Self> {
        // Initialize pattern matcher
        let use_regex = args.use_regex && !args.fixed_strings;
        let pattern_matcher = PatternMatcher::new(
            &args.pattern,
            use_regex,
            args.ignore_case,
        )?;

        // Initialize file processor
        let file_processor = FileProcessor::new(
            args.max_filesize_bytes(),
            args.use_mmap,
        );

        // Initialize output formatter
        let show_filenames = !args.no_filename && args.paths.len() > 1;
        let output_formatter = OutputFormatter::new(
            args.line_numbers,
            show_filenames,
            args.should_use_colors(),
            args.json_output,
            args.get_before_context(),
            args.get_after_context(),
            args.only_matching,
            args.invert_match,
            args.count_only,
            args.files_only,
            args.files_without_matches,
        );

        // Initialize worker pool with context settings
        let worker_pool = WorkerPool::new(
            file_processor.clone(),
            pattern_matcher.clone(),
            args.get_threads(),
            args.invert_match,
        ).with_context(args.get_before_context(), args.get_after_context());

        Ok(Self {
            args,
            pattern_matcher,
            file_processor,
            output_formatter,
            worker_pool,
        })
    }

    pub fn run(&self) -> Result<()> {
        let start_time = Instant::now();
        
        // Discover files to search
        let files_to_search = self.discover_files()?;
        
        if files_to_search.is_empty() {
            eprintln!("No files to search");
            return Ok(());
        }

        let mut stats = SearchStats::new();
        
        // Different execution modes based on output requirements
        if self.args.files_without_matches {
            self.run_files_without_matches_mode(&files_to_search, &mut stats)?;
        } else if self.args.files_only {
            self.run_files_only_mode(&files_to_search, &mut stats)?;
        } else if self.args.count_only {
            self.run_count_mode(&files_to_search, &mut stats)?;
        } else {
            self.run_normal_mode(&files_to_search, &mut stats)?;
        }

        stats.processing_time_ms = start_time.elapsed().as_millis() as u64;
        
        // Print statistics if verbose
        if std::env::var("FGREP_VERBOSE").is_ok() {
            self.print_stats(&stats);
        }

        Ok(())
    }

    fn discover_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        for path in &self.args.paths {
            if path.is_file() {
                files.push(path.clone());
            } else if path.is_dir() {
                let mut walk_builder = WalkBuilder::new(path);
                
                // Configure walk options
                walk_builder
                    .hidden(!self.args.search_hidden)
                    .ignore(self.args.respect_ignore)
                    .git_ignore(self.args.respect_ignore)
                    .max_filesize(Some(self.args.max_filesize_bytes()));

                // Add file type filters
                if let Some(ref types) = self.args.file_types {
                    walk_builder.types(self.build_file_types(types, false)?);
                }
                
                if let Some(ref types) = self.args.exclude_types {
                    walk_builder.types(self.build_file_types(types, true)?);
                }

                for entry in walk_builder.build() {
                    let entry = entry?;
                    if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        files.push(entry.into_path());
                    }
                }
            }
        }

        Ok(files)
    }

    fn build_file_types(&self, types_str: &str, negate: bool) -> Result<ignore::types::Types> {
        let mut builder = ignore::types::TypesBuilder::new();
        builder.add_defaults();
        
        for type_name in types_str.split(',') {
            if negate {
                builder.negate(type_name);
            } else {
                builder.select(type_name);
            }
        }
        
        Ok(builder.build()?)
    }

    fn run_files_only_mode(&self, files: &[PathBuf], stats: &mut SearchStats) -> Result<()> {
        let results = self.worker_pool.search_files(files.to_vec())?;
        let mut files_with_matches = std::collections::HashSet::new();
        
        for match_result in results {
            if !files_with_matches.contains(&match_result.file_path) {
                println!("{}", self.output_formatter.format_filename_only(&match_result.file_path));
                files_with_matches.insert(match_result.file_path.clone());
            }
        }
        
        for file_path in files {
            let had_matches = files_with_matches.contains(file_path);
            stats.add_file(had_matches, self.get_file_size(file_path), if had_matches { 1 } else { 0 });
        }
        Ok(())
    }

    fn run_files_without_matches_mode(&self, files: &[PathBuf], stats: &mut SearchStats) -> Result<()> {
        let results = self.worker_pool.search_files(files.to_vec())?;
        let mut files_with_matches = std::collections::HashSet::new();
        
        // Collect all files that have matches
        for match_result in results {
            files_with_matches.insert(match_result.file_path.clone());
        }
        
        // Print files that have NO matches
        for file_path in files {
            let had_matches = files_with_matches.contains(file_path);
            if !had_matches {
                println!("{}", self.output_formatter.format_filename_only(file_path));
            }
            stats.add_file(had_matches, self.get_file_size(file_path), if had_matches { 1 } else { 0 });
        }
        Ok(())
    }

    fn run_count_mode(&self, files: &[PathBuf], stats: &mut SearchStats) -> Result<()> {
        let results = self.worker_pool.search_files(files.to_vec())?;
        let mut file_counts: HashMap<PathBuf, usize> = HashMap::new();
        
        for match_result in results {
            *file_counts.entry(match_result.file_path).or_insert(0) += 1;
        }
        
        for file_path in files {
            let count = file_counts.get(file_path).copied().unwrap_or(0);
            if count > 0 {
                println!("{}", self.output_formatter.format_count(file_path, count));
            }
            stats.add_file(count > 0, self.get_file_size(file_path), count);
        }
        Ok(())
    }

    fn run_normal_mode(&self, files: &[PathBuf], stats: &mut SearchStats) -> Result<()> {
        let results = self.worker_pool.search_files(files.to_vec())?;
        let mut current_file: Option<PathBuf> = None;
        let mut file_has_matches = false;

        for match_result in results {
            // Print file header if this is a new file
            if current_file.as_ref() != Some(&match_result.file_path) {
                if files.len() > 1 && file_has_matches {
                    println!(); // Blank line between files
                }
                current_file = Some(match_result.file_path.clone());
                file_has_matches = true;
            }

            // Print context before
            for (line_num, content) in &match_result.context_before {
                println!("{}", self.output_formatter.format_context_line(
                    &match_result.file_path,
                    *line_num,
                    content,
                    true,
                ));
            }

            // Print the match
            println!("{}", self.output_formatter.format_match(
                &match_result.file_path,
                match_result.line_number,
                &match_result.line_content,
                match_result.match_start,
                match_result.match_end,
            ));

            // Print context after
            for (line_num, content) in &match_result.context_after {
                println!("{}", self.output_formatter.format_context_line(
                    &match_result.file_path,
                    *line_num,
                    content,
                    false,
                ));
            }

            // Print separator if there's context
            if !match_result.context_before.is_empty() || !match_result.context_after.is_empty() {
                println!("{}", self.output_formatter.format_separator());
            }
        }

        // Update stats
        stats.files_processed = files.len();
        Ok(())
    }

    fn get_file_size(&self, path: &PathBuf) -> u64 {
        std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    }

    fn print_stats(&self, stats: &SearchStats) {
        eprintln!("Search Statistics:");
        eprintln!("  Files processed: {}", stats.files_processed);
        eprintln!("  Files with matches: {}", stats.files_with_matches);
        eprintln!("  Total matches: {}", stats.total_matches);
        eprintln!("  Bytes processed: {} MB", stats.bytes_processed / (1024 * 1024));
        eprintln!("  Processing time: {} ms", stats.processing_time_ms);
        eprintln!("  Throughput: {:.2} MB/s", stats.throughput_mb_per_second());
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_file_discovery() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, "hello world").unwrap();
        
        let args = Args {
            pattern: "hello".to_string(),
            paths: vec![temp_dir.path().to_path_buf()],
            use_regex: false,
            fixed_strings: false,
            ignore_case: false,
            line_numbers: true,
            files_only: false,
            count_only: false,
            invert_match: false,
            only_matching: false,
            files_without_matches: false,
            no_filename: false,
            recursive: true,
            before_context: None,
            after_context: None,
            context: None,
            threads: None,
            file_types: None,
            exclude_types: None,
            respect_ignore: true,
            search_hidden: false,
            color: crate::cli::ColorOption::Auto,
            no_color: false,
            json_output: false,
            max_filesize_mb: 100,
            use_mmap: true,
        };
        
        let engine = SearchEngine::new(args).unwrap();
        let files = engine.discover_files().unwrap();
        
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], test_file);
    }
}