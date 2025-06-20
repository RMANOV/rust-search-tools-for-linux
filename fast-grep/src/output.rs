use anyhow::Result;
use colored::*;
use std::path::Path;

pub struct OutputFormatter {
    show_line_numbers: bool,
    show_filenames: bool,
    use_colors: bool,
    json_output: bool,
    before_context: usize,
    after_context: usize,
}

impl OutputFormatter {
    pub fn new(
        show_line_numbers: bool,
        show_filenames: bool,
        use_colors: bool,
        json_output: bool,
        before_context: usize,
        after_context: usize,
    ) -> Self {
        Self {
            show_line_numbers,
            show_filenames,
            use_colors,
            json_output,
            before_context,
            after_context,
        }
    }

    pub fn format_match(
        &self,
        file_path: &Path,
        line_number: usize,
        line_content: &str,
        match_start: usize,
        match_end: usize,
    ) -> String {
        if self.json_output {
            self.format_json_match(file_path, line_number, line_content, match_start, match_end)
        } else {
            self.format_text_match(file_path, line_number, line_content, match_start, match_end)
        }
    }

    fn format_text_match(
        &self,
        file_path: &Path,
        line_number: usize,
        line_content: &str,
        match_start: usize,
        match_end: usize,
    ) -> String {
        let mut output = String::new();

        // File path
        if self.show_filenames {
            let file_str = file_path.display().to_string();
            if self.use_colors {
                output.push_str(&file_str.magenta().bold().to_string());
            } else {
                output.push_str(&file_str);
            }
            output.push(':');
        }

        // Line number
        if self.show_line_numbers {
            let line_str = line_number.to_string();
            if self.use_colors {
                output.push_str(&line_str.green().to_string());
            } else {
                output.push_str(&line_str);
            }
            output.push(':');
        }

        // Line content with highlighted matches
        if self.use_colors {
            output.push_str(&self.highlight_match(line_content, match_start, match_end));
        } else {
            output.push_str(line_content);
        }

        output
    }

    fn format_json_match(
        &self,
        file_path: &Path,
        line_number: usize,
        line_content: &str,
        match_start: usize,
        match_end: usize,
    ) -> String {
        format!(
            r#"{{"file":"{}","line":{},"content":"{}","match_start":{},"match_end":{}}}"#,
            file_path.display(),
            line_number,
            line_content.replace('"', r#"\""#),
            match_start,
            match_end
        )
    }

    fn highlight_match(&self, line: &str, start: usize, end: usize) -> String {
        let mut result = String::new();
        
        // Add text before match
        if start > 0 {
            result.push_str(&line[..start]);
        }
        
        // Add highlighted match
        if end <= line.len() {
            let match_text = &line[start..end];
            result.push_str(&match_text.red().bold().to_string());
        }
        
        // Add text after match
        if end < line.len() {
            result.push_str(&line[end..]);
        }
        
        result
    }

    pub fn format_file_header(&self, file_path: &Path) -> String {
        if self.use_colors {
            format!("{}:", file_path.display().to_string().cyan().bold())
        } else {
            format!("{}:", file_path.display())
        }
    }

    pub fn format_context_line(
        &self,
        file_path: &Path,
        line_number: usize,
        line_content: &str,
        is_before: bool,
    ) -> String {
        let mut output = String::new();

        // File path (dimmed for context)
        if self.show_filenames {
            let file_str = file_path.display().to_string();
            if self.use_colors {
                output.push_str(&file_str.dimmed().to_string());
            } else {
                output.push_str(&file_str);
            }
            output.push(if is_before { '-' } else { '+' });
        }

        // Line number (dimmed for context)
        if self.show_line_numbers {
            let line_str = line_number.to_string();
            if self.use_colors {
                output.push_str(&line_str.dimmed().to_string());
            } else {
                output.push_str(&line_str);
            }
            output.push(if is_before { '-' } else { '+' });
        }

        // Line content (dimmed for context)
        if self.use_colors {
            output.push_str(&line_content.dimmed().to_string());
        } else {
            output.push_str(line_content);
        }

        output
    }

    pub fn format_separator(&self) -> String {
        if self.use_colors {
            "--".dimmed().to_string()
        } else {
            "--".to_string()
        }
    }

    pub fn format_count(&self, file_path: &Path, count: usize) -> String {
        if self.json_output {
            format!(
                r#"{{"file":"{}","count":{}}}"#,
                file_path.display(),
                count
            )
        } else {
            let mut output = String::new();
            
            if self.show_filenames {
                let file_str = file_path.display().to_string();
                if self.use_colors {
                    output.push_str(&file_str.magenta().bold().to_string());
                } else {
                    output.push_str(&file_str);
                }
                output.push(':');
            }
            
            let count_str = count.to_string();
            if self.use_colors {
                output.push_str(&count_str.yellow().bold().to_string());
            } else {
                output.push_str(&count_str);
            }
            
            output
        }
    }

    pub fn format_filename_only(&self, file_path: &Path) -> String {
        if self.json_output {
            format!(r#"{{"file":"{}"}}"#, file_path.display())
        } else if self.use_colors {
            file_path.display().to_string().magenta().bold().to_string()
        } else {
            file_path.display().to_string()
        }
    }
}

pub struct MatchResult {
    pub file_path: std::path::PathBuf,
    pub line_number: usize,
    pub line_content: String,
    pub match_start: usize,
    pub match_end: usize,
    pub context_before: Vec<(usize, String)>,
    pub context_after: Vec<(usize, String)>,
}

impl MatchResult {
    pub fn new(
        file_path: std::path::PathBuf,
        line_number: usize,
        line_content: String,
        match_start: usize,
        match_end: usize,
    ) -> Self {
        Self {
            file_path,
            line_number,
            line_content,
            match_start,
            match_end,
            context_before: Vec::new(),
            context_after: Vec::new(),
        }
    }

    pub fn add_context_before(&mut self, line_number: usize, content: String) {
        self.context_before.push((line_number, content));
    }

    pub fn add_context_after(&mut self, line_number: usize, content: String) {
        self.context_after.push((line_number, content));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_text_formatting() {
        let formatter = OutputFormatter::new(true, true, false, false, 0, 0);
        let result = formatter.format_match(
            &PathBuf::from("test.txt"),
            42,
            "hello world",
            0,
            5
        );
        assert_eq!(result, "test.txt:42:hello world");
    }

    #[test]
    fn test_json_formatting() {
        let formatter = OutputFormatter::new(true, true, false, true, 0, 0);
        let result = formatter.format_match(
            &PathBuf::from("test.txt"),
            42,
            "hello world",
            0,
            5
        );
        assert!(result.contains(r#""file":"test.txt""#));
        assert!(result.contains(r#""line":42"#));
    }
}