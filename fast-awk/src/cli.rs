use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, ValueEnum)]
pub enum ColorOption {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Csv,
}

#[derive(Parser, Debug)]
#[command(
    name = "fawk",
    about = "Ultra-fast AWK-compatible text processor with SIMD optimization",
    version = "0.1.0",
    author = "Rust Search Tools Team"
)]
pub struct Args {
    /// AWK script to execute
    #[arg(value_name = "SCRIPT", help = "AWK script or pattern-action program")]
    pub script: String,

    /// Input files to process
    #[arg(value_name = "FILES", help = "Input files (default: stdin)")]
    pub files: Vec<PathBuf>,

    /// Field separator (default: auto-detect)
    #[arg(short = 'F', long = "field-separator", value_name = "FS")]
    pub field_separator: Option<String>,

    /// Output field separator (default: space)
    #[arg(long = "output-separator", value_name = "OFS")]
    pub output_separator: Option<String>,

    /// Record separator (default: newline)
    #[arg(long = "record-separator", value_name = "RS")]
    pub record_separator: Option<String>,

    /// Output record separator (default: newline)
    #[arg(long = "output-record-separator", value_name = "ORS")]
    pub output_record_separator: Option<String>,

    /// Set variable assignments (e.g., -v var=value)
    #[arg(short = 'v', long = "assign", action = clap::ArgAction::Append)]
    pub variables: Vec<String>,

    /// Execute script from file
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub script_file: Option<PathBuf>,

    /// Print program (useful for debugging)
    #[arg(short = 'p', long = "print-program")]
    pub print_program: bool,

    /// Output format
    #[arg(long = "format", default_value = "text")]
    pub format: OutputFormat,

    /// Enable colored output
    #[arg(long = "color", default_value = "auto")]
    pub color: ColorOption,

    /// Number of parallel threads
    #[arg(short = 'j', long = "threads", value_name = "N")]
    pub threads: Option<usize>,

    /// Buffer size in KB for I/O operations
    #[arg(long = "buffer-size", default_value = "64", value_name = "KB")]
    pub buffer_size_kb: usize,

    /// Enable verbose output
    #[arg(long = "verbose")]
    pub verbose: bool,

    /// Quiet mode (suppress warnings)
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,

    /// Case-insensitive pattern matching
    #[arg(short = 'i', long = "ignore-case")]
    pub ignore_case: bool,

    /// Only process first N records
    #[arg(long = "max-records", value_name = "N")]
    pub max_records: Option<usize>,

    /// Skip first N records
    #[arg(long = "skip-records", value_name = "N")]
    pub skip_records: Option<usize>,

    /// Enable POSIX compatibility mode
    #[arg(long = "posix")]
    pub posix_mode: bool,

    /// Enable traditional AWK mode (disable extensions)
    #[arg(long = "traditional")]
    pub traditional_mode: bool,
}

impl Args {
    pub fn get_field_separator(&self) -> Option<&str> {
        self.field_separator.as_deref()
    }

    pub fn get_output_separator(&self) -> &str {
        self.output_separator.as_deref().unwrap_or(" ")
    }

    pub fn get_record_separator(&self) -> &str {
        self.record_separator.as_deref().unwrap_or("\n")
    }

    pub fn get_output_record_separator(&self) -> &str {
        self.output_record_separator.as_deref().unwrap_or("\n")
    }

    pub fn should_use_colors(&self) -> bool {
        match self.color {
            ColorOption::Always => true,
            ColorOption::Never => false,
            ColorOption::Auto => atty::is(atty::Stream::Stdout),
        }
    }

    pub fn get_threads(&self) -> usize {
        self.threads.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        })
    }

    pub fn buffer_size_bytes(&self) -> usize {
        self.buffer_size_kb * 1024
    }

    pub fn parse_variable_assignments(&self) -> Result<Vec<(String, String)>, String> {
        let mut assignments = Vec::new();
        
        for var_assignment in &self.variables {
            if let Some((name, value)) = var_assignment.split_once('=') {
                assignments.push((name.to_string(), value.to_string()));
            } else {
                return Err(format!("Invalid variable assignment: {}", var_assignment));
            }
        }
        
        Ok(assignments)
    }

    pub fn get_script(&self) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(ref script_file) = self.script_file {
            std::fs::read_to_string(script_file)
                .map_err(|e| format!("Failed to read script file '{}': {}", script_file.display(), e).into())
        } else {
            Ok(self.script.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_parsing() {
        let args = Args {
            script: "test".to_string(),
            files: vec![],
            field_separator: None,
            output_separator: None,
            record_separator: None,
            output_record_separator: None,
            variables: vec!["name=value".to_string(), "count=42".to_string()],
            script_file: None,
            print_program: false,
            format: OutputFormat::Text,
            color: ColorOption::Auto,
            threads: None,
            buffer_size_kb: 64,
            verbose: false,
            quiet: false,
            ignore_case: false,
            max_records: None,
            skip_records: None,
            posix_mode: false,
            traditional_mode: false,
        };

        let assignments = args.parse_variable_assignments().unwrap();
        assert_eq!(assignments.len(), 2);
        assert_eq!(assignments[0], ("name".to_string(), "value".to_string()));
        assert_eq!(assignments[1], ("count".to_string(), "42".to_string()));
    }

    #[test]
    fn test_default_separators() {
        let args = Args {
            script: "test".to_string(),
            files: vec![],
            field_separator: None,
            output_separator: None,
            record_separator: None,
            output_record_separator: None,
            variables: vec![],
            script_file: None,
            print_program: false,
            format: OutputFormat::Text,
            color: ColorOption::Auto,
            threads: None,
            buffer_size_kb: 64,
            verbose: false,
            quiet: false,
            ignore_case: false,
            max_records: None,
            skip_records: None,
            posix_mode: false,
            traditional_mode: false,
        };

        assert_eq!(args.get_output_separator(), " ");
        assert_eq!(args.get_record_separator(), "\n");
        assert_eq!(args.get_output_record_separator(), "\n");
    }
}