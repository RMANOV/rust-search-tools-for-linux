mod cli;
mod errors;
mod field_parser;
mod output;
mod stream_processor;

use cli::Args;
use clap::Parser;
use errors::{FastCutError, Result};
use stream_processor::StreamProcessor;

fn main() -> Result<()> {
    let args = Args::parse();

    if args.verbose {
        eprintln!("Fast-cut starting with {} files", args.files.len());
        if let Some(ref delimiter) = args.get_input_delimiter() {
            eprintln!("Using delimiter: {:?}", delimiter);
        } else {
            eprintln!("Auto-detecting delimiter");
        }
    }

    // Validate arguments
    if args.fields.trim().is_empty() {
        return Err(FastCutError::invalid_config("No fields specified"));
    }

    // Validate field selector
    let _field_selector = args.parse_field_selector()
        .map_err(FastCutError::invalid_field_selector)?;

    // Check if conflicting delimiter options are specified
    let delimiter_count = [
        args.delimiter.is_some(),
        args.tab_delimiter,
        args.space_delimiter,
        args.csv_mode,
    ].iter().filter(|&&x| x).count();

    if delimiter_count > 1 {
        return Err(FastCutError::invalid_config(
            "Multiple delimiter options specified. Use only one of: -d, -t, -s, -c"
        ));
    }

    // Create stream processor
    let mut processor = StreamProcessor::new(&args)?;

    // Process files
    processor.process_files(&args.files, &args)?;

    if args.verbose {
        eprintln!("Processing completed successfully");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_main_functionality() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "name,age,city").unwrap();
        writeln!(temp_file, "John,30,NYC").unwrap();
        writeln!(temp_file, "Jane,25,LA").unwrap();
        temp_file.flush().unwrap();

        let args = Args {
            files: vec![temp_file.path().to_path_buf()],
            fields: "1,3".to_string(),
            delimiter: Some(",".to_string()),
            tab_delimiter: false,
            space_delimiter: false,
            csv_mode: false,
            output_delimiter: None,
            format: cli::OutputFormat::Text,
            has_header: true,
            skip_header: false,
            line_numbers: false,
            zero_terminated: false,
            skip_lines: 0,
            max_lines: 0,
            color: cli::ColorOption::Never,
            threads: None,
            buffer_size_kb: 64,
            non_empty_only: false,
            verbose: false,
        };

        // This would normally process and output, but we just test that it doesn't panic
        let mut processor = StreamProcessor::new(&args).unwrap();
        let result = processor.process_files(&args.files, &args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_field_selector_validation() {
        let args = Args {
            files: vec![],
            fields: "1,3,5-7".to_string(),
            delimiter: None,
            tab_delimiter: false,
            space_delimiter: false,
            csv_mode: false,
            output_delimiter: None,
            format: cli::OutputFormat::Text,
            has_header: false,
            skip_header: false,
            line_numbers: false,
            zero_terminated: false,
            skip_lines: 0,
            max_lines: 0,
            color: cli::ColorOption::Never,
            threads: None,
            buffer_size_kb: 64,
            non_empty_only: false,
            verbose: false,
        };

        let field_selector = args.parse_field_selector();
        assert!(field_selector.is_ok());
        
        let selector = field_selector.unwrap();
        assert_eq!(selector.indices, vec![0, 2]); // 1-based to 0-based conversion
        assert_eq!(selector.ranges, vec![(4, 6)]); // 5-7 becomes (4, 6) in 0-based
    }

    #[test]
    fn test_invalid_field_selector() {
        let args = Args {
            files: vec![],
            fields: "0,invalid".to_string(),
            delimiter: None,
            tab_delimiter: false,
            space_delimiter: false,
            csv_mode: false,
            output_delimiter: None,
            format: cli::OutputFormat::Text,
            has_header: false,
            skip_header: false,
            line_numbers: false,
            zero_terminated: false,
            skip_lines: 0,
            max_lines: 0,
            color: cli::ColorOption::Never,
            threads: None,
            buffer_size_kb: 64,
            non_empty_only: false,
            verbose: false,
        };

        let field_selector = args.parse_field_selector();
        assert!(field_selector.is_err());
    }

    #[test]
    fn test_conflicting_delimiters() {
        let args = Args {
            files: vec![],
            fields: "1".to_string(),
            delimiter: Some(",".to_string()),
            tab_delimiter: true, // Conflicting with delimiter
            space_delimiter: false,
            csv_mode: false,
            output_delimiter: None,
            format: cli::OutputFormat::Text,
            has_header: false,
            skip_header: false,
            line_numbers: false,
            zero_terminated: false,
            skip_lines: 0,
            max_lines: 0,
            color: cli::ColorOption::Never,
            threads: None,
            buffer_size_kb: 64,
            non_empty_only: false,
            verbose: false,
        };

        // This should fail in main() validation
        let delimiter_count = [
            args.delimiter.is_some(),
            args.tab_delimiter,
            args.space_delimiter,
            args.csv_mode,
        ].iter().filter(|&&x| x).count();

        assert!(delimiter_count > 1);
    }
}