use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "rsync_tree",
    about = "Execute rsync with itemize-changes and display results as a tree",
    long_about = "This tool takes an rsync command, adds --itemize-changes, executes it, \
                  parses the output, and displays the results as a tree structure."
)]
pub struct Args {
    /// The rsync command to execute (without --itemize-changes, it will be added automatically)
    #[arg(help = "Rsync command to execute (e.g., 'rsync -av src/ dest/')")]
    pub rsync_command: String,

    /// Base path for tree construction (defaults to current directory)
    #[arg(
        short,
        long,
        help = "Base path for tree construction (defaults to current directory)"
    )]
    pub base_path: Option<PathBuf>,

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

    /// Save rsync output to file
    #[arg(short, long, help = "Save rsync output to specified file")]
    pub save_output: Option<PathBuf>,

    /// Save the built tree to a JSON file for later visualization
    #[arg(long, help = "Save the built tree to a JSON file")]
    pub save_tree: Option<PathBuf>,

    /// Load a previously saved tree from a JSON file instead of running rsync
    #[arg(long, help = "Load a tree from a JSON file instead of running rsync")]
    pub load_tree: Option<PathBuf>,
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
