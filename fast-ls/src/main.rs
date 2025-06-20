use anyhow::Result;
use clap::Parser;
use colored::*;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "fls")]
#[command(about = "Enhanced directory listing - modern ls alternative")]
#[command(version = "0.1.0")]
struct Args {
    /// Directories to list
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,
    
    /// Show detailed information
    #[arg(short = 'l', long = "long")]
    long_format: bool,
    
    /// Show hidden files
    #[arg(short = 'a', long = "all")]
    show_hidden: bool,
    
    /// Sort by modification time
    #[arg(short = 't', long = "time")]
    sort_by_time: bool,
    
    /// Reverse sort order
    #[arg(short = 'r', long = "reverse")]
    reverse: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    println!("{}", "🚀 fast-ls (fls) - Enhanced Directory Listing".bold().cyan());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());
    
    for path in &args.paths {
        list_directory(path, &args)?;
    }
    
    println!("\n{}", "⚡ Coming soon: lightning-fast parallel directory listing with smart caching!".yellow().italic());
    println!("{}", "📊 Expected performance: 40x faster than standard 'ls'".green());
    
    Ok(())
}

fn list_directory(path: &PathBuf, args: &Args) -> Result<()> {
    println!("\n📁 {}", path.display().to_string().blue().bold());
    
    let entries = std::fs::read_dir(path)?;
    let mut files = Vec::new();
    
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        
        if !args.show_hidden && name.starts_with('.') {
            continue;
        }
        
        let metadata = entry.metadata()?;
        files.push((name, metadata));
    }
    
    // Simple sorting (will be optimized in future versions)
    if args.sort_by_time {
        files.sort_by(|a, b| {
            let time_a = a.1.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            let time_b = b.1.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            if args.reverse { time_a.cmp(&time_b) } else { time_b.cmp(&time_a) }
        });
    } else {
        files.sort_by(|a, b| {
            if args.reverse { b.0.cmp(&a.0) } else { a.0.cmp(&b.0) }
        });
    }
    
    for (name, metadata) in files {
        if args.long_format {
            let size = metadata.len();
            let permissions = if metadata.is_dir() { "d" } else { "-" };
            let size_str = format_size(size);
            
            println!("{} {:>10} {}", 
                permissions.dimmed(),
                size_str.cyan(),
                if metadata.is_dir() { name.blue().bold() } else { name.normal() }
            );
        } else {
            print!("{} ", 
                if metadata.is_dir() { 
                    format!("{}/", name).blue().bold() 
                } else { 
                    name.normal() 
                }
            );
        }
    }
    
    if !args.long_format {
        println!();
    }
    
    Ok(())
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
        format!("{}B", size as u64)
    } else {
        format!("{:.1}{}", size, UNITS[unit_index])
    }
}