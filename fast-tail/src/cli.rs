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

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    /// Plain text output
    Text,
    /// JSON output
    Json,
}

#[derive(Parser, Debug)]
#[command(name = "ftail")]
#[command(about = "Ultra-fast real-time file monitoring and log tailing tool")]
#[command(version = "0.1.0")]
pub struct Args {
    /// Files to monitor
    #[arg(value_name = "FILE", required = true)]
    pub files: Vec<PathBuf>,

    /// Number of lines to show initially from the end of each file
    #[arg(short = 'n', long = "lines", default_value = "10")]
    pub initial_lines: usize,

    /// Follow file changes in real-time (like tail -f)
    #[arg(short = 'f', long = "follow")]
    pub follow: bool,

    /// Follow file by name (handles log rotation)
    #[arg(short = 'F', long = "follow-name")]
    pub follow_name: bool,

    /// Pattern to filter lines (grep-style)
    #[arg(short = 'g', long = "grep")]
    pub pattern: Option<String>,

    /// Use regular expressions for pattern matching
    #[arg(short = 'E', long = "regex")]
    pub use_regex: bool,

    /// Case insensitive pattern matching
    #[arg(short = 'i', long = "ignore-case")]
    pub ignore_case: bool,

    /// Invert pattern matching (show non-matching lines)
    #[arg(short = 'v', long = "invert-match")]
    pub invert_match: bool,

    /// Show line numbers
    #[arg(short = 'N', long = "line-number")]
    pub line_numbers: bool,

    /// Suppress filename headers
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,

    /// Control colored output
    #[arg(long = "color", value_enum, default_value = "auto")]
    pub color: ColorOption,

    /// Output format
    #[arg(long = "format", value_enum, default_value = "text")]
    pub format: OutputFormat,

    /// Add timestamps to output lines
    #[arg(short = 't', long = "timestamp")]
    pub timestamp: bool,

    /// Buffer size for reading files (in KB)
    #[arg(long = "buffer-size", default_value = "64")]
    pub buffer_size_kb: usize,

    /// Polling interval in milliseconds (fallback when inotify fails)
    #[arg(long = "poll-interval", default_value = "100")]
    pub poll_interval_ms: u64,

    /// Maximum number of lines to buffer in memory
    #[arg(long = "max-buffer-lines", default_value = "10000")]
    pub max_buffer_lines: usize,

    /// Print verbose debugging information
    #[arg(short = 'V', long = "verbose")]
    pub verbose: bool,
}

impl Args {
    pub fn should_use_colors(&self) -> bool {
        match self.color {
            ColorOption::Always => true,
            ColorOption::Never => false,
            ColorOption::Auto => atty::is(atty::Stream::Stdout),
        }
    }

    pub fn buffer_size_bytes(&self) -> usize {
        self.buffer_size_kb * 1024
    }

    pub fn has_pattern(&self) -> bool {
        self.pattern.is_some()
    }

    pub fn get_pattern(&self) -> Option<&str> {
        self.pattern.as_deref()
    }

    pub fn is_json_output(&self) -> bool {
        matches!(self.format, OutputFormat::Json)
    }

    pub fn is_text_output(&self) -> bool {
        matches!(self.format, OutputFormat::Text)
    }

    pub fn should_show_filenames(&self) -> bool {
        !self.quiet && self.files.len() > 1
    }
}