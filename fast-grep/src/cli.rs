use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, ValueEnum)]
pub enum ColorOption {
    /// Auto-detect color support
    Auto,
    /// Always use colors
    Always,
    /// Never use colors
    Never,
}

#[derive(Parser, Debug)]
#[command(name = "fgrep")]
#[command(about = "Ultra-fast parallel text search tool")]
#[command(version = "0.1.0")]
pub struct Args {
    /// Pattern to search for
    #[arg(value_name = "PATTERN")]
    pub pattern: String,

    /// Files or directories to search
    #[arg(value_name = "PATH", default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Use regular expressions (default: literal string search)
    #[arg(short = 'E', long = "regex")]
    pub use_regex: bool,

    /// Fixed string search (literal matching, no regex)
    #[arg(short = 'F', long = "fixed-strings")]
    pub fixed_strings: bool,

    /// Case insensitive search
    #[arg(short = 'i', long = "ignore-case")]
    pub ignore_case: bool,

    /// Show line numbers
    #[arg(short = 'n', long = "line-number")]
    pub line_numbers: bool,

    /// Show only file names with matches
    #[arg(short = 'l', long = "files-with-matches")]
    pub files_only: bool,

    /// Show count of matching lines per file
    #[arg(short = 'c', long = "count")]
    pub count_only: bool,

    /// Invert match (show non-matching lines)
    #[arg(short = 'v', long = "invert-match")]
    pub invert_match: bool,

    /// Show only the matching part of lines
    #[arg(short = 'o', long = "only-matching")]
    pub only_matching: bool,

    /// Show only names of files without matches
    #[arg(short = 'L', long = "files-without-match")]
    pub files_without_matches: bool,

    /// Suppress filename prefix in output
    #[arg(short = 'h', long = "no-filename")]
    pub no_filename: bool,

    /// Recursively search directories
    #[arg(short = 'r', long = "recursive", default_value_t = true)]
    pub recursive: bool,

    /// Show context lines before match
    #[arg(short = 'B', long = "before-context", value_name = "N")]
    pub before_context: Option<usize>,

    /// Show context lines after match
    #[arg(short = 'A', long = "after-context", value_name = "N")]
    pub after_context: Option<usize>,

    /// Show context lines before and after match
    #[arg(short = 'C', long = "context", value_name = "N")]
    pub context: Option<usize>,

    /// Number of worker threads (default: CPU cores)
    #[arg(short = 'j', long = "threads")]
    pub threads: Option<usize>,

    /// File types to include (e.g., "rs,py,js")
    #[arg(long = "type")]
    pub file_types: Option<String>,

    /// File types to exclude
    #[arg(long = "type-not")]
    pub exclude_types: Option<String>,

    /// Respect .gitignore files
    #[arg(long = "no-ignore", action = clap::ArgAction::SetFalse)]
    pub respect_ignore: bool,

    /// Search hidden files
    #[arg(long = "hidden")]
    pub search_hidden: bool,

    /// Control colored output
    #[arg(long = "color", value_enum, default_value = "auto")]
    pub color: ColorOption,

    /// Disable colored output (deprecated, use --color=never)
    #[arg(long = "no-color")]
    pub no_color: bool,

    /// Output in JSON format
    #[arg(long = "json")]
    pub json_output: bool,

    /// Maximum file size to search (in MB)
    #[arg(long = "max-filesize", default_value_t = 100)]
    pub max_filesize_mb: u64,

    /// Use memory mapping for large files
    #[arg(long = "mmap", default_value_t = true)]
    pub use_mmap: bool,
}

impl Args {
    pub fn get_before_context(&self) -> usize {
        self.context.or(self.before_context).unwrap_or(0)
    }

    pub fn get_after_context(&self) -> usize {
        self.context.or(self.after_context).unwrap_or(0)
    }

    pub fn get_threads(&self) -> usize {
        self.threads.unwrap_or_else(num_cpus::get)
    }

    pub fn max_filesize_bytes(&self) -> u64 {
        self.max_filesize_mb * 1024 * 1024
    }

    pub fn should_use_colors(&self) -> bool {
        if self.no_color {
            return false;
        }
        match self.color {
            ColorOption::Always => true,
            ColorOption::Never => false,
            ColorOption::Auto => atty::is(atty::Stream::Stdout),
        }
    }

    pub fn is_literal_search(&self) -> bool {
        self.fixed_strings || (!self.use_regex && !self.pattern_looks_like_regex())
    }

    fn pattern_looks_like_regex(&self) -> bool {
        // Simple heuristic to detect if pattern contains regex metacharacters
        self.pattern.chars().any(|c| matches!(c, '.' | '*' | '+' | '?' | '^' | '$' | '|' | '[' | ']' | '(' | ')' | '{' | '}'))
    }
}