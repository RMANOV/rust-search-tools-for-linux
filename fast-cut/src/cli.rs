use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    /// Plain text output (default)
    Text,
    /// CSV output with proper quoting
    Csv,
    /// JSON output
    Json,
}

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
#[command(name = "fcut")]
#[command(about = "Ultra-fast field extraction tool for delimited data and logs")]
#[command(version = "0.1.0")]
pub struct Args {
    /// Input files (stdin if not specified)
    #[arg(value_name = "FILE")]
    pub files: Vec<PathBuf>,

    /// Fields to extract (e.g., "1,3,5-7" or "name,age,city")
    #[arg(short = 'f', long = "fields", value_name = "LIST")]
    pub fields: String,

    /// Input field delimiter (auto-detect if not specified)
    #[arg(short = 'd', long = "delimiter", value_name = "DELIM")]
    pub delimiter: Option<String>,

    /// Use tab as delimiter
    #[arg(short = 't', long = "tab")]
    pub tab_delimiter: bool,

    /// Use space as delimiter (collapse multiple spaces)
    #[arg(short = 's', long = "space")]
    pub space_delimiter: bool,

    /// Enable CSV mode with quote handling
    #[arg(short = 'c', long = "csv")]
    pub csv_mode: bool,

    /// Output field delimiter (default: same as input)
    #[arg(short = 'o', long = "output-delimiter", value_name = "DELIM")]
    pub output_delimiter: Option<String>,

    /// Output format
    #[arg(long = "format", value_enum, default_value = "text")]
    pub format: OutputFormat,

    /// First line contains field headers
    #[arg(long = "header")]
    pub has_header: bool,

    /// Skip header line (don't output it)
    #[arg(long = "no-header")]
    pub skip_header: bool,

    /// Add line numbers to output
    #[arg(short = 'n', long = "line-numbers")]
    pub line_numbers: bool,

    /// Use null character as line separator
    #[arg(short = 'z', long = "zero-terminated")]
    pub zero_terminated: bool,

    /// Skip N lines from start
    #[arg(long = "skip-lines", value_name = "N", default_value = "0")]
    pub skip_lines: usize,

    /// Process only N lines (0 = unlimited)
    #[arg(long = "max-lines", value_name = "N", default_value = "0")]
    pub max_lines: usize,

    /// Control colored output
    #[arg(long = "color", value_enum, default_value = "auto")]
    pub color: ColorOption,

    /// Number of worker threads (default: CPU cores)
    #[arg(short = 'j', long = "threads")]
    pub threads: Option<usize>,

    /// Buffer size for I/O operations (in KB)
    #[arg(long = "buffer-size", default_value = "64")]
    pub buffer_size_kb: usize,

    /// Only output non-empty lines
    #[arg(long = "non-empty")]
    pub non_empty_only: bool,

    /// Print verbose debugging information
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
}

#[derive(Debug, Clone)]
pub struct FieldSelector {
    pub indices: Vec<usize>,
    pub ranges: Vec<(usize, usize)>,
    pub names: Vec<String>,
}

impl Args {
    pub fn should_use_colors(&self) -> bool {
        match self.color {
            ColorOption::Always => true,
            ColorOption::Never => false,
            ColorOption::Auto => atty::is(atty::Stream::Stdout),
        }
    }

    pub fn get_threads(&self) -> usize {
        self.threads.unwrap_or_else(num_cpus::get)
    }

    pub fn buffer_size_bytes(&self) -> usize {
        self.buffer_size_kb * 1024
    }

    pub fn get_input_delimiter(&self) -> Option<String> {
        if self.tab_delimiter {
            Some("\t".to_string())
        } else if self.space_delimiter {
            Some(" ".to_string())
        } else {
            self.delimiter.clone()
        }
    }

    pub fn get_output_delimiter(&self) -> Option<String> {
        self.output_delimiter.clone().or_else(|| self.get_input_delimiter())
    }

    pub fn is_csv_mode(&self) -> bool {
        self.csv_mode
    }

    pub fn is_json_output(&self) -> bool {
        matches!(self.format, OutputFormat::Json)
    }

    pub fn is_csv_output(&self) -> bool {
        matches!(self.format, OutputFormat::Csv)
    }

    pub fn parse_field_selector(&self) -> Result<FieldSelector, String> {
        let mut indices = Vec::new();
        let mut ranges = Vec::new();
        let mut names = Vec::new();

        for part in self.fields.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            // Check if it's a range (e.g., "5-7")
            if let Some(dash_pos) = part.find('-') {
                if dash_pos > 0 && dash_pos < part.len() - 1 {
                    let start_str = &part[..dash_pos];
                    let end_str = &part[dash_pos + 1..];
                    
                    if let (Ok(start), Ok(end)) = (start_str.parse::<usize>(), end_str.parse::<usize>()) {
                        if start == 0 || end == 0 {
                            return Err("Field indices must be >= 1".to_string());
                        }
                        if start > end {
                            return Err(format!("Invalid range: {}-{} (start > end)", start, end));
                        }
                        ranges.push((start - 1, end - 1)); // Convert to 0-based
                        continue;
                    }
                }
            }

            // Check if it's a numeric index
            if let Ok(index) = part.parse::<usize>() {
                if index == 0 {
                    return Err("Field indices must be >= 1".to_string());
                }
                indices.push(index - 1); // Convert to 0-based
            } else {
                // Treat as field name
                names.push(part.to_string());
            }
        }

        if indices.is_empty() && ranges.is_empty() && names.is_empty() {
            return Err("No valid fields specified".to_string());
        }

        Ok(FieldSelector {
            indices,
            ranges,
            names,
        })
    }

    pub fn should_process_line(&self, line_number: usize) -> bool {
        if line_number < self.skip_lines {
            return false;
        }

        if self.max_lines > 0 && line_number >= self.skip_lines + self.max_lines {
            return false;
        }

        true
    }
}