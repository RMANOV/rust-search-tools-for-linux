use anyhow::Result;
use rayon::prelude::*;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::file_walker::WalkResult;
use crate::output::FileInfo;
use crate::pattern_matcher::PatternMatcher;

pub struct WorkerPool {
    pattern_matcher: PatternMatcher,
    thread_count: usize,
    processed_count: Arc<AtomicUsize>,
    matched_count: Arc<AtomicUsize>,
}

#[derive(Debug)]
pub struct ProcessingResult {
    pub file_info: FileInfo,
    pub matches: bool,
}

#[derive(Debug, Clone)]
pub struct ProcessingStats {
    pub total_processed: usize,
    pub total_matched: usize,
    pub processing_time_ms: u64,
    pub throughput_per_second: f64,
}

impl WorkerPool {
    pub fn new(pattern_matcher: PatternMatcher, thread_count: usize) -> Self {
        Self {
            pattern_matcher,
            thread_count,
            processed_count: Arc::new(AtomicUsize::new(0)),
            matched_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn process_files(&self, walk_results: Vec<WalkResult>) -> Result<Vec<ProcessingResult>> {
        let _start_time = Instant::now();
        
        // Configure rayon thread pool
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.thread_count)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create thread pool: {}", e))?;

        let processed_count = Arc::clone(&self.processed_count);
        let matched_count = Arc::clone(&self.matched_count);

        // Process files in parallel
        let results: Vec<ProcessingResult> = pool.install(|| {
            walk_results
                .par_iter()
                .filter_map(|walk_result| {
                    match self.process_single_file(walk_result) {
                        Ok(Some(result)) => {
                            processed_count.fetch_add(1, Ordering::Relaxed);
                            if result.matches {
                                matched_count.fetch_add(1, Ordering::Relaxed);
                            }
                            Some(result)
                        }
                        Ok(None) => {
                            processed_count.fetch_add(1, Ordering::Relaxed);
                            None
                        }
                        Err(err) => {
                            eprintln!("Warning: Failed to process {}: {}", walk_result.path.display(), err);
                            processed_count.fetch_add(1, Ordering::Relaxed);
                            None
                        }
                    }
                })
                .collect()
        });

        Ok(results)
    }

    fn process_single_file(&self, walk_result: &WalkResult) -> Result<Option<ProcessingResult>> {
        let path = &walk_result.path;
        
        // Get file metadata
        let metadata = match fs::metadata(path) {
            Ok(md) => md,
            Err(err) => {
                // Skip files we can't read metadata for
                if std::env::var("FFIND_VERBOSE").is_ok() {
                    eprintln!("Warning: Cannot read metadata for {}: {}", path.display(), err);
                }
                return Ok(None);
            }
        };

        // Apply pattern matching filters
        let matches = self.pattern_matcher.matches(path, &metadata)?;

        if matches {
            let file_info = FileInfo {
                path: path.to_string_lossy().to_string(),
                file_type: if metadata.is_dir() {
                    "directory".to_string()
                } else if metadata.file_type().is_symlink() {
                    "symlink".to_string()
                } else {
                    "file".to_string()
                },
                size: if metadata.is_file() { Some(metadata.len()) } else { None },
                modified: metadata.modified()
                    .ok()
                    .and_then(|time| format_time_iso(time).ok()),
                permissions: Some(format_permissions(&metadata)),
                depth: walk_result.depth,
            };

            Ok(Some(ProcessingResult {
                file_info,
                matches: true,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_stats(&self, processing_time: std::time::Duration) -> ProcessingStats {
        let total_processed = self.processed_count.load(Ordering::Relaxed);
        let total_matched = self.matched_count.load(Ordering::Relaxed);
        let processing_time_ms = processing_time.as_millis() as u64;
        
        let throughput_per_second = if processing_time_ms > 0 {
            (total_processed as f64) / (processing_time_ms as f64 / 1000.0)
        } else {
            0.0
        };

        ProcessingStats {
            total_processed,
            total_matched,
            processing_time_ms,
            throughput_per_second,
        }
    }
}

// Utility functions (duplicated from output.rs to avoid circular dependency)
fn format_permissions(metadata: &fs::Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        
        let mut perms = String::with_capacity(10);
        
        // File type
        if metadata.is_dir() {
            perms.push('d');
        } else if metadata.file_type().is_symlink() {
            perms.push('l');
        } else {
            perms.push('-');
        }
        
        // Owner permissions
        perms.push(if mode & 0o400 != 0 { 'r' } else { '-' });
        perms.push(if mode & 0o200 != 0 { 'w' } else { '-' });
        perms.push(if mode & 0o100 != 0 { 'x' } else { '-' });
        
        // Group permissions
        perms.push(if mode & 0o040 != 0 { 'r' } else { '-' });
        perms.push(if mode & 0o020 != 0 { 'w' } else { '-' });
        perms.push(if mode & 0o010 != 0 { 'x' } else { '-' });
        
        // Other permissions
        perms.push(if mode & 0o004 != 0 { 'r' } else { '-' });
        perms.push(if mode & 0o002 != 0 { 'w' } else { '-' });
        perms.push(if mode & 0o001 != 0 { 'x' } else { '-' });
        
        perms
    }
    
    #[cfg(not(unix))]
    {
        let readonly = metadata.permissions().readonly();
        if metadata.is_dir() {
            if readonly { "dr-xr-xr-x".to_string() } else { "drwxrwxrwx".to_string() }
        } else {
            if readonly { "-r--r--r--".to_string() } else { "-rw-rw-rw-".to_string() }
        }
    }
}

fn format_time_iso(time: std::time::SystemTime) -> Result<String> {
    let duration = time.duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("Time conversion error: {}", e))?;

    let secs = duration.as_secs();
    let dt = chrono::DateTime::from_timestamp(secs as i64, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid timestamp"))?;

    Ok(dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

// Batch processing utilities for better performance
pub struct BatchProcessor {
    batch_size: usize,
    worker_pool: WorkerPool,
}

impl BatchProcessor {
    pub fn new(pattern_matcher: PatternMatcher, thread_count: usize, batch_size: Option<usize>) -> Self {
        Self {
            batch_size: batch_size.unwrap_or(1000),
            worker_pool: WorkerPool::new(pattern_matcher, thread_count),
        }
    }

    pub fn process_in_batches(&self, walk_results: Vec<WalkResult>) -> Result<Vec<ProcessingResult>> {
        let mut all_results = Vec::new();
        
        // Process in batches to manage memory usage
        for batch in walk_results.chunks(self.batch_size) {
            let batch_results = self.worker_pool.process_files(batch.to_vec())?;
            all_results.extend(batch_results);
        }
        
        Ok(all_results)
    }

    pub fn get_stats(&self, processing_time: std::time::Duration) -> ProcessingStats {
        self.worker_pool.get_stats(processing_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use crate::file_walker::WalkResult;
    use std::path::PathBuf;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_worker_pool_processing() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, "test content").unwrap();

        let args = Args::default();
        let pattern_matcher = PatternMatcher::new(&args).unwrap();
        let worker_pool = WorkerPool::new(pattern_matcher, 2);

        let walk_result = WalkResult {
            path: test_file,
            depth: 1,
            is_dir: false,
            is_symlink: false,
        };

        let results = worker_pool.process_files(vec![walk_result]).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].matches);
    }

    #[test]
    fn test_batch_processor() {
        let temp_dir = TempDir::new().unwrap();
        let test_files: Vec<_> = (0..10).map(|i| {
            let path = temp_dir.path().join(format!("test{}.txt", i));
            let mut file = File::create(&path).unwrap();
            writeln!(file, "content {}", i).unwrap();
            WalkResult {
                path,
                depth: 1,
                is_dir: false,
                is_symlink: false,
            }
        }).collect();

        let args = Args::default();
        let pattern_matcher = PatternMatcher::new(&args).unwrap();
        let batch_processor = BatchProcessor::new(pattern_matcher, 2, Some(5));

        let results = batch_processor.process_in_batches(test_files).unwrap();
        assert_eq!(results.len(), 10);
    }

    #[test]
    fn test_format_permissions() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        File::create(&test_file).unwrap();
        
        let metadata = fs::metadata(&test_file).unwrap();
        let perms = format_permissions(&metadata);
        
        // Should be a 10-character string starting with '-' for regular files
        assert_eq!(perms.len(), 10);
        assert!(perms.starts_with('-'));
    }
}