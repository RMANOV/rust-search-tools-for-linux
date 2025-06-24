use crate::errors::{FastTailError, Result};
use crate::output::LogEntry;
use crate::pattern_matcher::PatternMatcher;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub struct FileState {
    pub path: PathBuf,
    pub position: u64,
    pub size: u64,
    pub line_count: usize,
    pub inode: Option<u64>,
}

impl FileState {
    pub fn new(path: PathBuf) -> Result<Self> {
        let metadata = std::fs::metadata(&path)
            .map_err(|_| FastTailError::file_not_found(path.clone()))?;
        
        #[cfg(unix)]
        let inode = {
            use std::os::unix::fs::MetadataExt;
            Some(metadata.ino())
        };
        
        #[cfg(not(unix))]
        let inode = None;

        Ok(Self {
            path,
            position: metadata.len(),
            size: metadata.len(),
            line_count: 0,
            inode,
        })
    }

    pub fn update_from_metadata(&mut self, metadata: &std::fs::Metadata) {
        self.size = metadata.len();
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            self.inode = Some(metadata.ino());
        }
    }
}

pub struct FileMonitor {
    files: HashMap<PathBuf, FileState>,
    pattern_matcher: Option<PatternMatcher>,
    follow_name: bool,
    buffer_size: usize,
    max_buffer_lines: usize,
    verbose: bool,
}

impl FileMonitor {
    pub fn new(
        pattern_matcher: Option<PatternMatcher>,
        follow_name: bool,
        buffer_size: usize,
        max_buffer_lines: usize,
        verbose: bool,
    ) -> Self {
        Self {
            files: HashMap::new(),
            pattern_matcher,
            follow_name,
            buffer_size,
            max_buffer_lines,
            verbose,
        }
    }

    pub fn add_file(&mut self, path: PathBuf) -> Result<()> {
        let file_state = FileState::new(path.clone())?;
        self.files.insert(path, file_state);
        Ok(())
    }

    pub fn read_initial_lines(&mut self, path: &Path, num_lines: usize) -> Result<Vec<LogEntry>> {
        let file = File::open(path)
            .map_err(|_| FastTailError::file_not_found(path.to_path_buf()))?;
        
        let mut reader = BufReader::with_capacity(self.buffer_size, file);
        let mut lines = Vec::new();
        let mut temp_lines = Vec::new();
        let mut line_number = 1;

        // Read all lines first
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    // Remove trailing newline
                    if line.ends_with('\n') {
                        line.pop();
                        if line.ends_with('\r') {
                            line.pop();
                        }
                    }
                    temp_lines.push((line_number, line));
                    line_number += 1;
                }
                Err(e) => return Err(FastTailError::Io(e)),
            }
        }

        // Take only the last N lines
        let start_idx = if temp_lines.len() > num_lines {
            temp_lines.len() - num_lines
        } else {
            0
        };

        for (line_num, line_content) in temp_lines.into_iter().skip(start_idx) {
            let matches = self.pattern_matcher
                .as_ref()
                .map(|m| m.matches(&line_content))
                .unwrap_or(true);

            if matches {
                lines.push(LogEntry::new(
                    path.display().to_string(),
                    line_content,
                    Some(line_num),
                    self.pattern_matcher.is_some(),
                    false, // No timestamp for initial lines
                ));
            }
        }

        // Update file position
        if let Some(file_state) = self.files.get_mut(path) {
            let position = reader.stream_position().unwrap_or(0);
            file_state.position = position;
            file_state.line_count = line_number - 1;
        }

        Ok(lines)
    }

    pub async fn start_monitoring(
        &mut self,
        tx: tokio_mpsc::UnboundedSender<LogEntry>,
        poll_interval: Duration,
    ) -> Result<()> {
        let paths: Vec<PathBuf> = self.files.keys().cloned().collect();
        
        // Try to use inotify first, fall back to polling
        if let Ok(watcher_tx) = self.setup_inotify_watcher(&paths).await {
            self.run_inotify_monitor(tx, watcher_tx, poll_interval).await
        } else {
            if self.verbose {
                eprintln!("inotify failed, falling back to polling");
            }
            self.run_polling_monitor(tx, poll_interval).await
        }
    }

    async fn setup_inotify_watcher(&self, paths: &[PathBuf]) -> Result<mpsc::Receiver<notify::Result<Event>>> {
        let (watcher_tx, watcher_rx) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(watcher_tx, notify::Config::default())
            .map_err(FastTailError::WatchError)?;

        for path in paths {
            watcher.watch(path, RecursiveMode::NonRecursive)
                .map_err(FastTailError::WatchError)?;
        }

        // Keep watcher alive by moving it to a blocking task
        tokio::task::spawn_blocking(move || {
            let _watcher = watcher;
            // Keep the watcher alive
            loop {
                std::thread::sleep(Duration::from_secs(1));
            }
        });

        Ok(watcher_rx)
    }

    async fn run_inotify_monitor(
        &mut self,
        tx: tokio_mpsc::UnboundedSender<LogEntry>,
        watcher_rx: mpsc::Receiver<notify::Result<Event>>,
        poll_interval: Duration,
    ) -> Result<()> {
        let mut last_poll = tokio::time::Instant::now();

        loop {
            // Check for inotify events (non-blocking)
            match watcher_rx.try_recv() {
                Ok(Ok(event)) => {
                    if let Err(e) = self.handle_inotify_event(event, &tx).await {
                        if self.verbose {
                            eprintln!("Error handling inotify event: {}", e);
                        }
                    }
                }
                Ok(Err(e)) => {
                    if self.verbose {
                        eprintln!("Inotify error: {}", e);
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // No events, continue
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(FastTailError::WatchError(notify::Error::generic("Watcher disconnected")));
                }
            }

            // Also poll periodically as a fallback
            if last_poll.elapsed() >= poll_interval {
                self.poll_files(&tx).await?;
                last_poll = tokio::time::Instant::now();
            }

            sleep(Duration::from_millis(10)).await;
        }
    }

    async fn run_polling_monitor(
        &mut self,
        tx: tokio_mpsc::UnboundedSender<LogEntry>,
        poll_interval: Duration,
    ) -> Result<()> {
        loop {
            self.poll_files(&tx).await?;
            sleep(poll_interval).await;
        }
    }

    async fn handle_inotify_event(
        &mut self,
        event: Event,
        tx: &tokio_mpsc::UnboundedSender<LogEntry>,
    ) -> Result<()> {
        match event.kind {
            EventKind::Modify(_) | EventKind::Create(_) => {
                for path in event.paths {
                    if self.files.contains_key(&path) {
                        self.check_file_changes(&path, tx).await?;
                    }
                }
            }
            EventKind::Remove(_) => {
                for path in event.paths {
                    if self.follow_name && self.files.contains_key(&path) {
                        if self.verbose {
                            eprintln!("File {} was removed, watching for recreation", path.display());
                        }
                        // Reset file state but keep monitoring
                        if let Some(file_state) = self.files.get_mut(&path) {
                            file_state.position = 0;
                            file_state.size = 0;
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn poll_files(&mut self, tx: &tokio_mpsc::UnboundedSender<LogEntry>) -> Result<()> {
        let paths: Vec<PathBuf> = self.files.keys().cloned().collect();
        for path in paths {
            if let Err(e) = self.check_file_changes(&path, tx).await {
                if self.verbose {
                    eprintln!("Error checking file {}: {}", path.display(), e);
                }
            }
        }
        Ok(())
    }

    async fn check_file_changes(
        &mut self,
        path: &PathBuf,
        tx: &tokio_mpsc::UnboundedSender<LogEntry>,
    ) -> Result<()> {
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => {
                // File doesn't exist, check if we should wait for it
                if self.follow_name {
                    return Ok(());
                } else {
                    return Err(FastTailError::file_not_found(path.clone()));
                }
            }
        };

        let file_state = self.files.get_mut(path).unwrap();
        let current_size = metadata.len();

        // Check for file rotation (inode change or size decrease)
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let current_inode = metadata.ino();
            if let Some(old_inode) = file_state.inode {
                if current_inode != old_inode {
                    if self.follow_name {
                        if self.verbose {
                            eprintln!("File rotation detected for {}, resetting position", path.display());
                        }
                        file_state.position = 0;
                        file_state.line_count = 0;
                    } else {
                        return Err(FastTailError::file_rotation_detected(path.clone()));
                    }
                }
            }
        }

        // Check for truncation
        if current_size < file_state.size {
            if self.verbose {
                eprintln!("File {} was truncated, resetting position", path.display());
            }
            file_state.position = 0;
            file_state.line_count = 0;
        }

        file_state.update_from_metadata(&metadata);

        // Read new content if file has grown
        if current_size > file_state.position {
            self.read_new_lines(path, tx).await?;
        }

        Ok(())
    }

    async fn read_new_lines(
        &mut self,
        path: &PathBuf,
        tx: &tokio_mpsc::UnboundedSender<LogEntry>,
    ) -> Result<()> {
        let mut file = File::open(path)
            .map_err(|_| FastTailError::file_not_found(path.clone()))?;
        
        let file_state = self.files.get_mut(path).unwrap();
        file.seek(SeekFrom::Start(file_state.position))?;
        
        let mut reader = BufReader::with_capacity(self.buffer_size, file);
        let mut line_count = 0;

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    // Remove trailing newline
                    if line.ends_with('\n') {
                        line.pop();
                        if line.ends_with('\r') {
                            line.pop();
                        }
                    }

                    file_state.line_count += 1;
                    line_count += 1;

                    let matches = self.pattern_matcher
                        .as_ref()
                        .map(|m| m.matches(&line))
                        .unwrap_or(true);

                    if matches {
                        let entry = LogEntry::new(
                            path.display().to_string(),
                            line,
                            Some(file_state.line_count),
                            self.pattern_matcher.is_some(),
                            true, // Add timestamp for new lines
                        );

                        if tx.send(entry).is_err() {
                            break; // Receiver closed
                        }
                    }

                    // Prevent memory exhaustion
                    if line_count > self.max_buffer_lines {
                        return Err(FastTailError::buffer_overflow(line_count));
                    }
                }
                Err(e) => return Err(FastTailError::Io(e)),
            }
        }

        // Update position
        file_state.position = reader.stream_position().unwrap_or(file_state.position);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_file_state_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_state = FileState::new(temp_file.path().to_path_buf()).unwrap();
        assert_eq!(file_state.path, temp_file.path());
        assert_eq!(file_state.position, 0);
    }

    #[tokio::test]
    async fn test_read_initial_lines() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "line 1").unwrap();
        writeln!(temp_file, "line 2").unwrap();
        writeln!(temp_file, "line 3").unwrap();
        temp_file.flush().unwrap();

        let mut monitor = FileMonitor::new(None, false, 8192, 10000, false);
        monitor.add_file(temp_file.path().to_path_buf()).unwrap();
        
        let lines = monitor.read_initial_lines(temp_file.path(), 2).unwrap();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].content.contains("line 2"));
        assert!(lines[1].content.contains("line 3"));
    }
}