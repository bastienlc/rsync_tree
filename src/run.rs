use log::{debug, info, warn};
use rsync_tree::TreeBuildError;
use rsync_tree::build_tree_from_rsync_output;
use rsync_tree::compare::{compare_trees, filter_diff, load_trees};
use rsync_tree::display::{self, render_compare_tree};
use rsync_tree::rsync_types::ParseResult;
use std::fs;

use crate::cli::{BuildArgs, CompareArgs};

/// Run single-tree analysis mode.
pub fn run_single_mode(
    parse_results: Vec<ParseResult>,
    args: &BuildArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting rsync tree analysis");
    debug!("Number of parse results: {}", parse_results.len());

    if parse_results.is_empty() {
        warn!("No parseable items received from stdin");
        info!("This might indicate an empty rsync output or no files to synchronize");
        return Ok(());
    }

    info!("Building tree from parsed results...");
    let base_path = fs::canonicalize(&args.base_path)?;
    let tree =
        build_tree_from_rsync_output(parse_results, &base_path, args.show_sizes).map_err(|e| {
            match e {
                TreeBuildError::IoError(e) => format!("IO error while building tree: {}", e),
                TreeBuildError::InvalidPath(path) => {
                    format!("Invalid path encountered while building tree: {}", path)
                }
            }
        })?;

    if let Some(ref save_path) = args.save_tree {
        info!("Saving tree to file: {}", save_path.display());
        tree.save_to_file(save_path)
            .map_err(|e| format!("Failed to save tree to {}: {}", save_path.display(), e))?;
        info!("Tree saved successfully");
    }

    println!("\nRsync Analysis Tree:");
    println!("===================");

    display::render_tree(
        &tree,
        &mut std::io::stdout(),
        args.color,
        args.collapse,
        args.debug,
        args.show_sizes,
    )
    .map_err(|e| format!("Failed to render tree: {}", e))?;

    display::display_legends(args.debug, args.show_sizes, args.color, 0);
    info!("Analysis completed successfully");
    Ok(())
}

/// Run comparison mode.
pub fn run_compare_mode(args: &CompareArgs) -> Result<(), Box<dyn std::error::Error>> {
    let trees = load_trees(&args.paths).map_err(|e| e.to_string())?;
    info!("Loaded {} tree(s) for comparison", trees.len());

    info!("Building comparison tree...");
    let compared = compare_trees(&trees);
    info!("Comparison tree built successfully");

    let tree_labels: Vec<String> = args
        .paths
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
            render_compare_tree(
                &root,
                &mut std::io::stdout(),
                args.color,
                args.debug,
                &tree_labels,
            )?;
            display::display_legends(false, false, args.color, num_trees);
        }
        None => {
            println!("\nNo differences found — all trees are identical.");
        }
    }

    info!("Comparison completed successfully");
    Ok(())
}
