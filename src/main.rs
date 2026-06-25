use clap::Parser;

mod cli;
mod command_utils;
mod executor;
mod run;

use cli::{Args, setup_logging};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    setup_logging(&args.log_level)?;

    if let Some(ref tree_paths) = args.compare {
        return run::run_compare_mode(tree_paths, &args);
    }

    run::run_single_mode(&args)
}
