use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::tree::{NodeStatus, Tree};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PerTreeStatus {
    Included,
    Excluded,
    Mixed,
    Missing,
}

#[derive(Debug, Clone)]
pub struct ComparedNode {
    pub name: String,
    pub per_tree_status: Vec<PerTreeStatus>,
    pub children: BTreeMap<String, ComparedNode>,
    pub all_equal: bool,
    pub is_directory: bool,
}

/// Map a NodeStatus to a PerTreeStatus
pub fn status_to_compare_status(status: NodeStatus) -> PerTreeStatus {
    match status {
        NodeStatus::FileIncluded | NodeStatus::DirectoryIncluded => PerTreeStatus::Included,
        NodeStatus::FileExcluded | NodeStatus::DirectoryExcluded => PerTreeStatus::Excluded,
        NodeStatus::DirectoryMixed => PerTreeStatus::Mixed,
    }
}

fn status_is_directory(status: NodeStatus) -> bool {
    matches!(
        status,
        NodeStatus::DirectoryExcluded
            | NodeStatus::DirectoryIncluded
            | NodeStatus::DirectoryMixed
    )
}

/// Load trees from JSON files, validating that all roots have the same name.
pub fn load_trees(paths: &[PathBuf]) -> Result<Vec<Tree>, String> {
    if paths.len() < 2 {
        return Err("At least 2 trees are required for comparison".to_string());
    }

    let mut trees = Vec::with_capacity(paths.len());
    let mut root_name: Option<String> = None;

    for path in paths {
        let tree = Tree::load_from_file(path)
            .map_err(|e| format!("Failed to load tree from {}: {}", path.display(), e))?;

        match &root_name {
            Some(expected) => {
                if &tree.name != expected {
                    return Err(format!(
                        "Root name mismatch: '{}' has root '{}' but expected '{}'",
                        path.display(),
                        tree.name,
                        expected
                    ));
                }
            }
            None => {
                root_name = Some(tree.name.clone());
            }
        }

        trees.push(tree);
    }

    Ok(trees)
}

/// Compare multiple trees and build a ComparedNode.
pub fn compare_trees(trees: &[Tree]) -> ComparedNode {
    let tree_refs: Vec<Option<&Tree>> = trees.iter().map(Some).collect();
    merge_trees(&tree_refs)
}

/// Recursively merge N trees into a single ComparedNode tree.
///
/// `trees[i]` is `Some(tree)` if tree i has a node at this position,
/// or `None` if tree i has no such node (→ `PerTreeStatus::Missing`).
#[allow(clippy::only_used_in_recursion)]
fn merge_trees(trees: &[Option<&Tree>]) -> ComparedNode {
    // Per-tree status for this node
    let per_tree_status: Vec<PerTreeStatus> = trees
        .iter()
        .map(|t| match t {
            Some(tree) => status_to_compare_status(tree.status),
            None => PerTreeStatus::Missing,
        })
        .collect();

    // Union of all child names across all present trees
    let mut all_child_names: BTreeSet<String> = BTreeSet::new();
    for tree in trees.iter().flatten() {
        for name in tree.children.keys() {
            all_child_names.insert(name.clone());
        }
    }

    // Recursively merge children
    let mut children: BTreeMap<String, ComparedNode> = BTreeMap::new();
    for child_name in &all_child_names {
        let child_refs: Vec<Option<&Tree>> = trees
            .iter()
            .map(|t| t.and_then(|tree| tree.children.get(child_name)))
            .collect();

        let merged_child = merge_trees(&child_refs);
        children.insert(child_name.clone(), merged_child);
    }

    // all_equal: all per_tree_status entries are identical AND all children are equal
    let statuses_equal = per_tree_status.windows(2).all(|w| w[0] == w[1]);
    let children_all_equal = children.values().all(|c| c.all_equal);
    let all_equal = statuses_equal && children_all_equal;

    // A node is a directory if any present tree says it's a directory
    let is_directory = trees
        .iter()
        .flatten()
        .any(|t| status_is_directory(t.status));

    // Name from the first present tree (all roots validated to match)
    let name = trees
        .iter()
        .find_map(|t| t.map(|t| t.name.clone()))
        .unwrap_or_default();

    ComparedNode {
        name,
        per_tree_status,
        children,
        all_equal,
        is_directory,
    }
}

/// Filter the tree for diff mode.
///
/// Keep only nodes where `all_equal == false`. Nodes where all trees agree
/// are pruned entirely (including their subtrees).
pub fn filter_diff(node: &ComparedNode) -> Option<ComparedNode> {
    if node.all_equal {
        return None;
    }

    let mut filtered_children = BTreeMap::new();
    for (name, child) in &node.children {
        if let Some(filtered) = filter_diff(child) {
            filtered_children.insert(name.clone(), filtered);
        }
    }

    Some(ComparedNode {
        name: node.name.clone(),
        per_tree_status: node.per_tree_status.clone(),
        children: filtered_children,
        all_equal: node.all_equal,
        is_directory: node.is_directory,
    })
}

impl ComparedNode {
    /// Can this directory be collapsed in diff mode?
    ///
    /// A directory can be collapsed if none of its `per_tree_status` entries
    /// are `Mixed` — the difference is at the node level (e.g. Included vs
    /// Excluded), not in the children.
    pub fn can_collapse(&self) -> bool {
        !self.per_tree_status.contains(&PerTreeStatus::Mixed)
    }
}
