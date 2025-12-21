use clap::Parser;
use log::{debug, error, info, warn};

use rsync_tree::{TreeBuildError, build_tree_from_rsync_output_string};

mod cli;
mod command_utils;
mod display;
mod executor;

use cli::{Args, setup_logging};
use command_utils::{add_required_flags, determine_base_path, parse_command};
use display::display_legends;
use executor::execute_rsync_with_output;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Setup logging
    setup_logging(&args.log_level)?;

    info!("Starting rsync tree analysis");
    debug!("Arguments: {:?}", args);

    // Parse the rsync command
    let mut rsync_args = parse_command(&args.rsync_command);
    if rsync_args.is_empty() {
        error!("Empty rsync command provided");
        std::process::exit(1);
    }

    // Ensure we have rsync as the program
    if rsync_args[0] != "rsync" {
        error!("Command must start with 'rsync', got: {}", rsync_args[0]);
        std::process::exit(1);
    }

    // Add required flags
    rsync_args = add_required_flags(rsync_args);

    // Determine base path for tree construction
    let base_path = determine_base_path(&rsync_args, args.base_path.as_ref());
    info!("Using base path: {}", base_path.display());

    // Execute rsync command
    let rsync_output = match execute_rsync_with_output(rsync_args, args.save_output.as_ref()) {
        Ok(output) => output,
        Err(e) => {
            error!("Failed to execute rsync command: {}", e);
            std::process::exit(1);
        }
    };

    if rsync_output.trim().is_empty() {
        warn!("No output received from rsync command");
        info!("This might indicate that no files needed to be synchronized");
        return Ok(());
    }

    info!("Building tree from rsync output...");
    debug!("Rsync output length: {} characters", rsync_output.len());

    // Build tree from rsync output
    let tree = match build_tree_from_rsync_output_string(&rsync_output, &base_path, args.show_sizes)
    {
        Ok(tree) => tree,
        Err(TreeBuildError::IoError(e)) => {
            error!("IO error while building tree: {}", e);
            std::process::exit(1);
        }
        Err(TreeBuildError::InvalidPath(path)) => {
            error!("Invalid path encountered while building tree: {}", path);
            std::process::exit(1);
        }
    };

    info!("Tree construction completed successfully");

    // Display the tree
    println!("\nRsync Analysis Tree:");
    println!("===================");

    if let Err(e) = tree.render_ascii(
        &mut std::io::stdout(),
        args.color,
        args.collapse,
        args.debug,
        args.show_sizes,
    ) {
        error!("Failed to render tree: {}", e);
        std::process::exit(1);
    }

    display_legends(args.debug, args.show_sizes, args.color);

    info!("Analysis completed successfully");
    Ok(())
}
