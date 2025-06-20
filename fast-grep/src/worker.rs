use anyhow::Result;
use crossbeam::channel::{self, Receiver, Sender};
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use crate::file_processor::{FileProcessor, FileContent};
use crate::output::MatchResult;
use crate::pattern_matcher::{PatternMatcher, Match};

pub struct WorkerPool {
    file_processor: Arc<FileProcessor>,
    pattern_matcher: Arc<PatternMatcher>,
    num_threads: usize,
}

impl WorkerPool {
    pub fn new(
        file_processor: FileProcessor,
        pattern_matcher: PatternMatcher,
        num_threads: usize,
    ) -> Self {
        Self {
            file_processor: Arc::new(file_processor),
            pattern_matcher: Arc::new(pattern_matcher),
            num_threads,
        }
    }

    pub fn search_files(&self, file_paths: Vec<PathBuf>) -> Result<Vec<MatchResult>> {
        // Use rayon for parallel processing of files
        let results: Result<Vec<Vec<MatchResult>>, _> = file_paths
            .par_iter()
            .map(|path| self.search_single_file(path))
            .collect();

        // Flatten results
        let mut all_matches = Vec::new();
        for file_results in results? {
            all_matches.extend(file_results);
        }

        Ok(all_matches)
    }

    fn search_single_file(&self, file_path: &PathBuf) -> Result<Vec<MatchResult>> {
        let file_content = self.file_processor.process_file(file_path)?;
        
        match file_content {
            FileContent::Binary => Ok(Vec::new()),
            _ => {
                let bytes = file_content.as_bytes().unwrap();
                let matches = self.pattern_matcher.find_matches(bytes);
                
                if matches.is_empty() {
                    return Ok(Vec::new());
                }

                // Convert byte matches to line-based matches
                self.convert_to_line_matches(file_path.clone(), &file_content, matches)
            }
        }
    }

    fn convert_to_line_matches(
        &self,
        file_path: PathBuf,
        file_content: &FileContent,
        matches: Vec<Match>,
    ) -> Result<Vec<MatchResult>> {
        let lines = file_content.lines().unwrap();
        let mut results = Vec::new();

        for pattern_match in matches {
            // Find which line contains this match
            if let Some(line) = lines.iter().find(|line| line.contains_position(pattern_match.start)) {
                let line_content = line.as_str()?.to_string();
                
                // Calculate match position relative to line start
                let match_start_in_line = pattern_match.start.saturating_sub(line.start);
                let match_end_in_line = pattern_match.end.saturating_sub(line.start);
                
                let match_result = MatchResult::new(
                    file_path.clone(),
                    line.number,
                    line_content,
                    match_start_in_line,
                    match_end_in_line,
                );
                
                results.push(match_result);
            }
        }

        Ok(results)
    }

    pub fn search_with_streaming<F>(&self, file_paths: Vec<PathBuf>, mut callback: F) -> Result<()>
    where
        F: FnMut(MatchResult) -> Result<()> + Send + Sync,
    {
        let (tx, rx): (Sender<MatchResult>, Receiver<MatchResult>) = channel::unbounded();
        let processed_files = Arc::new(AtomicUsize::new(0));
        let total_files = file_paths.len();

        // Spawn worker threads
        let handles: Vec<_> = (0..self.num_threads)
            .map(|_| {
                let tx = tx.clone();
                let file_processor = Arc::clone(&self.file_processor);
                let pattern_matcher = Arc::clone(&self.pattern_matcher);
                let processed_files = Arc::clone(&processed_files);
                
                thread::spawn(move || -> Result<()> {
                    // Each worker processes files in parallel using rayon
                    Ok(())
                })
            })
            .collect();

        // Process files in parallel
        file_paths.par_iter().try_for_each(|file_path| -> Result<()> {
            let matches = self.search_single_file(file_path)?;
            
            for match_result in matches {
                tx.send(match_result).map_err(|e| anyhow::anyhow!("Send error: {}", e))?;
            }
            
            processed_files.fetch_add(1, Ordering::Relaxed);
            Ok(())
        })?;

        // Close the channel
        drop(tx);

        // Process results as they come in
        while let Ok(match_result) = rx.recv() {
            callback(match_result)?;
        }

        Ok(())
    }
}

pub struct SearchStats {
    pub files_processed: usize,
    pub files_with_matches: usize,
    pub total_matches: usize,
    pub bytes_processed: u64,
    pub processing_time_ms: u64,
}

impl SearchStats {
    pub fn new() -> Self {
        Self {
            files_processed: 0,
            files_with_matches: 0,
            total_matches: 0,
            bytes_processed: 0,
            processing_time_ms: 0,
        }
    }

    pub fn add_file(&mut self, had_matches: bool, file_size: u64, match_count: usize) {
        self.files_processed += 1;
        if had_matches {
            self.files_with_matches += 1;
        }
        self.total_matches += match_count;
        self.bytes_processed += file_size;
    }

    pub fn throughput_mb_per_second(&self) -> f64 {
        if self.processing_time_ms == 0 {
            return 0.0;
        }
        
        let seconds = self.processing_time_ms as f64 / 1000.0;
        let mb = self.bytes_processed as f64 / (1024.0 * 1024.0);
        mb / seconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern_matcher::PatternMatcher;
    use crate::file_processor::FileProcessor;

    #[test]
    fn test_worker_pool_creation() {
        let file_processor = FileProcessor::new(1024 * 1024, true);
        let pattern_matcher = PatternMatcher::new("test", false, false).unwrap();
        let worker_pool = WorkerPool::new(file_processor, pattern_matcher, 4);
        
        assert_eq!(worker_pool.num_threads, 4);
    }

    #[test]
    fn test_search_stats() {
        let mut stats = SearchStats::new();
        stats.add_file(true, 1024, 5);
        stats.add_file(false, 2048, 0);
        
        assert_eq!(stats.files_processed, 2);
        assert_eq!(stats.files_with_matches, 1);
        assert_eq!(stats.total_matches, 5);
        assert_eq!(stats.bytes_processed, 3072);
    }
}