use anyhow::{anyhow, Result};
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::cli::Args;

#[derive(Clone, Debug)]
pub struct PatternMatcher {
    // Name patterns
    name_pattern: Option<GlobPattern>,
    iname_pattern: Option<GlobPattern>,
    path_pattern: Option<GlobPattern>,
    ipath_pattern: Option<GlobPattern>,
    
    // File type filters
    file_types: Option<HashSet<FileType>>,
    allowed_extensions: Option<HashSet<String>>,
    excluded_extensions: Option<HashSet<String>>,
    
    // Size filters
    size_filter: Option<SizeFilter>,
    empty_only: bool,
    
    // Time filters
    mtime_filter: Option<TimeFilter>,
    atime_filter: Option<TimeFilter>,
    ctime_filter: Option<TimeFilter>,
    newer_than: Option<SystemTime>,
}

#[derive(Clone, Debug)]
struct GlobPattern {
    pattern: String,
    regex: Regex,
    case_sensitive: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FileType {
    File,
    Directory,
    Symlink,
}

#[derive(Clone, Debug)]
pub struct SizeFilter {
    operator: ComparisonOp,
    size_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct TimeFilter {
    operator: ComparisonOp,
    days: u32,
}

#[derive(Clone, Debug, PartialEq)]
enum ComparisonOp {
    Equal,
    Greater,
    Less,
}

impl PatternMatcher {
    pub fn new(args: &Args) -> Result<Self> {
        let mut matcher = Self {
            name_pattern: None,
            iname_pattern: None,
            path_pattern: None,
            ipath_pattern: None,
            file_types: None,
            allowed_extensions: None,
            excluded_extensions: None,
            size_filter: None,
            empty_only: args.empty,
            mtime_filter: None,
            atime_filter: None,
            ctime_filter: None,
            newer_than: None,
        };

        // Parse name patterns
        if let Some(ref pattern) = args.name {
            matcher.name_pattern = Some(GlobPattern::new(pattern, true, args.use_regex)?);
        }
        if let Some(ref pattern) = args.iname {
            matcher.iname_pattern = Some(GlobPattern::new(pattern, false, args.use_regex)?);
        }
        if let Some(ref pattern) = args.path {
            matcher.path_pattern = Some(GlobPattern::new(pattern, true, args.use_regex)?);
        }
        if let Some(ref pattern) = args.ipath {
            matcher.ipath_pattern = Some(GlobPattern::new(pattern, false, args.use_regex)?);
        }

        // Parse file type filters
        if let Some(ref ft) = args.file_type {
            matcher.file_types = Some(parse_file_types(ft)?);
        }

        // Parse extension filters
        if let Some(ref exts) = args.extensions {
            matcher.allowed_extensions = Some(parse_extensions(exts));
        }
        if let Some(ref exts) = args.exclude_extensions {
            matcher.excluded_extensions = Some(parse_extensions(exts));
        }

        // Parse size filter
        if let Some(ref size) = args.size {
            matcher.size_filter = Some(SizeFilter::parse(size)?);
        }

        // Parse time filters
        if let Some(ref mtime) = args.mtime {
            matcher.mtime_filter = Some(TimeFilter::parse(mtime)?);
        }
        if let Some(ref atime) = args.atime {
            matcher.atime_filter = Some(TimeFilter::parse(atime)?);
        }
        if let Some(ref ctime) = args.ctime {
            matcher.ctime_filter = Some(TimeFilter::parse(ctime)?);
        }

        // Parse newer reference
        if let Some(ref newer_path) = args.newer {
            matcher.newer_than = Some(get_modification_time(newer_path)?);
        }

        Ok(matcher)
    }

    pub fn matches(&self, path: &Path, metadata: &fs::Metadata) -> Result<bool> {
        // Check name patterns
        if let Some(ref pattern) = &self.name_pattern {
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if !pattern.matches(filename) {
                return Ok(false);
            }
        }

        if let Some(ref pattern) = &self.iname_pattern {
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if !pattern.matches(filename) {
                return Ok(false);
            }
        }

        // Check path patterns
        if let Some(ref pattern) = &self.path_pattern {
            let path_str = path.to_string_lossy();
            if !pattern.matches(&path_str) {
                return Ok(false);
            }
        }

        if let Some(ref pattern) = &self.ipath_pattern {
            let path_str = path.to_string_lossy();
            if !pattern.matches(&path_str) {
                return Ok(false);
            }
        }

        // Check file type
        if let Some(ref types) = &self.file_types {
            let file_type = if metadata.is_file() {
                FileType::File
            } else if metadata.is_dir() {
                FileType::Directory
            } else {
                FileType::Symlink
            };
            
            if !types.contains(&file_type) {
                return Ok(false);
            }
        }

        // Check extensions
        if let Some(ref allowed) = &self.allowed_extensions {
            let ext = path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();
            if !allowed.contains(&ext) {
                return Ok(false);
            }
        }

        if let Some(ref excluded) = &self.excluded_extensions {
            let ext = path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();
            if excluded.contains(&ext) {
                return Ok(false);
            }
        }

        // Check size
        if let Some(ref filter) = &self.size_filter {
            if !filter.matches(metadata.len()) {
                return Ok(false);
            }
        }

        // Check empty files/directories
        if self.empty_only {
            if metadata.is_file() && metadata.len() > 0 {
                return Ok(false);
            }
            if metadata.is_dir() {
                match fs::read_dir(path) {
                    Ok(mut entries) => {
                        if entries.next().is_some() {
                            return Ok(false);
                        }
                    }
                    Err(_) => return Ok(false),
                }
            }
        }

        // Check modification time
        if let Some(ref filter) = &self.mtime_filter {
            let mtime = metadata.modified().map_err(|e| anyhow!("Failed to get mtime: {}", e))?;
            if !filter.matches(mtime)? {
                return Ok(false);
            }
        }

        // Check access time (if available)
        if let Some(ref filter) = &self.atime_filter {
            let atime = metadata.accessed().map_err(|e| anyhow!("Failed to get atime: {}", e))?;
            if !filter.matches(atime)? {
                return Ok(false);
            }
        }

        // Check creation/change time (limited platform support)
        if let Some(ref filter) = &self.ctime_filter {
            let ctime = metadata.created().or_else(|_| metadata.modified())
                .map_err(|e| anyhow!("Failed to get ctime: {}", e))?;
            if !filter.matches(ctime)? {
                return Ok(false);
            }
        }

        // Check newer than reference
        if let Some(ref reference_time) = &self.newer_than {
            let mtime = metadata.modified().map_err(|e| anyhow!("Failed to get mtime: {}", e))?;
            if mtime <= *reference_time {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

impl GlobPattern {
    fn new(pattern: &str, case_sensitive: bool, use_regex: bool) -> Result<Self> {
        let regex_pattern = if use_regex {
            if case_sensitive {
                pattern.to_string()
            } else {
                format!("(?i){}", pattern)
            }
        } else {
            // Convert glob to regex
            let mut regex_pattern = String::new();
            if !case_sensitive {
                regex_pattern.push_str("(?i)");
            }
            regex_pattern.push('^');
            
            for ch in pattern.chars() {
                match ch {
                    '*' => regex_pattern.push_str(".*"),
                    '?' => regex_pattern.push('.'),
                    '[' => regex_pattern.push('['),
                    ']' => regex_pattern.push(']'),
                    '^' | '$' | '.' | '\\' | '|' | '+' | '(' | ')' | '{' | '}' => {
                        regex_pattern.push('\\');
                        regex_pattern.push(ch);
                    }
                    _ => regex_pattern.push(ch),
                }
            }
            regex_pattern.push('$');
            regex_pattern
        };

        let regex = Regex::new(&regex_pattern)
            .map_err(|e| anyhow!("Invalid pattern '{}': {}", pattern, e))?;

        Ok(Self {
            pattern: pattern.to_string(),
            regex,
            case_sensitive,
        })
    }

    fn matches(&self, text: &str) -> bool {
        self.regex.is_match(text)
    }
}

impl SizeFilter {
    fn parse(spec: &str) -> Result<Self> {
        if spec.is_empty() {
            return Err(anyhow!("Empty size specification"));
        }

        let (operator, rest) = if let Some(rest) = spec.strip_prefix('+') {
            (ComparisonOp::Greater, rest)
        } else if let Some(rest) = spec.strip_prefix('-') {
            (ComparisonOp::Less, rest)
        } else if let Some(rest) = spec.strip_prefix('=') {
            (ComparisonOp::Equal, rest)
        } else {
            (ComparisonOp::Equal, spec)
        };

        if rest.is_empty() {
            return Err(anyhow!("Missing size value"));
        }

        let (number_str, suffix) = match rest.find(|c: char| c.is_alphabetic()) {
            Some(pos) => (&rest[..pos], &rest[pos..]),
            None => (rest, ""),
        };

        let number: u64 = number_str.parse()
            .map_err(|_| anyhow!("Invalid size number: {}", number_str))?;

        let multiplier = match suffix {
            "" | "c" => 1,
            "b" => 512,
            "k" => 1024,
            "M" => 1024 * 1024,
            "G" => 1024 * 1024 * 1024,
            "T" => 1024_u64.pow(4),
            _ => return Err(anyhow!("Invalid size suffix: {}", suffix)),
        };

        let size_bytes = number.checked_mul(multiplier)
            .ok_or_else(|| anyhow!("Size value too large"))?;

        Ok(Self { operator, size_bytes })
    }

    fn matches(&self, file_size: u64) -> bool {
        match self.operator {
            ComparisonOp::Equal => file_size == self.size_bytes,
            ComparisonOp::Greater => file_size > self.size_bytes,
            ComparisonOp::Less => file_size < self.size_bytes,
        }
    }
}

impl TimeFilter {
    fn parse(spec: &str) -> Result<Self> {
        if spec.is_empty() {
            return Err(anyhow!("Empty time specification"));
        }

        let (operator, rest) = if let Some(rest) = spec.strip_prefix('+') {
            (ComparisonOp::Greater, rest)
        } else if let Some(rest) = spec.strip_prefix('-') {
            (ComparisonOp::Less, rest)
        } else if let Some(rest) = spec.strip_prefix('=') {
            (ComparisonOp::Equal, rest)
        } else {
            (ComparisonOp::Equal, spec)
        };

        let days: u32 = rest.parse()
            .map_err(|_| anyhow!("Invalid time value: {}", rest))?;

        Ok(Self { operator, days })
    }

    fn matches(&self, file_time: SystemTime) -> Result<bool> {
        let now = SystemTime::now();
        let file_age = now.duration_since(file_time)
            .unwrap_or(Duration::from_secs(0));
        let file_age_days = file_age.as_secs() / (24 * 60 * 60);

        let matches = match self.operator {
            ComparisonOp::Equal => file_age_days == self.days as u64,
            ComparisonOp::Greater => file_age_days > self.days as u64,
            ComparisonOp::Less => file_age_days < self.days as u64,
        };

        Ok(matches)
    }
}

fn parse_file_types(spec: &str) -> Result<HashSet<FileType>> {
    let mut types = HashSet::new();
    
    match spec {
        "f" | "file" => { types.insert(FileType::File); }
        "d" | "dir" | "directory" => { types.insert(FileType::Directory); }
        "l" | "symlink" => { types.insert(FileType::Symlink); }
        _ => return Err(anyhow!("Invalid file type: {}", spec)),
    }
    
    Ok(types)
}

fn parse_extensions(spec: &str) -> HashSet<String> {
    spec.split(',')
        .map(|ext| ext.trim().to_lowercase())
        .filter(|ext| !ext.is_empty())
        .collect()
}

fn get_modification_time(path: &Path) -> Result<SystemTime> {
    let metadata = fs::metadata(path)
        .map_err(|e| anyhow!("Failed to get metadata for {}: {}", path.display(), e))?;
    
    metadata.modified()
        .map_err(|e| anyhow!("Failed to get modification time for {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_glob_pattern() {
        // Case sensitive: *.rs matches files ending in .rs (lowercase extension)
        let pattern = GlobPattern::new("*.rs", true, false).unwrap();
        assert!(pattern.matches("main.rs"));
        assert!(pattern.matches("lib.rs"));
        assert!(pattern.matches("Main.rs"));  // Name can be any case, extension must be .rs
        assert!(!pattern.matches("main.py"));
        assert!(!pattern.matches("main.RS")); // Extension .RS != .rs (case sensitive)
        assert!(!pattern.matches("MAIN.RS")); // Extension .RS != .rs (case sensitive)

        // Case insensitive: *.RS matches any case of extension
        let pattern = GlobPattern::new("*.RS", false, false).unwrap();
        assert!(pattern.matches("main.rs"));
        assert!(pattern.matches("Main.RS"));
        assert!(pattern.matches("MAIN.rs"));
    }

    #[test]
    fn test_size_filter() {
        let filter = SizeFilter::parse("+100k").unwrap();
        assert!(!filter.matches(50 * 1024));
        assert!(filter.matches(200 * 1024));

        let filter = SizeFilter::parse("-1M").unwrap();
        assert!(filter.matches(500 * 1024));
        assert!(!filter.matches(2 * 1024 * 1024));

        let filter = SizeFilter::parse("=1024").unwrap();
        assert!(filter.matches(1024));
        assert!(!filter.matches(1025));
    }

    #[test]
    fn test_extensions() {
        let exts = parse_extensions("rs,py,js");
        assert!(exts.contains("rs"));
        assert!(exts.contains("py"));
        assert!(exts.contains("js"));
        assert!(!exts.contains("txt"));
    }
}