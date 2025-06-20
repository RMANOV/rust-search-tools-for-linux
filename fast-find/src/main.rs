use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "ffind")]
#[command(about = "Ultra-fast parallel file finder")]
struct Args {
    /// Pattern to search for in filenames
    pattern: String,
    
    /// Directories to search in
    #[arg(default_value = ".")]
    paths: Vec<std::path::PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    println!("Fast-find placeholder - Pattern: {}, Paths: {:?}", args.pattern, args.paths);
    println!("This tool will be implemented next!");
    
    Ok(())
}