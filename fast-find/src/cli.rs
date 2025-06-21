use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "ffind")]
#[command(about = "Ultra-fast parallel file finder - modern find alternative")]
#[command(version = "0.1.0")]
pub struct Args {
    /// Paths to search (default: current directory)
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    // Name/Path Pattern Matching
    /// Base of file name matches shell pattern (case sensitive)
    #[arg(short = 'n', long = "name")]
    pub name: Option<String>,

    /// Base of file name matches shell pattern (case insensitive)
    #[arg(long = "iname")]
    pub iname: Option<String>,

    /// File path matches shell pattern (case sensitive)
    #[arg(long = "path")]
    pub path: Option<String>,

    /// File path matches shell pattern (case insensitive)
    #[arg(long = "ipath")]
    pub ipath: Option<String>,

    /// Use regular expressions instead of shell patterns
    #[arg(short = 'E', long = "regex")]
    pub use_regex: bool,

    // File Type Filters
    /// File type (f=file, d=directory, l=symlink)
    #[arg(short = 't', long = "type")]
    pub file_type: Option<String>,

    /// File extensions to include (e.g., "rs,py,js")
    #[arg(long = "ext")]
    pub extensions: Option<String>,

    /// File extensions to exclude
    #[arg(long = "not-ext")]
    pub exclude_extensions: Option<String>,

    // Size Filters
    /// File size (e.g., "+100k", "-1M", "=50G")
    #[arg(short = 's', long = "size")]
    pub size: Option<String>,

    /// Empty files and directories
    #[arg(long = "empty")]
    pub empty: bool,

    // Time Filters
    /// Modified time in days (e.g., "+7", "-1", "=0")
    #[arg(long = "mtime")]
    pub mtime: Option<String>,

    /// Access time in days
    #[arg(long = "atime")]
    pub atime: Option<String>,

    /// Status change time in days
    #[arg(long = "ctime")]
    pub ctime: Option<String>,

    /// Files newer than reference file
    #[arg(long = "newer")]
    pub newer: Option<PathBuf>,

    // Depth Control
    /// Maximum search depth
    #[arg(long = "max-depth")]
    pub max_depth: Option<usize>,

    /// Minimum search depth
    #[arg(long = "min-depth")]
    pub min_depth: Option<usize>,

    // Traversal Options
    /// Follow symbolic links
    #[arg(short = 'L', long = "follow")]
    pub follow_symlinks: bool,

    /// Search hidden files and directories
    #[arg(short = 'H', long = "hidden")]
    pub search_hidden: bool,

    /// Respect .gitignore files
    #[arg(long = "no-ignore", action = clap::ArgAction::SetFalse)]
    pub respect_ignore: bool,

    /// Cross filesystem boundaries
    #[arg(long = "mount")]
    pub cross_filesystem: bool,

    // Performance Options
    /// Number of worker threads (default: CPU cores)
    #[arg(short = 'j', long = "threads")]
    pub threads: Option<usize>,

    /// Maximum number of open file descriptors
    #[arg(long = "max-open")]
    pub max_open: Option<usize>,

    // Output Options
    /// Print results separated by null characters
    #[arg(short = '0', long = "print0")]
    pub print0: bool,

    /// Output in JSON format
    #[arg(long = "json")]
    pub json_output: bool,

    /// Disable colored output
    #[arg(long = "no-color")]
    pub no_color: bool,

    /// Show file details (size, mtime, permissions)
    #[arg(short = 'l', long = "long")]
    pub long_format: bool,

    /// Count matching files only
    #[arg(short = 'c', long = "count")]
    pub count_only: bool,

    /// Show statistics after search
    #[arg(long = "stats")]
    pub show_stats: bool,

    // Actions (simplified - no exec/delete for safety)
    /// Print matching files (default action)
    #[arg(long = "print")]
    pub print: bool,

    /// Sort results by name
    #[arg(long = "sort")]
    pub sort_results: bool,

    /// Reverse sort order
    #[arg(short = 'r', long = "reverse")]
    pub reverse_sort: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            paths: vec![PathBuf::from(".")],
            name: None,
            iname: None,
            path: None,
            ipath: None,
            use_regex: false,
            file_type: None,
            extensions: None,
            exclude_extensions: None,
            size: None,
            empty: false,
            mtime: None,
            atime: None,
            ctime: None,
            newer: None,
            max_depth: None,
            min_depth: None,
            follow_symlinks: false,
            search_hidden: false,
            respect_ignore: true,
            cross_filesystem: false,
            threads: None,
            max_open: None,
            print0: false,
            json_output: false,
            no_color: false,
            long_format: false,
            count_only: false,
            show_stats: false,
            print: false,
            sort_results: false,
            reverse_sort: false,
        }
    }
}

impl Args {
    pub fn get_threads(&self) -> usize {
        self.threads.unwrap_or_else(num_cpus::get)
    }

    pub fn get_max_open(&self) -> usize {
        self.max_open.unwrap_or(1024)
    }

    pub fn get_paths(&self) -> Vec<PathBuf> {
        if self.paths.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            self.paths.clone()
        }
    }

    pub fn has_pattern_filters(&self) -> bool {
        self.name.is_some() 
            || self.iname.is_some() 
            || self.path.is_some() 
            || self.ipath.is_some()
    }

    pub fn has_size_filters(&self) -> bool {
        self.size.is_some() || self.empty
    }

    pub fn has_time_filters(&self) -> bool {
        self.mtime.is_some() 
            || self.atime.is_some() 
            || self.ctime.is_some() 
            || self.newer.is_some()
    }

    pub fn validate(&self) -> Result<(), String> {
        // Validate file type
        if let Some(ref t) = self.file_type {
            if !matches!(t.as_str(), "f" | "d" | "l" | "file" | "dir" | "directory" | "symlink") {
                return Err(format!("Invalid file type: '{}'. Use f/file, d/dir/directory, or l/symlink", t));
            }
        }

        // Validate size format
        if let Some(ref s) = self.size {
            if !is_valid_size_spec(s) {
                return Err(format!("Invalid size specification: '{}'. Use format like '+100k', '-1M', '=50G'", s));
            }
        }

        // Validate time format
        for (field, value) in [("mtime", &self.mtime), ("atime", &self.atime), ("ctime", &self.ctime)] {
            if let Some(ref t) = value {
                if !is_valid_time_spec(t) {
                    return Err(format!("Invalid {} specification: '{}'. Use format like '+7', '-1', '=0'", field, t));
                }
            }
        }

        // Validate depth
        if let (Some(min), Some(max)) = (self.min_depth, self.max_depth) {
            if min > max {
                return Err("min-depth cannot be greater than max-depth".to_string());
            }
        }

        Ok(())
    }
}

fn is_valid_size_spec(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    
    let (prefix, rest) = if s.starts_with(['+', '-', '=']) {
        (&s[0..1], &s[1..])
    } else {
        ("", s)
    };
    
    if rest.is_empty() {
        return false;
    }
    
    let (number_part, suffix) = if let Some(pos) = rest.find(|c: char| c.is_alphabetic()) {
        (&rest[..pos], &rest[pos..])
    } else {
        (rest, "")
    };
    
    if number_part.parse::<u64>().is_err() {
        return false;
    }
    
    matches!(suffix, "" | "c" | "b" | "k" | "M" | "G" | "T")
}

fn is_valid_time_spec(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    
    let rest = if s.starts_with(['+', '-', '=']) {
        &s[1..]
    } else {
        s
    };
    
    rest.parse::<u32>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_validation() {
        assert!(is_valid_size_spec("100"));
        assert!(is_valid_size_spec("+100k"));
        assert!(is_valid_size_spec("-50M"));
        assert!(is_valid_size_spec("=1G"));
        assert!(!is_valid_size_spec(""));
        assert!(!is_valid_size_spec("+"));
        assert!(!is_valid_size_spec("abc"));
        assert!(!is_valid_size_spec("100x"));
    }

    #[test]
    fn test_time_validation() {
        assert!(is_valid_time_spec("7"));
        assert!(is_valid_time_spec("+7"));
        assert!(is_valid_time_spec("-1"));
        assert!(is_valid_time_spec("=0"));
        assert!(!is_valid_time_spec(""));
        assert!(!is_valid_time_spec("+"));
        assert!(!is_valid_time_spec("abc"));
    }
}