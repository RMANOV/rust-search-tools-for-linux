use anyhow::Result;
use colored::{ColoredString, Colorize};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::file_walker::WalkStats;

#[derive(Debug, Clone)]
pub struct OutputFormatter {
    use_colors: bool,
    long_format: bool,
    print0: bool,
    json_output: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub file_type: String,
    pub size: Option<u64>,
    pub modified: Option<String>,
    pub permissions: Option<String>,
    pub depth: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResults {
    pub files: Vec<FileInfo>,
    pub stats: SearchStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchStats {
    pub total_found: usize,
    pub files_visited: usize,
    pub dirs_visited: usize,
    pub processing_time_ms: u64,
}

impl OutputFormatter {
    pub fn new(use_colors: bool, long_format: bool, print0: bool, json_output: bool) -> Self {
        Self {
            use_colors: use_colors && !json_output, // No colors in JSON mode
            long_format,
            print0,
            json_output,
        }
    }

    pub fn format_path(&self, path: &Path, metadata: Option<&fs::Metadata>, depth: usize) -> Result<String> {
        if self.json_output {
            return Ok(String::new()); // JSON output handled separately
        }

        let path_str = path.to_string_lossy();
        
        if self.print0 {
            return Ok(format!("{}\0", path_str));
        }

        if !self.long_format {
            return Ok(if self.use_colors {
                self.colorize_path(path, metadata).to_string()
            } else {
                path_str.to_string()
            });
        }

        // Long format: permissions size modified_time path
        let mut output = String::new();
        
        if let Some(md) = metadata {
            // Permissions
            let perms = format_permissions(md);
            output.push_str(&perms);
            output.push(' ');
            
            // Size (right-aligned in 10 chars)
            let size = if md.is_dir() {
                "     <DIR>".to_string()
            } else {
                format!("{:>10}", format_size(md.len()))
            };
            output.push_str(&size);
            output.push(' ');
            
            // Modified time
            if let Ok(modified) = md.modified() {
                let formatted_time = format_time(modified);
                output.push_str(&formatted_time);
            } else {
                output.push_str("                   "); // 19 spaces for missing time
            }
            output.push(' ');
        }

        // Path with colors
        let colored_path = if self.use_colors {
            self.colorize_path(path, metadata)
        } else {
            ColoredString::from(path_str.as_ref())
        };
        
        output.push_str(&colored_path.to_string());
        Ok(output)
    }

    pub fn format_json_results(&self, file_infos: Vec<FileInfo>, stats: SearchStats) -> Result<String> {
        let results = SearchResults {
            files: file_infos,
            stats,
        };
        
        serde_json::to_string_pretty(&results)
            .map_err(|e| anyhow::anyhow!("Failed to serialize JSON: {}", e))
    }

    pub fn format_count(&self, count: usize) -> String {
        if self.json_output {
            format!(r#"{{"count": {}}}"#, count)
        } else {
            count.to_string()
        }
    }

    pub fn format_stats(&self, stats: &SearchStats) -> String {
        if self.json_output {
            serde_json::to_string_pretty(stats).unwrap_or_default()
        } else {
            format!(
                "Search completed:\n  Files found: {}\n  Files visited: {}\n  Directories visited: {}\n  Processing time: {}ms",
                stats.total_found,
                stats.files_visited,
                stats.dirs_visited,
                stats.processing_time_ms
            )
        }
    }

    fn colorize_path(&self, path: &Path, metadata: Option<&fs::Metadata>) -> ColoredString {
        let path_str = path.to_string_lossy();
        
        // Determine file type for coloring
        if let Some(md) = metadata {
            if md.is_dir() {
                return path_str.blue().bold();
            }
            
            if md.file_type().is_symlink() {
                return path_str.cyan();
            }
            
            // Check if executable (Unix-like systems)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if md.permissions().mode() & 0o111 != 0 {
                    return path_str.green().bold();
                }
            }
        }

        // Color by extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                // Source code
                "rs" | "py" | "js" | "ts" | "go" | "c" | "cpp" | "h" | "hpp" | "java" | "kt" => {
                    path_str.green()
                }
                // Config files
                "json" | "yaml" | "yml" | "toml" | "xml" | "ini" | "conf" => {
                    path_str.yellow()
                }
                // Documentation
                "md" | "txt" | "rst" | "tex" => {
                    path_str.white()
                }
                // Archives
                "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => {
                    path_str.red()
                }
                // Images
                "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" => {
                    path_str.magenta()
                }
                // Audio/Video
                "mp3" | "wav" | "ogg" | "mp4" | "avi" | "mkv" | "webm" => {
                    path_str.magenta().bold()
                }
                _ => path_str.normal(),
            }
        } else {
            path_str.normal()
        }
    }

    pub fn create_file_info(&self, path: &Path, metadata: &fs::Metadata, depth: usize) -> FileInfo {
        let file_type = if metadata.is_dir() {
            "directory"
        } else if metadata.file_type().is_symlink() {
            "symlink"
        } else {
            "file"
        };

        let modified = metadata.modified()
            .ok()
            .and_then(|time| format_time_iso(time).ok());

        let permissions = Some(format_permissions(metadata));

        FileInfo {
            path: path.to_string_lossy().to_string(),
            file_type: file_type.to_string(),
            size: if metadata.is_file() { Some(metadata.len()) } else { None },
            modified,
            permissions,
            depth,
        }
    }
}

fn format_permissions(metadata: &fs::Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        
        let mut perms = String::with_capacity(10);
        
        // File type
        if metadata.is_dir() {
            perms.push('d');
        } else if metadata.file_type().is_symlink() {
            perms.push('l');
        } else {
            perms.push('-');
        }
        
        // Owner permissions
        perms.push(if mode & 0o400 != 0 { 'r' } else { '-' });
        perms.push(if mode & 0o200 != 0 { 'w' } else { '-' });
        perms.push(if mode & 0o100 != 0 { 'x' } else { '-' });
        
        // Group permissions
        perms.push(if mode & 0o040 != 0 { 'r' } else { '-' });
        perms.push(if mode & 0o020 != 0 { 'w' } else { '-' });
        perms.push(if mode & 0o010 != 0 { 'x' } else { '-' });
        
        // Other permissions
        perms.push(if mode & 0o004 != 0 { 'r' } else { '-' });
        perms.push(if mode & 0o002 != 0 { 'w' } else { '-' });
        perms.push(if mode & 0o001 != 0 { 'x' } else { '-' });
        
        perms
    }
    
    #[cfg(not(unix))]
    {
        let readonly = metadata.permissions().readonly();
        if metadata.is_dir() {
            if readonly { "dr-xr-xr-x".to_string() } else { "drwxrwxrwx".to_string() }
        } else {
            if readonly { "-r--r--r--".to_string() } else { "-rw-rw-rw-".to_string() }
        }
    }
}

fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "K", "M", "G", "T"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{:.0}{}", size, UNITS[unit_index])
    } else {
        format!("{:.1}{}", size, UNITS[unit_index])
    }
}

fn format_time(time: SystemTime) -> String {
    match time.duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => {
            let secs = duration.as_secs();
            let dt = chrono::NaiveDateTime::from_timestamp(secs as i64, 0);
            if let Some(dt) = dt {
                dt.format("%Y-%m-%d %H:%M:%S").to_string()
            } else {
                "????-??-?? ??:??:??".to_string()
            }
        }
        Err(_) => "????-??-?? ??:??:??".to_string(),
    }
}

fn format_time_iso(time: SystemTime) -> Result<String> {
    let duration = time.duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("Time conversion error: {}", e))?;
    
    let secs = duration.as_secs();
    let dt = chrono::NaiveDateTime::from_timestamp(secs as i64, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid timestamp"))?;
    
    Ok(dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::TempDir;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(512), "512B");
        assert_eq!(format_size(1024), "1.0K");
        assert_eq!(format_size(1536), "1.5K");
        assert_eq!(format_size(1048576), "1.0M");
        assert_eq!(format_size(1073741824), "1.0G");
    }

    #[test]
    fn test_format_path_simple() {
        let formatter = OutputFormatter::new(false, false, false, false);
        let path = Path::new("test.txt");
        let result = formatter.format_path(path, None, 0).unwrap();
        assert_eq!(result, "test.txt");
    }

    #[test]
    fn test_format_path_print0() {
        let formatter = OutputFormatter::new(false, false, true, false);
        let path = Path::new("test.txt");
        let result = formatter.format_path(path, None, 0).unwrap();
        assert_eq!(result, "test.txt\0");
    }

    #[test]
    fn test_json_output() {
        let formatter = OutputFormatter::new(false, false, false, true);
        let file_info = FileInfo {
            path: "test.txt".to_string(),
            file_type: "file".to_string(),
            size: Some(1024),
            modified: Some("2023-01-01T12:00:00Z".to_string()),
            permissions: Some("-rw-r--r--".to_string()),
            depth: 1,
        };
        
        let stats = SearchStats {
            total_found: 1,
            files_visited: 1,
            dirs_visited: 0,
            processing_time_ms: 10,
        };
        
        let result = formatter.format_json_results(vec![file_info], stats);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("test.txt"));
    }
}