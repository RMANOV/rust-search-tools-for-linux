use crate::errors::{FastGrepError, Result};
use content_inspector::{inspect, ContentType};
use memmap2::Mmap;
use std::fs::File;
use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct FileProcessor {
    max_size_for_mmap: u64,
    use_mmap: bool,
}

impl FileProcessor {
    pub fn new(max_size_for_mmap: u64, use_mmap: bool) -> Self {
        Self {
            max_size_for_mmap,
            use_mmap,
        }
    }

    pub fn process_file<P: AsRef<Path>>(&self, path: P) -> Result<FileContent> {
        let path = path.as_ref();
        let path_buf = path.to_path_buf();
        let metadata = std::fs::metadata(path)
            .map_err(|e| FastGrepError::file_processing(path_buf.clone(), e))?;
        let file_size = metadata.len();

        // Skip binary files with better detection
        if self.is_likely_binary(path).map_err(|e| 
            FastGrepError::content_inspection(path_buf.clone(), e)
        )? {
            return Err(FastGrepError::BinaryFile { path: path_buf });
        }

        // Use memory mapping for large files if enabled
        if self.use_mmap && file_size > self.max_size_for_mmap {
            self.process_with_mmap(path)
        } else {
            self.process_with_read(path)
        }
    }

    fn process_with_mmap<P: AsRef<Path>>(&self, path: P) -> Result<FileContent> {
        let path = path.as_ref();
        let path_buf = path.to_path_buf();
        
        let file = File::open(path)
            .map_err(|e| FastGrepError::file_processing(path_buf.clone(), e))?;
        
        let mmap = unsafe { 
            Mmap::map(&file)
                .map_err(|e| FastGrepError::memory_mapping(path_buf, e))?
        };
        
        Ok(FileContent::Mapped(mmap))
    }

    fn process_with_read<P: AsRef<Path>>(&self, path: P) -> Result<FileContent> {
        let path = path.as_ref();
        let path_buf = path.to_path_buf();
        
        let mut file = File::open(path)
            .map_err(|e| FastGrepError::file_processing(path_buf.clone(), e))?;
        
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| FastGrepError::file_processing(path_buf, e))?;
        
        Ok(FileContent::InMemory(buffer))
    }

    fn is_likely_binary<P: AsRef<Path>>(&self, path: P) -> std::io::Result<bool> {
        let mut file = File::open(path)?;
        let mut buffer = vec![0; 8192]; // Check first 8KB for better accuracy
        let bytes_read = file.read(&mut buffer)?;
        
        if bytes_read == 0 {
            return Ok(false); // Empty files are considered text
        }
        
        // Use content_inspector for more accurate binary detection
        let content_type = inspect(&buffer[..bytes_read]);
        Ok(matches!(content_type, ContentType::BINARY))
    }
}

pub enum FileContent {
    InMemory(Vec<u8>),
    Mapped(Mmap),
    Binary,
}

impl FileContent {
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            FileContent::InMemory(data) => Some(data),
            FileContent::Mapped(mmap) => Some(&mmap[..]),
            FileContent::Binary => None,
        }
    }

    pub fn lines(&self) -> Option<Vec<Line>> {
        let bytes = self.as_bytes()?;
        let mut lines = Vec::new();
        let mut start = 0;
        let mut line_number = 1;

        for (pos, &byte) in bytes.iter().enumerate() {
            if byte == b'\n' {
                lines.push(Line {
                    number: line_number,
                    start,
                    end: pos,
                    content: &bytes[start..pos],
                });
                start = pos + 1;
                line_number += 1;
            }
        }

        // Handle last line if it doesn't end with newline
        if start < bytes.len() {
            lines.push(Line {
                number: line_number,
                start,
                end: bytes.len(),
                content: &bytes[start..],
            });
        }

        Some(lines)
    }
}

#[derive(Debug, Clone)]
pub struct Line<'a> {
    pub number: usize,
    pub start: usize,
    pub end: usize,
    pub content: &'a [u8],
}

impl<'a> Line<'a> {
    pub fn as_str(&self) -> std::result::Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.content)
    }

    pub fn contains_position(&self, pos: usize) -> bool {
        pos >= self.start && pos <= self.end
    }
}

// Optimized line-by-line processor for streaming large files
pub struct LineProcessor<R: BufRead> {
    reader: R,
    line_number: usize,
}

impl<R: BufRead> LineProcessor<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line_number: 0,
        }
    }

    pub fn process_lines<F>(&mut self, mut callback: F) -> Result<()>
    where
        F: FnMut(usize, &[u8]) -> Result<bool>, // line_number, content -> continue?
    {
        let mut buffer = Vec::new();
        
        loop {
            buffer.clear();
            let bytes_read = self.reader.read_until(b'\n', &mut buffer)?;
            
            if bytes_read == 0 {
                break; // EOF
            }
            
            self.line_number += 1;
            
            // Remove trailing newline
            if buffer.last() == Some(&b'\n') {
                buffer.pop();
            }
            
            let should_continue = callback(self.line_number, &buffer)?;
            if !should_continue {
                break;
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_line_processor() {
        let data = b"line1\nline2\nline3";
        let cursor = Cursor::new(data);
        let mut processor = LineProcessor::new(cursor);
        
        let mut lines = Vec::new();
        processor.process_lines(|line_num, content| {
            lines.push((line_num, content.to_vec()));
            Ok(true)
        }).unwrap();
        
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], (1, b"line1".to_vec()));
        assert_eq!(lines[1], (2, b"line2".to_vec()));
        assert_eq!(lines[2], (3, b"line3".to_vec()));
    }
}