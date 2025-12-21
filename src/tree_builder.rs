use std::fs;
use std::path::Path;

use log::warn;

use crate::path_utils::{PathTrie, get_display_name, normalize_path};
use crate::rsync_types::{ParseResult, RsyncItem};
use crate::tree::{NodeStatus, Tree};

/// Error types for tree construction
#[derive(Debug)]
pub enum TreeBuildError {
    IoError(std::io::Error),
    InvalidPath(String),
}

impl From<std::io::Error> for TreeBuildError {
    fn from(error: std::io::Error) -> Self {
        TreeBuildError::IoError(error)
    }
}

/// Builds a tree representation from rsync output and filesystem analysis
pub fn build_tree_from_rsync_output(
    parse_results: Vec<ParseResult>,
    base_path: &Path,
    collect_sizes: bool,
) -> Result<Tree, TreeBuildError> {
    // Extract included items from parsed results
    let included_items: Vec<RsyncItem> = parse_results
        .into_iter()
        .filter_map(|result| match result {
            ParseResult::Item(item) => Some(item),
            _ => None,
        })
        .collect();

    // Create a path trie of all included paths
    let mut included_paths = PathTrie::new();
    for item in &included_items {
        let normalized_path = normalize_path(&item.path, base_path);
        included_paths.insert(&normalized_path);
    }

    // Get the base name for the root tree node
    let root_name = get_display_name(base_path);

    // Create the root tree node with initial status
    let mut root = Tree::new(
        root_name,
        base_path.to_path_buf(),
        NodeStatus::DirectoryMixed,
    );

    // First, scan the entire filesystem to build the complete tree structure
    build_complete_tree(
        &mut root,
        base_path,
        &included_paths,
        base_path,
        collect_sizes,
    )?;

    // If size collection is enabled, compute folder sizes recursively
    if collect_sizes {
        root.compute_folder_sizes();
    }

    Ok(root)
}

/// Determine the initial status for a node based on its type and inclusion
fn determine_initial_status(entry_path: &Path, is_included: bool) -> NodeStatus {
    if entry_path.is_dir() {
        if is_included {
            NodeStatus::DirectoryStandalone // Will be updated based on children
        } else {
            NodeStatus::DirectoryExcluded
        }
    } else {
        if is_included {
            NodeStatus::FileIncluded
        } else {
            NodeStatus::FileExcluded
        }
    }
}

/// Create a child node with metadata
fn create_child_node(
    entry_path: &Path,
    is_included: bool,
    collect_sizes: bool,
) -> Result<Tree, TreeBuildError> {
    let file_name = get_display_name(&entry_path);
    let initial_status = determine_initial_status(entry_path, is_included);
    let mut child = Tree::new(file_name, entry_path.to_path_buf(), initial_status);

    // Collect file size if enabled and file is included
    if collect_sizes && !entry_path.is_dir() && is_included {
        if let Ok(metadata) = fs::metadata(&entry_path) {
            child.size = Some(metadata.len());
        } else {
            warn!("Failed to get metadata for file: {}", entry_path.display());
        }
    }

    Ok(child)
}
/// Recursively builds the complete tree by scanning filesystem and marking included/excluded status
fn build_complete_tree(
    node: &mut Tree,
    current_path: &Path,
    included_paths: &PathTrie,
    base_path: &Path,
    collect_sizes: bool,
) -> Result<(), TreeBuildError> {
    if !current_path.exists() || !current_path.is_dir() {
        return Ok(());
    }

    let entries = fs::read_dir(current_path)?;

    for entry in entries {
        let entry = entry?;
        let entry_path = entry.path();

        // Calculate the relative path from the base path
        let relative_path = entry_path
            .strip_prefix(base_path)
            .unwrap_or(&entry_path)
            .to_path_buf();

        // Determine if this path is included
        let is_included = included_paths.is_path_included(&relative_path);

        // Create the child node
        let mut child = create_child_node(&entry_path, is_included, collect_sizes)?;

        // Recursively process if it's a directory
        if entry_path.is_dir() {
            if is_included {
                // Directory is included, so we need to check its children
                build_complete_tree(
                    &mut child,
                    &entry_path,
                    included_paths,
                    base_path,
                    collect_sizes,
                )?;

                // Update directory status based on children's statuses
                child.status = compute_directory_status(&child);
            } else {
                child.status = NodeStatus::DirectoryExcluded;
            }
        }

        node.add_child(child);
    }

    Ok(())
}

/// Compute the directory status based on its inclusion and children's statuses
fn compute_directory_status(directory: &Tree) -> NodeStatus {
    // Directory is included
    if directory.children.is_empty() {
        // Empty directory that is included
        NodeStatus::DirectoryStandalone
    } else {
        let all_children_included = directory.children.values().all(|child| match child.status {
            NodeStatus::FileIncluded
            | NodeStatus::DirectoryIncluded
            | NodeStatus::DirectoryStandalone => true,
            _ => false,
        });

        let any_children_included = directory.children.values().any(|child| match child.status {
            NodeStatus::FileIncluded
            | NodeStatus::DirectoryIncluded
            | NodeStatus::DirectoryStandalone
            | NodeStatus::DirectoryMixed => true,
            _ => false,
        });

        if all_children_included {
            NodeStatus::DirectoryIncluded
        } else if any_children_included {
            NodeStatus::DirectoryMixed
        } else {
            NodeStatus::DirectoryStandalone
        }
    }
}

/// Convenience function to build tree from rsync output string
pub fn build_tree_from_rsync_output_string(
    output: &str,
    base_path: &Path,
    collect_sizes: bool,
) -> Result<Tree, TreeBuildError> {
    let parse_results = crate::rsync_parser::parse_rsync_output(output);

    for result in &parse_results {
        if let ParseResult::InvalidFormat(line) = result {
            warn!("Could not parse line: {}", line);
        }
    }

    build_tree_from_rsync_output(parse_results, base_path, collect_sizes)
}
