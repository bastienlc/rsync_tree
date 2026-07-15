use std::fs;
use std::path::Path;

use log::warn;

use crate::path_utils::{PathTrie, get_display_name, normalize_path};
use crate::rsync_types::{FileType, ParseResult, RsyncItem};
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

    // Create a path trie of all included paths, carrying the full RsyncItem
    let mut included_paths = PathTrie::new();
    for item in &included_items {
        let normalized_path = normalize_path(&item.path, base_path);
        included_paths.insert(&normalized_path, item);
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
            NodeStatus::DirectoryMixed // Will be updated based on children
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

/// Create a child node with metadata.
///
/// When `rsync_item` indicates a symlink (`FileType::Symlink`), the size is read from
/// `symlink_metadata` (the inode itself, not the target) and `link_target` is stored.
/// Otherwise `metadata` is used (following symlinks) and no `link_target` is set.
fn create_child_node(
    entry_path: &Path,
    is_included: bool,
    collect_sizes: bool,
    rsync_item: Option<&&RsyncItem>,
) -> Result<Tree, TreeBuildError> {
    let file_name = get_display_name(entry_path);

    // Determine whether rsync reported this as a symlink
    let is_rsync_symlink = rsync_item.and_then(|i| i.file_type) == Some(FileType::Symlink);

    let initial_status = if is_rsync_symlink {
        // A symlink is always a leaf node — never a directory, even if it points to one.
        // `determine_initial_status` uses `entry_path.is_dir()` which follows symlinks,
        // so we bypass it for symlinks explicitly.
        if is_included {
            NodeStatus::FileIncluded
        } else {
            NodeStatus::FileExcluded
        }
    } else {
        determine_initial_status(entry_path, is_included)
    };

    let mut child = Tree::new(file_name, entry_path.to_path_buf(), initial_status);

    // Collect size if enabled and the node is included
    if collect_sizes && is_included {
        if is_rsync_symlink {
            // Use symlink_metadata: reads the symlink inode, works on broken symlinks
            if let Ok(meta) = fs::symlink_metadata(entry_path) {
                child.size = Some(meta.len());
            }
        } else if !entry_path.is_dir() {
            if let Ok(metadata) = fs::metadata(entry_path) {
                child.size = Some(metadata.len());
            } else {
                warn!("Failed to get metadata for file: {}", entry_path.display());
            }
        }
    }

    // Store the link target from rsync output
    if is_rsync_symlink {
        child.link_target = rsync_item.and_then(|i| i.link_target.clone());
    }

    Ok(child)
}
/// Recursively builds the complete tree by scanning filesystem and marking included/excluded status
fn build_complete_tree(
    node: &mut Tree,
    current_path: &Path,
    included_paths: &PathTrie<&RsyncItem>,
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

        // Look up the full RsyncItem from the trie
        let rsync_item = included_paths.get(&relative_path);
        let is_included = rsync_item.is_some();

        // Determine whether rsync reported this as a symlink
        let is_rsync_symlink = rsync_item.and_then(|i| i.file_type) == Some(FileType::Symlink);

        // Create the child node
        let mut child = create_child_node(&entry_path, is_included, collect_sizes, rsync_item)?;

        // Recursively process if it's a directory — but NOT if rsync reported a symlink
        // (symlinks are always leaf nodes even when they point to directories).
        if entry_path.is_dir() && !is_rsync_symlink {
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
        // Empty directory that is included — nothing to be mixed about
        NodeStatus::DirectoryIncluded
    } else {
        let all_children_included = directory.children.values().all(|child| {
            matches!(
                child.status,
                NodeStatus::FileIncluded | NodeStatus::DirectoryIncluded
            )
        });

        if all_children_included {
            NodeStatus::DirectoryIncluded
        } else {
            NodeStatus::DirectoryMixed
        }
    }
}
