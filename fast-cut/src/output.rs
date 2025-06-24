use crate::cli::OutputFormat;
use crate::errors::Result;
use crate::field_parser::ParsedLine;
use colored::*;
use serde_json::Value;
use std::collections::HashMap;

pub struct OutputFormatter {
    format: OutputFormat,
    use_colors: bool,
    output_delimiter: String,
    line_numbers: bool,
    header_names: Option<Vec<String>>,
}

impl OutputFormatter {
    pub fn new(
        format: OutputFormat,
        use_colors: bool,
        output_delimiter: Option<String>,
        line_numbers: bool,
    ) -> Self {
        let delimiter = output_delimiter.unwrap_or_else(|| {
            match format {
                OutputFormat::Csv => ",".to_string(),
                OutputFormat::Json => ",".to_string(),
                OutputFormat::Text => "\t".to_string(),
            }
        });

        Self {
            format,
            use_colors,
            output_delimiter: delimiter,
            line_numbers,
            header_names: None,
        }
    }

    pub fn set_header_names(&mut self, names: Vec<String>) {
        self.header_names = Some(names);
    }

    pub fn format_header(&self, header_fields: &[String]) -> Result<String> {
        match self.format {
            OutputFormat::Text => {
                let mut output = String::new();
                if self.line_numbers {
                    output.push_str("line");
                    output.push_str(&self.output_delimiter);
                }
                
                let header_line = header_fields.join(&self.output_delimiter);
                if self.use_colors {
                    output.push_str(&header_line.cyan().bold().to_string());
                } else {
                    output.push_str(&header_line);
                }
                Ok(output)
            }
            OutputFormat::Csv => {
                let mut wtr = csv::WriterBuilder::new()
                    .delimiter(self.get_csv_delimiter())
                    .from_writer(vec![]);
                
                let mut record = Vec::new();
                if self.line_numbers {
                    record.push("line");
                }
                record.extend(header_fields.iter().map(|s| s.as_str()));
                
                wtr.write_record(&record)?;
                let data = wtr.into_inner().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                Ok(String::from_utf8_lossy(&data).trim_end().to_string())
            }
            OutputFormat::Json => {
                // For JSON, we'll output the header as a comment or metadata
                Ok(format!(
                    "{{\"_metadata\":{{\"fields\":{},\"line_numbers\":{}}}}}",
                    serde_json::to_string(header_fields)?,
                    self.line_numbers
                ))
            }
        }
    }

    pub fn format_line(&self, parsed_line: &ParsedLine) -> Result<String> {
        match self.format {
            OutputFormat::Text => self.format_text_line(parsed_line),
            OutputFormat::Csv => self.format_csv_line(parsed_line),
            OutputFormat::Json => self.format_json_line(parsed_line),
        }
    }

    fn format_text_line(&self, parsed_line: &ParsedLine) -> Result<String> {
        let mut output = String::new();
        
        if self.line_numbers {
            let line_num_str = parsed_line.line_number.to_string();
            if self.use_colors {
                output.push_str(&line_num_str.green().to_string());
            } else {
                output.push_str(&line_num_str);
            }
            output.push_str(&self.output_delimiter);
        }

        let fields_str = parsed_line.fields.join(&self.output_delimiter);
        
        // Apply alternating colors for better readability
        if self.use_colors && parsed_line.fields.len() > 1 {
            let colored_fields: Vec<String> = parsed_line.fields
                .iter()
                .enumerate()
                .map(|(i, field)| {
                    if i % 2 == 0 {
                        field.normal().to_string()
                    } else {
                        field.bright_white().to_string()
                    }
                })
                .collect();
            output.push_str(&colored_fields.join(&self.output_delimiter));
        } else {
            output.push_str(&fields_str);
        }

        Ok(output)
    }

    fn format_csv_line(&self, parsed_line: &ParsedLine) -> Result<String> {
        let mut wtr = csv::WriterBuilder::new()
            .delimiter(self.get_csv_delimiter())
            .from_writer(vec![]);
        
        let mut record = Vec::new();
        if self.line_numbers {
            record.push(parsed_line.line_number.to_string());
        }
        record.extend(parsed_line.fields.iter().cloned());
        
        wtr.write_record(&record)?;
        let data = wtr.into_inner().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(String::from_utf8_lossy(&data).trim_end().to_string())
    }

    fn format_json_line(&self, parsed_line: &ParsedLine) -> Result<String> {
        let mut obj = HashMap::new();
        
        if self.line_numbers {
            obj.insert("line_number".to_string(), Value::Number(parsed_line.line_number.into()));
        }

        // Use header names if available, otherwise use field indices
        if let Some(ref headers) = self.header_names {
            let mut fields_obj = HashMap::new();
            for (i, field) in parsed_line.fields.iter().enumerate() {
                let field_name = headers.get(i)
                    .map(|s| s.clone())
                    .unwrap_or_else(|| format!("field_{}", i + 1));
                fields_obj.insert(field_name, Value::String(field.clone()));
            }
            obj.insert("fields".to_string(), Value::Object(fields_obj.into_iter().collect()));
        } else {
            let fields: Vec<Value> = parsed_line.fields
                .iter()
                .map(|f| Value::String(f.clone()))
                .collect();
            obj.insert("fields".to_string(), Value::Array(fields));
        }

        Ok(serde_json::to_string(&obj)?)
    }

    fn get_csv_delimiter(&self) -> u8 {
        self.output_delimiter.chars().next().unwrap_or(',') as u8
    }

    pub fn format_error(&self, error: &str, line_number: Option<usize>) -> String {
        let error_msg = if let Some(line_num) = line_number {
            format!("Error at line {}: {}", line_num, error)
        } else {
            format!("Error: {}", error)
        };

        if self.use_colors {
            error_msg.red().bold().to_string()
        } else {
            error_msg
        }
    }

    pub fn format_info(&self, message: &str) -> String {
        if self.use_colors {
            message.blue().to_string()
        } else {
            message.to_string()
        }
    }

    pub fn format_warning(&self, message: &str) -> String {
        if self.use_colors {
            message.yellow().to_string()
        } else {
            format!("Warning: {}", message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_parser::ParsedLine;

    #[test]
    fn test_text_formatting() {
        let formatter = OutputFormatter::new(
            OutputFormat::Text,
            false,
            Some("\t".to_string()),
            true,
        );
        
        let parsed_line = ParsedLine {
            line_number: 42,
            fields: vec!["field1".to_string(), "field2".to_string()],
            raw_line: "field1,field2".to_string(),
        };
        
        let result = formatter.format_line(&parsed_line).unwrap();
        assert_eq!(result, "42\tfield1\tfield2");
    }

    #[test]
    fn test_csv_formatting() {
        let formatter = OutputFormatter::new(
            OutputFormat::Csv,
            false,
            None,
            false,
        );
        
        let parsed_line = ParsedLine {
            line_number: 1,
            fields: vec!["hello, world".to_string(), "test".to_string()],
            raw_line: "hello, world,test".to_string(),
        };
        
        let result = formatter.format_line(&parsed_line).unwrap();
        assert!(result.contains("\"hello, world\""));
        assert!(result.contains("test"));
    }

    #[test]
    fn test_json_formatting() {
        let mut formatter = OutputFormatter::new(
            OutputFormat::Json,
            false,
            None,
            true,
        );
        formatter.set_header_names(vec!["name".to_string(), "age".to_string()]);
        
        let parsed_line = ParsedLine {
            line_number: 1,
            fields: vec!["John".to_string(), "30".to_string()],
            raw_line: "John,30".to_string(),
        };
        
        let result = formatter.format_line(&parsed_line).unwrap();
        assert!(result.contains("\"line_number\":1"));
        assert!(result.contains("\"name\":\"John\""));
        assert!(result.contains("\"age\":\"30\""));
    }

    #[test]
    fn test_header_formatting() {
        let formatter = OutputFormatter::new(
            OutputFormat::Text,
            false,
            Some(",".to_string()),
            false,
        );
        
        let header_fields = vec!["Name".to_string(), "Age".to_string(), "City".to_string()];
        let result = formatter.format_header(&header_fields).unwrap();
        assert_eq!(result, "Name,Age,City");
    }
}