use crate::cli::FieldSelector;
use crate::errors::{FastCutError, Result};
use memchr::memchr_iter;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ParsedLine {
    pub line_number: usize,
    pub fields: Vec<String>,
    pub raw_line: String,
}

#[derive(Debug, Clone)]
pub struct FieldParser {
    delimiter: Option<String>,
    csv_mode: bool,
    space_mode: bool,
    header_map: Option<HashMap<String, usize>>,
    field_selector: FieldSelector,
}

impl FieldParser {
    pub fn new(
        delimiter: Option<String>,
        csv_mode: bool,
        space_mode: bool,
        field_selector: FieldSelector,
    ) -> Self {
        Self {
            delimiter,
            csv_mode,
            space_mode,
            header_map: None,
            field_selector,
        }
    }

    pub fn set_header(&mut self, header_line: &str) -> Result<()> {
        let fields = self.parse_line_fields(header_line)?;
        let mut header_map = HashMap::new();
        
        for (index, field) in fields.iter().enumerate() {
            header_map.insert(field.trim().to_string(), index);
        }
        
        self.header_map = Some(header_map);
        Ok(())
    }

    pub fn parse_line(&self, line: &str, line_number: usize) -> Result<ParsedLine> {
        let all_fields = self.parse_line_fields(line)?;
        let selected_fields = self.select_fields(&all_fields)?;
        
        Ok(ParsedLine {
            line_number,
            fields: selected_fields,
            raw_line: line.to_string(),
        })
    }

    fn parse_line_fields(&self, line: &str) -> Result<Vec<String>> {
        if line.trim().is_empty() {
            return Ok(Vec::new());
        }

        if self.csv_mode {
            self.parse_csv_line(line)
        } else if self.space_mode {
            self.parse_space_delimited(line)
        } else if let Some(ref delimiter) = self.delimiter {
            self.parse_delimited_line(line, delimiter)
        } else {
            self.auto_detect_and_parse(line)
        }
    }

    fn parse_csv_line(&self, line: &str) -> Result<Vec<String>> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(line.as_bytes());
        
        let mut record = csv::StringRecord::new();
        if rdr.read_record(&mut record)? {
            Ok(record.iter().map(|field| field.to_string()).collect())
        } else {
            Ok(Vec::new())
        }
    }

    fn parse_space_delimited(&self, line: &str) -> Result<Vec<String>> {
        // Split on whitespace and collapse multiple spaces
        Ok(line
            .split_whitespace()
            .map(|s| s.to_string())
            .collect())
    }

    fn parse_delimited_line(&self, line: &str, delimiter: &str) -> Result<Vec<String>> {
        if delimiter.len() == 1 {
            // Single character delimiter - use SIMD optimization
            let delim_byte = delimiter.as_bytes()[0];
            self.parse_single_char_delimited(line, delim_byte)
        } else {
            // Multi-character delimiter
            Ok(line.split(delimiter).map(|s| s.to_string()).collect())
        }
    }

    fn parse_single_char_delimited(&self, line: &str, delimiter: u8) -> Result<Vec<String>> {
        let line_bytes = line.as_bytes();
        let mut fields = Vec::new();
        let mut start = 0;

        // Use SIMD-optimized memchr to find delimiters
        for pos in memchr_iter(delimiter, line_bytes) {
            let field = std::str::from_utf8(&line_bytes[start..pos])
                .map_err(|e| FastCutError::encoding_error(e.to_string()))?;
            fields.push(field.to_string());
            start = pos + 1;
        }

        // Add the last field
        if start < line_bytes.len() {
            let field = std::str::from_utf8(&line_bytes[start..])
                .map_err(|e| FastCutError::encoding_error(e.to_string()))?;
            fields.push(field.to_string());
        } else if start == line_bytes.len() {
            // Line ends with delimiter
            fields.push(String::new());
        }

        Ok(fields)
    }

    fn auto_detect_and_parse(&self, line: &str) -> Result<Vec<String>> {
        let line_bytes = line.as_bytes();
        
        // Count different delimiter types
        let comma_count = memchr_iter(b',', line_bytes).count();
        let tab_count = memchr_iter(b'\t', line_bytes).count();
        let space_count = line.split_whitespace().count().saturating_sub(1);

        // Choose the most frequent delimiter
        if comma_count > 0 && comma_count >= tab_count && comma_count >= space_count {
            self.parse_single_char_delimited(line, b',')
        } else if tab_count > 0 && tab_count >= space_count {
            self.parse_single_char_delimited(line, b'\t')
        } else if space_count > 0 {
            self.parse_space_delimited(line)
        } else {
            // No delimiters found, treat as single field
            Ok(vec![line.to_string()])
        }
    }

    fn select_fields(&self, all_fields: &[String]) -> Result<Vec<String>> {
        let mut selected = Vec::new();

        // Process individual indices
        for &index in &self.field_selector.indices {
            if index >= all_fields.len() {
                return Err(FastCutError::invalid_field_index(index + 1, all_fields.len()));
            }
            selected.push(all_fields[index].clone());
        }

        // Process ranges
        for &(start, end) in &self.field_selector.ranges {
            if start >= all_fields.len() {
                return Err(FastCutError::invalid_field_index(start + 1, all_fields.len()));
            }
            let actual_end = std::cmp::min(end, all_fields.len() - 1);
            for i in start..=actual_end {
                selected.push(all_fields[i].clone());
            }
        }

        // Process field names
        if !self.field_selector.names.is_empty() {
            let header_map = self.header_map.as_ref()
                .ok_or(FastCutError::NoHeaderFound)?;
            
            for name in &self.field_selector.names {
                if let Some(&index) = header_map.get(name) {
                    if index < all_fields.len() {
                        selected.push(all_fields[index].clone());
                    } else {
                        return Err(FastCutError::invalid_field_index(index + 1, all_fields.len()));
                    }
                } else {
                    let available: Vec<String> = header_map.keys().cloned().collect();
                    return Err(FastCutError::field_not_found(name.clone(), available));
                }
            }
        }

        Ok(selected)
    }

    pub fn get_header_fields(&self) -> Option<Vec<String>> {
        self.header_map.as_ref().map(|map| {
            let mut fields: Vec<(String, usize)> = map.iter().map(|(k, &v)| (k.clone(), v)).collect();
            fields.sort_by_key(|(_, index)| *index);
            fields.into_iter().map(|(name, _)| name).collect()
        })
    }

    pub fn detect_delimiter(line: &str) -> Option<String> {
        let line_bytes = line.as_bytes();
        
        let comma_count = memchr_iter(b',', line_bytes).count();
        let tab_count = memchr_iter(b'\t', line_bytes).count();
        let semicolon_count = memchr_iter(b';', line_bytes).count();
        let pipe_count = memchr_iter(b'|', line_bytes).count();

        // Return the most frequent delimiter
        let max_count = comma_count.max(tab_count).max(semicolon_count).max(pipe_count);
        
        if max_count == 0 {
            return None;
        }

        if comma_count == max_count {
            Some(",".to_string())
        } else if tab_count == max_count {
            Some("\t".to_string())
        } else if semicolon_count == max_count {
            Some(";".to_string())
        } else if pipe_count == max_count {
            Some("|".to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::FieldSelector;

    #[test]
    fn test_csv_parsing() {
        let selector = FieldSelector {
            indices: vec![0, 2],
            ranges: vec![],
            names: vec![],
        };
        let parser = FieldParser::new(None, true, false, selector);
        
        let result = parser.parse_line_fields("\"hello, world\",test,\"quoted\"").unwrap();
        assert_eq!(result, vec!["hello, world", "test", "quoted"]);
    }

    #[test]
    fn test_tab_delimited() {
        let selector = FieldSelector {
            indices: vec![0, 1],
            ranges: vec![],
            names: vec![],
        };
        let parser = FieldParser::new(Some("\t".to_string()), false, false, selector);
        
        let result = parser.parse_line_fields("field1\tfield2\tfield3").unwrap();
        assert_eq!(result, vec!["field1", "field2", "field3"]);
    }

    #[test]
    fn test_space_delimited() {
        let selector = FieldSelector {
            indices: vec![0, 2],
            ranges: vec![],
            names: vec![],
        };
        let parser = FieldParser::new(None, false, true, selector);
        
        let result = parser.parse_line_fields("  field1    field2   field3  ").unwrap();
        assert_eq!(result, vec!["field1", "field2", "field3"]);
    }

    #[test]
    fn test_field_selection() {
        let selector = FieldSelector {
            indices: vec![0, 2],
            ranges: vec![(1, 2)],
            names: vec![],
        };
        let parser = FieldParser::new(Some(",".to_string()), false, false, selector);
        
        let fields = vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string()];
        let selected = parser.select_fields(&fields).unwrap();
        assert_eq!(selected, vec!["a", "c", "b", "c"]);
    }

    #[test]
    fn test_delimiter_detection() {
        assert_eq!(FieldParser::detect_delimiter("a,b,c"), Some(",".to_string()));
        assert_eq!(FieldParser::detect_delimiter("a\tb\tc"), Some("\t".to_string()));
        assert_eq!(FieldParser::detect_delimiter("a;b;c"), Some(";".to_string()));
        assert_eq!(FieldParser::detect_delimiter("a|b|c"), Some("|".to_string()));
        assert_eq!(FieldParser::detect_delimiter("abc"), None);
    }
}