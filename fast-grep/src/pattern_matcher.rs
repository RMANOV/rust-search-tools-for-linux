use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use anyhow::Result;
use memchr::{memchr, memchr_iter};
use regex::bytes::{Regex, RegexBuilder};

pub struct PatternMatcher {
    pattern_string: String,
    use_regex: bool,
    ignore_case: bool,
    matcher: PatternMatcherImpl,
}

enum PatternMatcherImpl {
    /// Single literal string - fastest using memchr SIMD
    SingleLiteral {
        pattern: Vec<u8>,
        ignore_case: bool,
    },
    /// Multiple literal strings - Aho-Corasick algorithm
    MultiLiteral {
        ac: AhoCorasick,
    },
    /// Regular expression - most flexible but slower
    Regex {
        regex: Regex,
    },
}

impl PatternMatcher {
    pub fn new(pattern: &str, use_regex: bool, ignore_case: bool) -> Result<Self> {
        let matcher = if use_regex {
            let regex = RegexBuilder::new(pattern)
                .case_insensitive(ignore_case)
                .multi_line(true)
                .build()?;
            PatternMatcherImpl::Regex { regex }
        } else if pattern.contains('|') {
            // Multiple literal patterns separated by |
            let patterns: Vec<&str> = pattern.split('|').collect();
            let ac = AhoCorasickBuilder::new()
                .ascii_case_insensitive(ignore_case)
                .build(patterns)?;
            PatternMatcherImpl::MultiLiteral { ac }
        } else {
            // Single literal pattern - use SIMD-optimized memchr
            let pattern_bytes = if ignore_case {
                pattern.to_lowercase().into_bytes()
            } else {
                pattern.as_bytes().to_vec()
            };
            PatternMatcherImpl::SingleLiteral {
                pattern: pattern_bytes,
                ignore_case,
            }
        };

        Ok(PatternMatcher {
            pattern_string: pattern.to_string(),
            use_regex,
            ignore_case,
            matcher,
        })
    }

    pub fn find_matches(&self, data: &[u8]) -> Vec<Match> {
        match &self.matcher {
            PatternMatcherImpl::SingleLiteral { pattern, ignore_case } => {
                self.find_single_literal(data, pattern, *ignore_case)
            }
            PatternMatcherImpl::MultiLiteral { ac } => {
                self.find_multi_literal(data, ac)
            }
            PatternMatcherImpl::Regex { regex } => {
                self.find_regex_matches(data, regex)
            }
        }
    }

    fn find_single_literal(&self, data: &[u8], pattern: &[u8], ignore_case: bool) -> Vec<Match> {
        let mut matches = Vec::new();
        
        if pattern.is_empty() {
            return matches;
        }

        let search_data = if ignore_case {
            // For case-insensitive, we need to convert data to lowercase
            // This is expensive, but still faster than regex for simple patterns
            String::from_utf8_lossy(data).to_lowercase().into_bytes()
        } else {
            data.to_vec()
        };

        let search_slice = if ignore_case { &search_data } else { data };
        
        // Use SIMD-optimized memchr for the first byte, then verify full pattern
        let first_byte = pattern[0];
        
        for pos in memchr_iter(first_byte, search_slice) {
            if pos + pattern.len() <= search_slice.len() {
                if &search_slice[pos..pos + pattern.len()] == pattern {
                    matches.push(Match {
                        start: pos,
                        end: pos + pattern.len(),
                        pattern_id: 0,
                    });
                }
            }
        }

        matches
    }

    fn find_multi_literal(&self, data: &[u8], ac: &AhoCorasick) -> Vec<Match> {
        ac.find_iter(data)
            .map(|m| Match {
                start: m.start(),
                end: m.end(),
                pattern_id: m.pattern().as_usize(),
            })
            .collect()
    }

    fn find_regex_matches(&self, data: &[u8], regex: &Regex) -> Vec<Match> {
        regex.find_iter(data)
            .map(|m| Match {
                start: m.start(),
                end: m.end(),
                pattern_id: 0,
            })
            .collect()
    }
}

impl Clone for PatternMatcher {
    fn clone(&self) -> Self {
        // Recreate the matcher from stored parameters
        PatternMatcher::new(&self.pattern_string, self.use_regex, self.ignore_case)
            .expect("Failed to clone PatternMatcher")
    }
}

#[derive(Debug, Clone)]
pub struct Match {
    pub start: usize,
    pub end: usize,
    pub pattern_id: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_literal() {
        let matcher = PatternMatcher::new("hello", false, false).unwrap();
        let data = b"hello world hello rust";
        let matches = matcher.find_matches(data);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].start, 0);
        assert_eq!(matches[1].start, 12);
    }

    #[test]
    fn test_case_insensitive() {
        let matcher = PatternMatcher::new("HELLO", false, true).unwrap();
        let data = b"hello world Hello RUST";
        let matches = matcher.find_matches(data);
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_regex() {
        let matcher = PatternMatcher::new(r"\d+", true, false).unwrap();
        let data = b"file123.txt and file456.txt";
        let matches = matcher.find_matches(data);
        assert_eq!(matches.len(), 2);
    }
}