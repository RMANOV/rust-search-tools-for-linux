use anyhow::Result;
use ignore::{WalkBuilder, WalkState};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::cli::Args;

pub struct FileWalker {
    args: Args,
    files_visited: Arc<AtomicUsize>,
    dirs_visited: Arc<AtomicUsize>,
}

#[derive(Debug, Clone)]
pub struct WalkResult {
    pub path: PathBuf,
    pub depth: usize,
    pub is_dir: bool,
    pub is_symlink: bool,
}

impl FileWalker {
    pub fn new(args: Args) -> Self {
        Self {
            args,
            files_visited: Arc::new(AtomicUsize::new(0)),
            dirs_visited: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn walk(&self) -> Result<Vec<WalkResult>> {
        let paths = self.args.get_paths();
        
        // Collect all entries in parallel
        let all_results: Vec<WalkResult> = paths
            .par_iter()
            .map(|path| self.walk_path(path))
            .collect::<Result<Vec<Vec<WalkResult>>>>()?
            .into_iter()
            .flatten()
            .collect();

        // Sort results if requested
        let mut results = all_results;
        if self.args.sort_results {
            results.sort_by(|a, b| {
                let cmp = a.path.cmp(&b.path);
                if self.args.reverse_sort {
                    cmp.reverse()
                } else {
                    cmp
                }
            });
        }

        Ok(results)
    }

    fn walk_path(&self, root_path: &Path) -> Result<Vec<WalkResult>> {
        let mut results = Vec::new();
        
        // Handle single file case
        if root_path.is_file() {
            let metadata = std::fs::metadata(root_path)?;
            let is_symlink = metadata.file_type().is_symlink();
            
            results.push(WalkResult {
                path: root_path.to_path_buf(),
                depth: 0,
                is_dir: false,
                is_symlink,
            });
            
            self.files_visited.fetch_add(1, Ordering::Relaxed);
            return Ok(results);
        }

        // Configure directory walker
        let mut builder = WalkBuilder::new(root_path);
        
        // Basic traversal options
        builder
            .hidden(!self.args.search_hidden)
            .ignore(self.args.respect_ignore)
            .git_ignore(self.args.respect_ignore)
            .follow_links(self.args.follow_symlinks)
            .same_file_system(!self.args.cross_filesystem)
            .threads(self.args.get_threads());

        // Depth limits
        if let Some(max_depth) = self.args.max_depth {
            builder.max_depth(Some(max_depth));
        }

        // Use parallel walking for better performance
        let walker = builder.build_parallel();
        
        // Thread-safe result collection
        let results_mutex = std::sync::Mutex::new(Vec::new());
        let files_visited = Arc::clone(&self.files_visited);
        let dirs_visited = Arc::clone(&self.dirs_visited);
        let min_depth = self.args.min_depth.unwrap_or(0);

        walker.run(|| {
            let results_mutex = &results_mutex;
            let files_visited = Arc::clone(&files_visited);
            let dirs_visited = Arc::clone(&dirs_visited);
            
            Box::new(move |entry_result| {
                match entry_result {
                    Ok(entry) => {
                        let depth = entry.depth();
                        
                        // Skip if below minimum depth
                        if depth < min_depth {
                            return WalkState::Continue;
                        }

                        let path = entry.path();
                        let file_type = entry.file_type();
                        
                        if let Some(file_type) = file_type {
                            let is_dir = file_type.is_dir();
                            let is_symlink = file_type.is_symlink();
                            
                            // Update counters
                            if is_dir {
                                dirs_visited.fetch_add(1, Ordering::Relaxed);
                            } else {
                                files_visited.fetch_add(1, Ordering::Relaxed);
                            }
                            
                            // Collect the result
                            let walk_result = WalkResult {
                                path: path.to_path_buf(),
                                depth,
                                is_dir,
                                is_symlink,
                            };
                            
                            if let Ok(mut results) = results_mutex.lock() {
                                results.push(walk_result);
                            }
                        }
                        
                        WalkState::Continue
                    }
                    Err(err) => {
                        eprintln!("Warning: {}", err);
                        WalkState::Continue
                    }
                }
            })
        });

        let results = results_mutex.into_inner().unwrap_or_default();
        Ok(results)
    }

    pub fn get_stats(&self) -> WalkStats {
        WalkStats {
            files_visited: self.files_visited.load(Ordering::Relaxed),
            dirs_visited: self.dirs_visited.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WalkStats {
    pub files_visited: usize,
    pub dirs_visited: usize,
}

impl WalkStats {
    pub fn total_entries(&self) -> usize {
        self.files_visited + self.dirs_visited
    }
}

// Helper function to check if path matches depth constraints
pub fn check_depth_constraints(path: &Path, root: &Path, min_depth: Option<usize>, max_depth: Option<usize>) -> bool {
    let depth = path.strip_prefix(root)
        .map(|p| p.components().count())
        .unwrap_or(0);
    
    if let Some(min) = min_depth {
        if depth < min {
            return false;
        }
    }
    
    if let Some(max) = max_depth {
        if depth > max {
            return false;
        }
    }
    
    true
}

// Helper function to determine if a path should be followed
pub fn should_follow_symlink(path: &Path, follow_symlinks: bool) -> bool {
    if !follow_symlinks {
        return false;
    }

    // Reject paths that contain parent directory references (potential escape)
    let path_str = path.to_string_lossy();
    if path_str.contains("..") {
        return false;
    }

    // Additional safety checks to prevent infinite loops
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            // Check if this symlink would create a cycle
            if let Ok(target) = std::fs::read_link(path) {
                // Simple heuristic: don't follow symlinks that point to parent directories
                if target.to_string_lossy().contains("..") {
                    return false;
                }

                // Don't follow symlinks that point to absolute paths above the current directory
                if target.is_absolute() {
                    if let Ok(current_dir) = std::env::current_dir() {
                        if !target.starts_with(&current_dir) {
                            return false;
                        }
                    }
                }
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_walk_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let args = Args {
            paths: vec![test_file.clone()],
            ..Args::default()
        };

        let walker = FileWalker::new(args);
        let results = walker.walk().unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, test_file);
        assert!(!results[0].is_dir);
    }

    #[test]
    fn test_walk_directory() {
        let temp_dir = TempDir::new().unwrap();
        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir).unwrap();
        
        let test_file1 = temp_dir.path().join("file1.txt");
        let test_file2 = sub_dir.join("file2.txt");
        fs::write(&test_file1, "content1").unwrap();
        fs::write(&test_file2, "content2").unwrap();

        let args = Args {
            paths: vec![temp_dir.path().to_path_buf()],
            ..Args::default()
        };

        let walker = FileWalker::new(args);
        let results = walker.walk().unwrap();

        assert!(results.len() >= 3); // root dir + subdir + at least one file
        
        let paths: Vec<_> = results.iter().map(|r| &r.path).collect();
        assert!(paths.contains(&&test_file1));
        assert!(paths.contains(&&test_file2));
    }

    #[test]
    fn test_depth_constraints() {
        let root = Path::new("/root");
        let path1 = Path::new("/root/file.txt");
        let path2 = Path::new("/root/sub/file.txt");
        let path3 = Path::new("/root/sub/deep/file.txt");

        assert!(check_depth_constraints(path1, root, Some(1), None));
        assert!(!check_depth_constraints(path1, root, Some(2), None));
        
        assert!(check_depth_constraints(path2, root, None, Some(2)));
        assert!(!check_depth_constraints(path3, root, None, Some(2)));
    }

    #[test]
    fn test_symlink_following() {
        assert!(!should_follow_symlink(Path::new("../parent"), true));
        assert!(should_follow_symlink(Path::new("regular_file"), false) == false);
    }
}