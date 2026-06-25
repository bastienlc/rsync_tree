use clap::Parser;
use log::{debug, info, warn};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rsync_tree::{Tree, TreeBuildError, build_tree_from_rsync_output_string, compare};

mod cli;
mod command_utils;
mod display;
mod executor;

use cli::{Args, setup_logging};
use command_utils::{add_required_flags, determine_base_path, parse_command};
use compare::{ComparisonOptions, compare_trees, filter_diff, load_trees};
use display::display_legends;
use executor::execute_rsync_with_output;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    setup_logging(&args.log_level)?;

    if let Some(ref tree_paths) = args.compare {
        return run_compare_mode(tree_paths, &args);
    }

    run_single_mode(&args)
}

fn run_single_mode(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting rsync tree analysis");
    debug!("Arguments: {:?}", args);

    let tree = match acquire_tree(args)? {
        Some(tree) => tree,
        None => return Ok(()),
    };

    println!("\nRsync Analysis Tree:");
    println!("===================");

    tree.render_ascii(
        &mut std::io::stdout(),
        args.color,
        args.collapse,
        args.debug,
        args.show_sizes,
    )
    .map_err(|e| format!("Failed to render tree: {}", e))?;

    display_legends(args.debug, args.show_sizes, args.color, 0);
    info!("Analysis completed successfully");
    Ok(())
}

/// Load a tree from a saved file or build one by running rsync.
/// Returns `None` when rsync produces no output.
fn acquire_tree(args: &Args) -> Result<Option<Tree>, Box<dyn std::error::Error>> {
    if let Some(ref load_path) = args.load_tree {
        info!("Loading tree from file: {}", load_path.display());
        let tree = Tree::load_from_file(load_path)
            .map_err(|e| format!("Failed to load tree from {}: {}", load_path.display(), e))?;
        info!("Tree loaded successfully");
        return Ok(Some(tree));
    }

    let (rsync_output, base_path) = run_rsync(args)?;

    if rsync_output.trim().is_empty() {
        warn!("No output received from rsync command");
        info!("This might indicate that no files needed to be synchronized");
        return Ok(None);
    }

    info!("Building tree from rsync output...");
    debug!("Rsync output length: {} characters", rsync_output.len());

    let tree = build_tree(&rsync_output, &base_path, args.show_sizes)?;

    if let Some(ref save_path) = args.save_tree {
        info!("Saving tree to file: {}", save_path.display());
        tree.save_to_file(save_path)
            .map_err(|e| format!("Failed to save tree to {}: {}", save_path.display(), e))?;
        info!("Tree saved successfully");
    }

    Ok(Some(tree))
}

/// Validate the rsync command, add required flags, execute it, and return
/// the captured output together with the detected base path.
fn run_rsync(args: &Args) -> Result<(String, PathBuf), Box<dyn std::error::Error>> {
    let rsync_command = args.rsync_command.as_ref().ok_or(
        "No rsync command provided. Use --compare to compare trees, or provide an rsync command.",
    )?;

    let mut rsync_args = parse_command(rsync_command);
    if rsync_args.is_empty() {
        return Err("Empty rsync command provided".into());
    }
    if rsync_args[0] != "rsync" {
        return Err(format!("Command must start with 'rsync', got: '{}'", rsync_args[0]).into());
    }

    rsync_args = add_required_flags(rsync_args);

    let base_path = determine_base_path(&rsync_args, args.base_path.as_ref());
    info!("Using base path: {}", base_path.display());

    let output = execute_rsync_with_output(rsync_args, args.save_output.as_ref())
        .map_err(|e| format!("Failed to execute rsync command: {}", e))?;

    Ok((output, base_path))
}

/// Parse the rsync output string into a `Tree`, mapping errors to
/// user-friendly messages.
fn build_tree(
    output: &str,
    base_path: &Path,
    show_sizes: bool,
) -> Result<Tree, Box<dyn std::error::Error>> {
    match build_tree_from_rsync_output_string(output, base_path, show_sizes) {
        Ok(tree) => Ok(tree),
        Err(TreeBuildError::IoError(e)) => {
            Err(format!("IO error while building tree: {}", e).into())
        }
        Err(TreeBuildError::InvalidPath(path)) => {
            Err(format!("Invalid path encountered while building tree: {}", path).into())
        }
    }
}

fn run_compare_mode(tree_paths: &[PathBuf], args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let options = ComparisonOptions {
        ignored_fields: if args.ignore_size {
            HashSet::from(["size".to_string()])
        } else {
            HashSet::new()
        },
    };

    let trees = load_trees(tree_paths).map_err(|e| e.to_string())?;
    info!("Loaded {} tree(s) for comparison", trees.len());

    info!("Building comparison tree...");
    let compared = compare_trees(&trees, &options);
    info!("Comparison tree built successfully");

    let tree_labels: Vec<String> = tree_paths
        .iter()
        .map(|p| {
            p.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| p.to_string_lossy().to_string())
        })
        .collect();
    debug!("Tree labels: {:?}", tree_labels);

    let filtered = filter_diff(&compared);

    let num_trees = tree_labels.len();
    match filtered {
        Some(root) => {
            println!("\nTree Comparison — Differences:");
            println!("========================");
            root.render_ascii(&mut std::io::stdout(), args.color, args.debug, &tree_labels)?;
            display_legends(false, false, args.color, num_trees);
        }
        None => {
            println!("\nNo differences found — all trees are identical.");
        }
    }

    info!("Comparison completed successfully");
    Ok(())
}
