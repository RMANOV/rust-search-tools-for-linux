mod ast;
mod cli;
mod errors;
mod interpreter;
mod lexer;
mod parser;
mod runtime;
mod value;

use clap::Parser;
use cli::Args;
use errors::{FastAwkError, Result};
use interpreter::Interpreter;
use parser::Parser as AwkParser;
use std::fs::File;
use std::io::{BufRead, BufReader, stdin};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();

    // Set up logging/verbosity
    if args.verbose && !args.quiet {
        eprintln!("Fast-AWK v0.1.0 - Ultra-fast AWK-compatible text processor");
    }

    // Get the AWK script
    let script = args.get_script()?;

    if args.print_program {
        println!("Program: {}", script);
        return Ok(());
    }

    // Parse the script
    let mut parser = AwkParser::new(&script)?;
    let program = parser.parse().map_err(|e| {
        FastAwkError::syntax_error(format!("Script parsing failed: {}", e))
    })?;

    if args.verbose && !args.quiet {
        eprintln!("Script parsed successfully");
        eprintln!("Rules: {}", program.rules.len());
        eprintln!("Functions: {}", program.functions.len());
        eprintln!("Has BEGIN: {}", program.has_begin_rules());
        eprintln!("Has END: {}", program.has_end_rules());
    }

    // Create interpreter
    let mut interpreter = Interpreter::new();

    // Initialize with command-line variable assignments
    let variable_assignments = args.parse_variable_assignments().map_err(|e| {
        FastAwkError::syntax_error(format!("Invalid variable assignment: {}", e))
    })?;
    interpreter.context.initialize_with_args(&variable_assignments)?;

    // Set built-in variables from command line
    if let Some(ref fs) = args.field_separator {
        interpreter.context.set_variable("FS", value::Value::String(fs.clone()));
    }
    if let Some(ref ofs) = args.output_separator {
        interpreter.context.set_variable("OFS", value::Value::String(ofs.clone()));
    }
    if let Some(ref rs) = args.record_separator {
        interpreter.context.set_variable("RS", value::Value::String(rs.clone()));
    }
    if let Some(ref ors) = args.output_record_separator {
        interpreter.context.set_variable("ORS", value::Value::String(ors.clone()));
    }

    // Execute BEGIN rules
    if args.verbose && !args.quiet && program.has_begin_rules() {
        eprintln!("Executing BEGIN rules...");
    }
    interpreter.execute_program(&program)?;

    // Check if we should exit early (e.g., from BEGIN block)
    if let Some(exit_code) = interpreter.context.exit_code {
        if args.verbose && !args.quiet {
            eprintln!("Exiting with code: {}", exit_code);
        }
        std::process::exit(exit_code);
    }

    // Process input files or stdin
    let main_rules = program.get_main_rules();
    if !main_rules.is_empty() || program.has_end_rules() {
        if args.files.is_empty() {
            // Read from stdin
            if args.verbose && !args.quiet {
                eprintln!("Reading from stdin...");
            }
            process_reader(&mut interpreter, &program, &args, stdin().lock(), "stdin")?;
        } else {
            // Process each file
            for file_path in &args.files {
                if args.verbose && !args.quiet {
                    eprintln!("Processing file: {}", file_path.display());
                }
                
                interpreter.context.set_filename(file_path.display().to_string());
                
                let file = File::open(file_path).map_err(|e| {
                    FastAwkError::file_not_found(file_path.clone())
                })?;
                
                let reader = BufReader::with_capacity(args.buffer_size_bytes(), file);
                process_reader(&mut interpreter, &program, &args, reader, &file_path.display().to_string())?;
                
                // Check for exit condition
                if interpreter.context.exit_code.is_some() {
                    break;
                }
            }
        }
    }

    // Execute END rules
    if program.has_end_rules() {
        if args.verbose && !args.quiet {
            eprintln!("Executing END rules...");
        }
        interpreter.execute_end_rules(&program)?;
    }

    // Exit with appropriate code
    let exit_code = interpreter.context.exit_code.unwrap_or(0);
    if args.verbose && !args.quiet {
        eprintln!("Records processed: {}", interpreter.context.nr);
        eprintln!("Exiting with code: {}", exit_code);
    }
    
    if exit_code != 0 {
        std::process::exit(exit_code);
    }

    Ok(())
}

fn process_reader<R: BufRead>(
    interpreter: &mut Interpreter,
    program: &ast::Program,
    args: &Args,
    reader: R,
    source_name: &str,
) -> Result<()> {
    let mut records_processed = 0;
    let mut records_skipped = 0;

    for (_line_number, line_result) in reader.lines().enumerate() {
        let line = line_result?;
        
        // Handle skip_records
        if let Some(skip_count) = args.skip_records {
            if records_processed < skip_count {
                records_skipped += 1;
                records_processed += 1;
                continue;
            }
        }
        
        // Handle max_records
        if let Some(max_count) = args.max_records {
            if records_processed - records_skipped >= max_count {
                break;
            }
        }

        // Process the record
        let _any_matched = interpreter.execute_main_rules(program, &line)?;
        records_processed += 1;

        // Check for control flow
        if interpreter.context.exit_code.is_some() {
            break;
        }

        // Verbose progress reporting
        if args.verbose && !args.quiet && records_processed % 10000 == 0 {
            eprintln!("Processed {} records from {}", records_processed, source_name);
        }
    }

    if args.verbose && !args.quiet {
        if records_skipped > 0 {
            eprintln!("Skipped {} records, processed {} records from {}", 
                     records_skipped, records_processed - records_skipped, source_name);
        } else {
            eprintln!("Processed {} records from {}", records_processed, source_name);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_simple_script() {
        let script = r#"BEGIN { print "Starting..." } { print NR, $0 } END { print "Done." }"#;
        let mut parser = AwkParser::new(script).unwrap();
        let program = parser.parse().unwrap();

        assert!(program.has_begin_rules());
        assert!(program.has_end_rules());
        assert_eq!(program.get_main_rules().len(), 1);
    }

    #[test]
    fn test_field_processing() {
        let script = r#"{ print $1, $2 }"#;
        let mut parser = AwkParser::new(script).unwrap();
        let program = parser.parse().unwrap();

        let mut interpreter = Interpreter::new();
        interpreter.execute_program(&program).unwrap();

        let input = "hello world\nfoo bar\n";
        let reader = Cursor::new(input);
        
        // This would normally output the fields, but in tests we just verify no errors
        let args = Args {
            script: script.to_string(),
            files: vec![],
            field_separator: None,
            output_separator: None,
            record_separator: None,
            output_record_separator: None,
            variables: vec![],
            script_file: None,
            print_program: false,
            format: cli::OutputFormat::Text,
            color: cli::ColorOption::Never,
            threads: None,
            buffer_size_kb: 64,
            verbose: false,
            quiet: true,
            ignore_case: false,
            max_records: None,
            skip_records: None,
            posix_mode: false,
            traditional_mode: false,
        };
        
        let result = process_reader(&mut interpreter, &program, &args, reader, "test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_variable_assignment() {
        let script = r#"BEGIN { x = 42; print x }"#;
        let mut parser = AwkParser::new(script).unwrap();
        let program = parser.parse().unwrap();

        let mut interpreter = Interpreter::new();
        let result = interpreter.execute_program(&program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_field_separator() {
        let script = r#"{ print $1 }"#;
        let mut parser = AwkParser::new(script).unwrap();
        let program = parser.parse().unwrap();

        let mut interpreter = Interpreter::new();
        interpreter.context.set_variable("FS", value::Value::String(",".to_string()));
        
        interpreter.execute_program(&program).unwrap();
        
        // Test with CSV data
        let _any_matched = interpreter.execute_main_rules(&program, "a,b,c").unwrap();
        assert_eq!(interpreter.context.get_field(1), "a");
    }

    #[test]
    fn test_built_in_functions() {
        let script = r#"BEGIN { print length("hello"); print substr("hello", 2, 3) }"#;
        let mut parser = AwkParser::new(script).unwrap();
        let program = parser.parse().unwrap();

        let mut interpreter = Interpreter::new();
        let result = interpreter.execute_program(&program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pattern_matching() {
        let script = r#"/world/ { print "Found world!" }"#;
        let mut parser = AwkParser::new(script).unwrap();
        let program = parser.parse().unwrap();

        let mut interpreter = Interpreter::new();
        interpreter.execute_program(&program).unwrap();
        
        let matched1 = interpreter.execute_main_rules(&program, "hello world").unwrap();
        let matched2 = interpreter.execute_main_rules(&program, "hello there").unwrap();
        
        assert!(matched1);
        assert!(!matched2);
    }

    #[test]
    fn test_arithmetic_operations() {
        let script = r#"BEGIN { print 1 + 2 * 3; print 10 / 2; print 10 % 3 }"#;
        let mut parser = AwkParser::new(script).unwrap();
        let program = parser.parse().unwrap();

        let mut interpreter = Interpreter::new();
        let result = interpreter.execute_program(&program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_control_flow() {
        let script = r#"BEGIN { for (i = 1; i <= 3; i++) print i }"#;
        let mut parser = AwkParser::new(script).unwrap();
        let program = parser.parse().unwrap();

        let mut interpreter = Interpreter::new();
        let result = interpreter.execute_program(&program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_file_processing() -> std::io::Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "line1")?;
        writeln!(temp_file, "line2")?;
        writeln!(temp_file, "line3")?;
        temp_file.flush()?;

        let script = r#"{ print NR, $0 }"#;
        let mut parser = AwkParser::new(script).unwrap();
        let program = parser.parse().unwrap();

        let mut interpreter = Interpreter::new();
        interpreter.execute_program(&program).unwrap();

        let file = File::open(temp_file.path())?;
        let reader = BufReader::new(file);
        
        let args = Args {
            script: script.to_string(),
            files: vec![],
            field_separator: None,
            output_separator: None,
            record_separator: None,
            output_record_separator: None,
            variables: vec![],
            script_file: None,
            print_program: false,
            format: cli::OutputFormat::Text,
            color: cli::ColorOption::Never,
            threads: None,
            buffer_size_kb: 64,
            verbose: false,
            quiet: true,
            ignore_case: false,
            max_records: None,
            skip_records: None,
            posix_mode: false,
            traditional_mode: false,
        };
        
        let result = process_reader(&mut interpreter, &program, &args, reader, "test_file");
        assert!(result.is_ok());
        assert_eq!(interpreter.context.nr, 3);

        Ok(())
    }
}