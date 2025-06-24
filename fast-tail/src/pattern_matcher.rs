use crate::errors::{FastTailError, Result};
use memchr::memchr;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct PatternMatcher {
    pattern: String,
    regex: Option<Regex>,
    use_regex: bool,
    ignore_case: bool,
    invert_match: bool,
}

impl PatternMatcher {
    pub fn new(
        pattern: &str,
        use_regex: bool,
        ignore_case: bool,
        invert_match: bool,
    ) -> Result<Self> {
        let regex = if use_regex {
            let mut builder = regex::RegexBuilder::new(pattern);
            builder.case_insensitive(ignore_case);
            Some(builder.build().map_err(|e| {
                FastTailError::pattern_compilation(pattern.to_string(), e)
            })?)
        } else {
            None
        };

        Ok(Self {
            pattern: if ignore_case && !use_regex {
                pattern.to_lowercase()
            } else {
                pattern.to_string()
            },
            regex,
            use_regex,
            ignore_case,
            invert_match,
        })
    }

    pub fn matches(&self, line: &str) -> bool {
        let found = if let Some(ref regex) = self.regex {
            regex.is_match(line)
        } else {
            self.literal_match(line)
        };

        if self.invert_match {
            !found
        } else {
            found
        }
    }

    fn literal_match(&self, line: &str) -> bool {
        if self.ignore_case {
            line.to_lowercase().contains(&self.pattern)
        } else {
            // Use SIMD-optimized memchr for the first byte, then verify
            if let Some(first_byte) = self.pattern.as_bytes().first() {
                let line_bytes = line.as_bytes();
                let mut pos = 0;
                
                while let Some(idx) = memchr(*first_byte, &line_bytes[pos..]) {
                    let start = pos + idx;
                    if start + self.pattern.len() <= line_bytes.len() {
                        if &line_bytes[start..start + self.pattern.len()] == self.pattern.as_bytes() {
                            return true;
                        }
                    }
                    pos = start + 1;
                }
                false
            } else {
                false
            }
        }
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn is_regex(&self) -> bool {
        self.use_regex
    }

    pub fn is_inverted(&self) -> bool {
        self.invert_match
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_match() {
        let matcher = PatternMatcher::new("hello", false, false, false).unwrap();
        assert!(matcher.matches("hello world"));
        assert!(!matcher.matches("goodbye world"));
    }

    #[test]
    fn test_case_insensitive() {
        let matcher = PatternMatcher::new("HELLO", false, true, false).unwrap();
        assert!(matcher.matches("hello world"));
        assert!(matcher.matches("Hello World"));
        assert!(matcher.matches("HELLO WORLD"));
    }

    #[test]
    fn test_regex_match() {
        let matcher = PatternMatcher::new(r"\d+", true, false, false).unwrap();
        assert!(matcher.matches("error 404"));
        assert!(!matcher.matches("no numbers here"));
    }

    #[test]
    fn test_invert_match() {
        let matcher = PatternMatcher::new("hello", false, false, true).unwrap();
        assert!(!matcher.matches("hello world"));
        assert!(matcher.matches("goodbye world"));
    }
}