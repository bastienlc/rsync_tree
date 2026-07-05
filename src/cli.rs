use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "rsync_tree",
    about = "Read rsync --itemize-changes output from stdin and display results as a tree",
    long_about = "This tool reads rsync's --itemize-changes output from stdin, \
                  parses it, and displays the results as a tree structure. \
                  It can also compare multiple previously-saved trees. \
                  \n\nUsage: rsync -av --dry-run --itemize-changes src/ dest/ | rsync_tree build -b src/\n       rsync_tree compare backup-week1.json backup-week2.json"
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Build and display a tree from rsync --itemize-changes output
    Build(BuildArgs),
    /// Compare multiple previously-saved tree JSON files
    Compare(CompareArgs),
}

#[derive(Parser, Debug)]
pub struct BuildArgs {
    /// Base path for tree construction (required)
    #[arg(short, long)]
    pub base_path: PathBuf,

    /// Enable colored output
    #[arg(short, long, default_value = "true")]
    pub color: bool,

    /// Collapse directories that are entirely included or excluded
    #[arg(short = 'C', long, default_value = "true")]
    pub collapse: bool,

    /// Show debug information (node status) in tree output
    #[arg(short = 'D', long)]
    pub debug: bool,

    /// Collect and display file/folder sizes (may be slow for large directories)
    #[arg(short = 'S', long, default_value = "true")]
    pub show_sizes: bool,

    /// Logging level
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Save the built tree to a JSON file for later visualization
    #[arg(long)]
    pub save_tree: Option<PathBuf>,

    /// Load a previously saved tree from a JSON file instead of running rsync
    #[arg(long)]
    pub load_tree: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct CompareArgs {
    /// Paths to saved tree JSON files (at least 2 required)
    #[arg(required = true, num_args = 2..)]
    pub paths: Vec<PathBuf>,

    /// Enable colored output
    #[arg(short, long, default_value = "true")]
    pub color: bool,

    /// Show debug information in tree output
    #[arg(short = 'D', long)]
    pub debug: bool,

    /// Logging level
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Ignore file sizes when comparing trees
    #[arg(long)]
    pub ignore_size: bool,
}

pub fn setup_logging(level: &str) -> Result<(), Box<dyn std::error::Error>> {
    let level_filter = match level.to_lowercase().as_str() {
        "error" => log::LevelFilter::Error,
        "warn" => log::LevelFilter::Warn,
        "info" => log::LevelFilter::Info,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        _ => {
            eprintln!("Invalid log level: {}. Using 'info'", level);
            log::LevelFilter::Info
        }
    };

    env_logger::Builder::from_default_env()
        .filter_level(level_filter)
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .init();

    Ok(())
}
