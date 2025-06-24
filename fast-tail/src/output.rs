use chrono::{DateTime, Local};
use colored::*;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: Option<DateTime<Local>>,
    pub file: String,
    pub line_number: Option<usize>,
    pub content: String,
    pub matched: bool,
}

impl LogEntry {
    pub fn new(
        file: impl Into<String>,
        content: impl Into<String>,
        line_number: Option<usize>,
        matched: bool,
        add_timestamp: bool,
    ) -> Self {
        Self {
            timestamp: if add_timestamp {
                Some(Local::now())
            } else {
                None
            },
            file: file.into(),
            line_number,
            content: content.into(),
            matched,
        }
    }
}

pub struct OutputFormatter {
    use_colors: bool,
    show_line_numbers: bool,
    show_filenames: bool,
    show_timestamps: bool,
    json_output: bool,
}

impl OutputFormatter {
    pub fn new(
        use_colors: bool,
        show_line_numbers: bool,
        show_filenames: bool,
        show_timestamps: bool,
        json_output: bool,
    ) -> Self {
        Self {
            use_colors,
            show_line_numbers,
            show_filenames,
            show_timestamps,
            json_output,
        }
    }

    pub fn format_entry(&self, entry: &LogEntry) -> String {
        if self.json_output {
            self.format_json(entry)
        } else {
            self.format_text(entry)
        }
    }

    fn format_json(&self, entry: &LogEntry) -> String {
        serde_json::to_string(entry).unwrap_or_else(|_| {
            format!(
                r#"{{"error":"JSON serialization failed","content":"{}"}}"#,
                entry.content.replace('"', r#"\""#)
            )
        })
    }

    fn format_text(&self, entry: &LogEntry) -> String {
        let mut output = String::new();

        // Timestamp
        if self.show_timestamps {
            if let Some(ref timestamp) = entry.timestamp {
                let ts_str = timestamp.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
                if self.use_colors {
                    output.push_str(&ts_str.blue().to_string());
                } else {
                    output.push_str(&ts_str);
                }
                output.push(' ');
            }
        }

        // Filename
        if self.show_filenames {
            let file_str = Path::new(&entry.file)
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_else(|| entry.file.as_str().into());

            if self.use_colors {
                output.push_str(&file_str.magenta().bold().to_string());
            } else {
                output.push_str(&file_str);
            }
            output.push(':');
        }

        // Line number
        if self.show_line_numbers {
            if let Some(line_num) = entry.line_number {
                let line_str = line_num.to_string();
                if self.use_colors {
                    output.push_str(&line_str.green().to_string());
                } else {
                    output.push_str(&line_str);
                }
                output.push(':');
            }
        }

        // Add separator if we have prefixes
        if (self.show_filenames || self.show_line_numbers || self.show_timestamps) 
            && !output.is_empty() {
            output.push(' ');
        }

        // Content
        if entry.matched && self.use_colors {
            // For matched lines, highlight the entire line
            output.push_str(&entry.content.yellow().to_string());
        } else {
            output.push_str(&entry.content);
        }

        output
    }

    pub fn format_file_header(&self, file_path: &Path) -> String {
        if self.json_output {
            format!(
                r#"{{"event":"file_header","file":"{}"}}"#,
                file_path.display()
            )
        } else {
            let header = format!("==> {} <==", file_path.display());
            if self.use_colors {
                header.cyan().bold().to_string()
            } else {
                header
            }
        }
    }

    pub fn format_error(&self, error: &str, file_path: Option<&Path>) -> String {
        if self.json_output {
            format!(
                r#"{{"event":"error","message":"{}","file":"{}"}}"#,
                error.replace('"', r#"\""#),
                file_path.map(|p| p.display().to_string()).unwrap_or_else(|| "unknown".to_string())
            )
        } else {
            let error_msg = if let Some(path) = file_path {
                format!("Error in {}: {}", path.display(), error)
            } else {
                format!("Error: {}", error)
            };

            if self.use_colors {
                error_msg.red().bold().to_string()
            } else {
                error_msg
            }
        }
    }

    pub fn format_info(&self, message: &str) -> String {
        if self.json_output {
            format!(
                r#"{{"event":"info","message":"{}"}}"#,
                message.replace('"', r#"\""#)
            )
        } else if self.use_colors {
            message.bright_blue().to_string()
        } else {
            message.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_text_formatting() {
        let formatter = OutputFormatter::new(false, true, true, false, false);
        let entry = LogEntry::new("test.log", "hello world", Some(42), false, false);
        let result = formatter.format_entry(&entry);
        assert!(result.contains("test.log:42: hello world"));
    }

    #[test]
    fn test_json_formatting() {
        let formatter = OutputFormatter::new(false, true, true, false, true);
        let entry = LogEntry::new("test.log", "hello world", Some(42), false, false);
        let result = formatter.format_entry(&entry);
        assert!(result.contains(r#""file":"test.log""#));
        assert!(result.contains(r#""line_number":42"#));
        assert!(result.contains(r#""content":"hello world""#));
    }
}