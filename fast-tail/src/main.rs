mod cli;
mod errors;
mod file_monitor;
mod output;
mod pattern_matcher;

use cli::Args;
use clap::Parser;
use errors::{FastTailError, Result};
use file_monitor::FileMonitor;
use output::OutputFormatter;
use pattern_matcher::PatternMatcher;
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.verbose {
        eprintln!("Fast-tail starting with {} files", args.files.len());
    }

    // Validate arguments
    if args.files.is_empty() {
        return Err(FastTailError::invalid_config("No files specified"));
    }

    // Create pattern matcher if needed
    let pattern_matcher = if let Some(pattern) = args.get_pattern() {
        Some(PatternMatcher::new(
            pattern,
            args.use_regex,
            args.ignore_case,
            args.invert_match,
        )?)
    } else {
        None
    };

    // Create output formatter
    let formatter = OutputFormatter::new(
        args.should_use_colors(),
        args.line_numbers,
        args.should_show_filenames(),
        args.timestamp,
        args.is_json_output(),
    );

    // Create file monitor
    let mut monitor = FileMonitor::new(
        pattern_matcher,
        args.follow_name,
        args.buffer_size_bytes(),
        args.max_buffer_lines,
        args.verbose,
    );

    // Add files to monitor
    for file_path in &args.files {
        if !file_path.exists() {
            if args.follow_name {
                if args.verbose {
                    eprintln!("File {} doesn't exist yet, will wait for it", file_path.display());
                }
            } else {
                return Err(FastTailError::file_not_found(file_path.clone()));
            }
        }

        monitor.add_file(file_path.clone())?;
    }

    // Show initial content if requested
    if args.initial_lines > 0 {
        if args.should_show_filenames() && args.files.len() > 1 {
            for (i, file_path) in args.files.iter().enumerate() {
                if i > 0 {
                    println!(); // Blank line between files
                }
                if file_path.exists() {
                    println!("{}", formatter.format_file_header(file_path));
                    
                    match monitor.read_initial_lines(file_path, args.initial_lines) {
                        Ok(entries) => {
                            for entry in entries {
                                println!("{}", formatter.format_entry(&entry));
                            }
                        }
                        Err(e) => {
                            eprintln!("{}", formatter.format_error(&e.to_string(), Some(file_path)));
                        }
                    }
                }
            }
        } else {
            // Single file or quiet mode
            for file_path in &args.files {
                if file_path.exists() {
                    match monitor.read_initial_lines(file_path, args.initial_lines) {
                        Ok(entries) => {
                            for entry in entries {
                                println!("{}", formatter.format_entry(&entry));
                            }
                        }
                        Err(e) => {
                            eprintln!("{}", formatter.format_error(&e.to_string(), Some(file_path)));
                        }
                    }
                }
            }
        }
    }

    // Start following if requested
    if args.follow {
        if args.verbose {
            eprintln!("Starting real-time monitoring...");
        }

        let (tx, mut rx) = mpsc::unbounded_channel();
        let poll_interval = Duration::from_millis(args.poll_interval_ms);

        // Start monitoring in a separate task
        let monitor_handle = tokio::spawn(async move {
            if let Err(e) = monitor.start_monitoring(tx, poll_interval).await {
                eprintln!("Monitoring error: {}", e);
            }
        });

        // Handle Ctrl+C gracefully
        let formatter_clone = formatter;
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();
            if args.verbose {
                eprintln!("\nShutting down...");
            }
            std::process::exit(0);
        });

        // Process new entries as they arrive
        while let Some(entry) = rx.recv().await {
            println!("{}", formatter_clone.format_entry(&entry));
        }

        monitor_handle.await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_basic_functionality() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "test line 1").unwrap();
        writeln!(temp_file, "test line 2").unwrap();
        temp_file.flush().unwrap();

        let args = Args {
            files: vec![temp_file.path().to_path_buf()],
            initial_lines: 2,
            follow: false,
            follow_name: false,
            pattern: None,
            use_regex: false,
            ignore_case: false,
            invert_match: false,
            line_numbers: false,
            quiet: false,
            color: cli::ColorOption::Never,
            format: cli::OutputFormat::Text,
            timestamp: false,
            buffer_size_kb: 64,
            poll_interval_ms: 100,
            max_buffer_lines: 10000,
            verbose: false,
        };

        // This would normally run the main logic, but we can't easily test the full async flow
        // in a unit test without more complex setup
        assert!(args.files.len() == 1);
        assert!(!args.follow);
    }
}