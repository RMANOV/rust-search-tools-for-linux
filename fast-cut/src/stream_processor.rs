use crate::cli::Args;
use crate::errors::{FastCutError, Result};
use crate::field_parser::FieldParser;
use crate::output::OutputFormatter;
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, stdin};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct StreamProcessor {
    field_parser: FieldParser,
    output_formatter: OutputFormatter,
    buffer_size: usize,
    threads: usize,
    verbose: bool,
}

impl StreamProcessor {
    pub fn new(args: &Args) -> Result<Self> {
        let field_selector = args.parse_field_selector()
            .map_err(FastCutError::invalid_field_selector)?;

        let field_parser = FieldParser::new(
            args.get_input_delimiter(),
            args.is_csv_mode(),
            args.space_delimiter,
            field_selector,
        );

        let output_formatter = OutputFormatter::new(
            args.format.clone(),
            args.should_use_colors(),
            args.get_output_delimiter(),
            args.line_numbers,
        );

        Ok(Self {
            field_parser,
            output_formatter,
            buffer_size: args.buffer_size_bytes(),
            threads: args.get_threads(),
            verbose: args.verbose,
        })
    }

    pub fn process_files(&mut self, files: &[std::path::PathBuf], args: &Args) -> Result<()> {
        if files.is_empty() {
            self.process_stdin(args)
        } else if files.len() == 1 {
            self.process_single_file(&files[0], args)
        } else {
            self.process_multiple_files(files, args)
        }
    }

    fn process_stdin(&mut self, args: &Args) -> Result<()> {
        if self.verbose {
            eprintln!("Reading from stdin...");
        }

        let stdin = stdin();
        let reader = stdin.lock();
        self.process_reader(reader, args, "stdin")
    }

    fn process_single_file(&mut self, file_path: &Path, args: &Args) -> Result<()> {
        if self.verbose {
            eprintln!("Processing file: {}", file_path.display());
        }

        let file = File::open(file_path)
            .map_err(|_| FastCutError::file_not_found(file_path.to_path_buf()))?;
        
        let reader = BufReader::with_capacity(self.buffer_size, file);
        self.process_reader(reader, args, &file_path.display().to_string())
    }

    fn process_multiple_files(&mut self, files: &[std::path::PathBuf], args: &Args) -> Result<()> {
        // For multiple files, we can process them in parallel
        let processed_count = Arc::new(AtomicUsize::new(0));
        let total_files = files.len();

        let results: Result<Vec<_>> = files
            .par_iter()
            .map(|file_path| {
                let mut processor = self.clone_processor(args)?;
                let result = processor.process_single_file(file_path, args);
                
                let count = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
                if self.verbose {
                    eprintln!("Processed {}/{} files", count, total_files);
                }
                
                result
            })
            .collect();

        results?;
        Ok(())
    }

    fn clone_processor(&self, args: &Args) -> Result<StreamProcessor> {
        StreamProcessor::new(args)
    }

    fn process_reader<R: BufRead>(&mut self, reader: R, args: &Args, source_name: &str) -> Result<()> {
        let mut line_number = 0;
        let mut processed_lines = 0;
        let mut header_processed = false;

        for line_result in reader.lines() {
            let line = line_result?;
            line_number += 1;

            // Skip lines if requested
            if !args.should_process_line(line_number - 1) {
                continue;
            }

            // Handle header line
            if args.has_header && !header_processed {
                if args.skip_header {
                    self.field_parser.set_header(&line)?;
                    header_processed = true;
                    continue;
                } else {
                    self.field_parser.set_header(&line)?;
                    if let Some(header_fields) = self.field_parser.get_header_fields() {
                        self.output_formatter.set_header_names(header_fields.clone());
                        let header_output = self.output_formatter.format_header(&header_fields)?;
                        println!("{}", header_output);
                    }
                    header_processed = true;
                    continue;
                }
            }

            // Skip empty lines if requested
            if args.non_empty_only && line.trim().is_empty() {
                continue;
            }

            // Process the line
            match self.process_line(&line, line_number) {
                Ok(Some(output)) => {
                    println!("{}", output);
                    processed_lines += 1;
                }
                Ok(None) => {
                    // Line was filtered out or empty
                }
                Err(e) => {
                    if self.verbose {
                        eprintln!("{}", self.output_formatter.format_error(&e.to_string(), Some(line_number)));
                    }
                    // Continue processing other lines
                }
            }

            // Check if we've hit the max lines limit
            if args.max_lines > 0 && processed_lines >= args.max_lines {
                break;
            }
        }

        if self.verbose {
            eprintln!("Processed {} lines from {}", processed_lines, source_name);
        }

        Ok(())
    }

    fn process_line(&self, line: &str, line_number: usize) -> Result<Option<String>> {
        if line.trim().is_empty() {
            return Ok(None);
        }

        let parsed_line = self.field_parser.parse_line(line, line_number)?;
        
        // Check if any fields were extracted
        if parsed_line.fields.is_empty() {
            return Ok(None);
        }

        let output = self.output_formatter.format_line(&parsed_line)?;
        Ok(Some(output))
    }

    pub fn process_parallel_chunks<R: Read + Send>(&mut self, reader: R, args: &Args) -> Result<()> {
        // For very large files, we can process in parallel chunks
        const CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks
        
        let mut buffer = vec![0; CHUNK_SIZE];
        let mut reader = BufReader::with_capacity(self.buffer_size, reader);
        let mut chunk_number = 0;
        
        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break; // EOF
            }
            
            chunk_number += 1;
            if self.verbose && chunk_number % 100 == 0 {
                eprintln!("Processing chunk {}...", chunk_number);
            }
            
            // Find complete lines in the chunk
            let chunk_data = &buffer[..bytes_read];
            let chunk_str = std::str::from_utf8(chunk_data)
                .map_err(|e| FastCutError::encoding_error(e.to_string()))?;
            
            // Process lines in this chunk
            for (line_idx, line) in chunk_str.lines().enumerate() {
                let line_number = (chunk_number - 1) * CHUNK_SIZE + line_idx + 1;
                
                if !args.should_process_line(line_number - 1) {
                    continue;
                }
                
                match self.process_line(line, line_number) {
                    Ok(Some(output)) => println!("{}", output),
                    Ok(None) => {} // Filtered out
                    Err(e) => {
                        if self.verbose {
                            eprintln!("{}", self.output_formatter.format_error(&e.to_string(), Some(line_number)));
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    pub fn get_field_parser(&self) -> &FieldParser {
        &self.field_parser
    }

    pub fn get_output_formatter(&self) -> &OutputFormatter {
        &self.output_formatter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Args, OutputFormat, ColorOption, FieldSelector};
    use std::io::Cursor;
    use tempfile::NamedTempFile;
    use std::io::Write;

    fn create_test_args() -> Args {
        Args {
            files: vec![],
            fields: "1,3".to_string(),
            delimiter: Some(",".to_string()),
            tab_delimiter: false,
            space_delimiter: false,
            csv_mode: false,
            output_delimiter: None,
            format: OutputFormat::Text,
            has_header: false,
            skip_header: false,
            line_numbers: false,
            zero_terminated: false,
            skip_lines: 0,
            max_lines: 0,
            color: ColorOption::Never,
            threads: None,
            buffer_size_kb: 64,
            non_empty_only: false,
            verbose: false,
        }
    }

    #[test]
    fn test_stream_processor_creation() {
        let args = create_test_args();
        let processor = StreamProcessor::new(&args);
        assert!(processor.is_ok());
    }

    #[test]
    fn test_process_reader() {
        let args = create_test_args();
        let mut processor = StreamProcessor::new(&args).unwrap();
        
        let input = "field1,field2,field3\nvalue1,value2,value3\n";
        let reader = Cursor::new(input);
        
        // This would normally print to stdout, but in tests we just verify it doesn't error
        let result = processor.process_reader(reader, &args, "test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_file_processing() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "a,b,c").unwrap();
        writeln!(temp_file, "1,2,3").unwrap();
        writeln!(temp_file, "x,y,z").unwrap();
        temp_file.flush().unwrap();

        let args = create_test_args();
        let mut processor = StreamProcessor::new(&args).unwrap();
        
        let result = processor.process_single_file(temp_file.path(), &args);
        assert!(result.is_ok());
    }
}