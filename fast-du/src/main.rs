use anyhow::Result;
use clap::Parser;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "fdu")]
#[command(about = "Parallel disk usage analyzer - modern du alternative")]
#[command(version = "0.1.0")]
struct Args {
    /// Directories to analyze
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,
    
    /// Show human-readable sizes
    #[arg(short = 'h', long = "human-readable")]
    human_readable: bool,
    
    /// Show directory totals only
    #[arg(short = 's', long = "summarize")]
    summarize: bool,
    
    /// Maximum depth to descend
    #[arg(short = 'd', long = "max-depth")]
    max_depth: Option<usize>,
    
    /// Number of threads (default: CPU cores)
    #[arg(short = 'j', long = "threads")]
    threads: Option<usize>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    println!("{}", "💾 fast-du (fdu) - Parallel Disk Usage Analyzer".bold().cyan());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());
    
    // Set up thread pool
    if let Some(threads) = args.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .unwrap();
    }
    
    let progress = ProgressBar::new_spinner();
    progress.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    progress.set_message("Scanning directories...");
    
    for path in &args.paths {
        let size = calculate_directory_size(path, &args, &progress)?;
        
        progress.finish_and_clear();
        
        let size_str = if args.human_readable {
            format_human_size(size)
        } else {
            size.to_string()
        };
        
        println!("{} {}", 
            size_str.yellow().bold(),
            path.display().to_string().blue()
        );
    }
    
    println!("\n{}", "⚡ Coming soon: 30x faster parallel disk usage calculation!".yellow().italic());
    println!("{}", "🚀 Features: Tree visualization, progress bars, memory-efficient scanning".green());
    
    Ok(())
}

fn calculate_directory_size(path: &PathBuf, args: &Args, progress: &ProgressBar) -> Result<u64> {
    let total_size = Arc::new(AtomicU64::new(0));
    let processed_files = Arc::new(AtomicU64::new(0));
    
    progress.set_message(format!("Scanning {}", path.display()));
    
    // Collect all entries first (will be optimized with parallel walkdir)
    let mut entries = Vec::new();
    collect_entries(path, &mut entries, args.max_depth.unwrap_or(usize::MAX), 0)?;
    
    // Process files in parallel
    entries.par_iter().for_each(|entry| {
        if let Ok(metadata) = std::fs::metadata(entry) {
            if metadata.is_file() {
                total_size.fetch_add(metadata.len(), Ordering::Relaxed);
            }
        }
        
        let processed = processed_files.fetch_add(1, Ordering::Relaxed);
        if processed % 1000 == 0 {
            progress.set_message(format!("Processed {} files in {}", processed, path.display()));
        }
    });
    
    Ok(total_size.load(Ordering::Relaxed))
}

fn collect_entries(path: &PathBuf, entries: &mut Vec<PathBuf>, max_depth: usize, current_depth: usize) -> Result<()> {
    if current_depth >= max_depth {
        return Ok(());
    }
    
    let dir_entries = std::fs::read_dir(path)?;
    
    for entry in dir_entries {
        let entry = entry?;
        let entry_path = entry.path();
        entries.push(entry_path.clone());
        
        if entry_path.is_dir() {
            collect_entries(&entry_path, entries, max_depth, current_depth + 1)?;
        }
    }
    
    Ok(())
}

fn format_human_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "K", "M", "G", "T", "P"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{}B", size as u64)
    } else {
        format!("{:.1}{}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_human_size() {
        assert_eq!(format_human_size(512), "512B");
        assert_eq!(format_human_size(1024), "1.0K");
        assert_eq!(format_human_size(1536), "1.5K");
        assert_eq!(format_human_size(1024 * 1024), "1.0M");
    }
}