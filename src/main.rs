use clap::Parser;

use rsync_tree::input::read_and_parse_stdin;

mod cli;
mod run;

use cli::{Args, setup_logging};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    setup_logging(&args.log_level)?;

    if let Some(ref tree_paths) = args.compare {
        return run::run_compare_mode(tree_paths, &args);
    }

    let parse_results = read_and_parse_stdin()?;
    run::run_single_mode(parse_results, &args)
}
