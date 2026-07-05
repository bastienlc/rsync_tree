use clap::Parser;

use rsync_tree::input::read_and_parse_stdin;

mod cli;
mod run;

use cli::{Args, Command, setup_logging};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.command {
        Command::Build(tree_args) => {
            setup_logging(&tree_args.log_level)?;
            let parse_results = read_and_parse_stdin()?;
            run::run_single_mode(parse_results, &tree_args)
        }
        Command::Compare(compare_args) => {
            setup_logging(&compare_args.log_level)?;
            run::run_compare_mode(&compare_args)
        }
    }
}
