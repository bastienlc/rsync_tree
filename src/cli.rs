use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "rsync_tree",
    about = "Read rsync --itemize-changes output from stdin and display results as a tree",
    long_about = "This tool reads rsync's --itemize-changes output from stdin, \
                  parses it, and displays the results as a tree structure. \
                  It can also compare multiple previously-saved trees. \
                  \n\nUsage: rsync -av --dry-run --itemize-changes src/ dest/ | rsync_tree --base-path src/"
)]
pub struct Args {
    /// Base path for tree construction (REQUIRED)
    #[arg(short, long, help = "Base path for tree construction (required)")]
    pub base_path: PathBuf,

    /// Enable colored output
    #[arg(
        short,
        long,
        default_value = "true",
        help = "Enable colored tree output"
    )]
    pub color: bool,

    /// Collapse included/excluded directories in tree output
    #[arg(
        short = 'C',
        long,
        default_value = "true",
        help = "Collapse directories that are entirely included or excluded"
    )]
    pub collapse: bool,

    /// Show debug information (node status) in tree output
    #[arg(short = 'D', long, help = "Show debug information in tree output")]
    pub debug: bool,

    /// Collect and display file/folder sizes (may be slow for large directories)
    #[arg(
        short = 'S',
        long,
        default_value = "true",
        help = "Collect and display file/folder sizes"
    )]
    pub show_sizes: bool,

    /// Logging level
    #[arg(
        long,
        default_value = "info",
        help = "Set logging level (error, warn, info, debug, trace)"
    )]
    pub log_level: String,

    /// Save the built tree to a JSON file for later visualization
    #[arg(long, help = "Save the built tree to a JSON file")]
    pub save_tree: Option<PathBuf>,

    /// Load a previously saved tree from a JSON file instead of running rsync
    #[arg(long, help = "Load a tree from a JSON file instead of running rsync")]
    pub load_tree: Option<PathBuf>,

    /// Compare multiple previously-saved trees (JSON files)
    #[arg(
        long,
        num_args = 2..,
        help = "Compare multiple saved tree JSON files"
    )]
    pub compare: Option<Vec<PathBuf>>,

    /// Ignore file sizes when comparing trees
    #[arg(long, help = "Ignore file size differences when comparing trees")]
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
