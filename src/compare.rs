use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::path_utils::find_common_ancestor;
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
        NodeStatus::DirectoryExcluded | NodeStatus::DirectoryIncluded | NodeStatus::DirectoryMixed
    )
}

/// Re-root a tree under a new base path by wrapping it in synthetic ancestor nodes.
///
/// Given a tree rooted at `/home/user/docs` and a new base of `/home`, this produces:
///
/// ```text
/// user (DirectoryMixed)
/// └── docs (original tree, unchanged)
/// ```
///
/// When `new_base` is the filesystem root (`/`), the result is wrapped under a
/// synthetic root node named `"/"`:
///
/// ```text
/// / (DirectoryMixed)
/// └── home (DirectoryMixed)
///     └── user (DirectoryMixed)
///         └── docs (original tree, unchanged)
/// ```
///
/// The synthetic ancestors get `DirectoryMixed` status because they contain children
/// but don't appear in the original rsync output — some descendants may be included,
/// some excluded.
///
/// Returns the original tree unchanged if `new_base` is already the tree's root path.
pub fn reroot_tree(tree: Tree, new_base: &Path) -> Tree {
    let tree_path = tree.path.clone();

    if &tree_path == new_base {
        return tree;
    }

    // Strip the new_base prefix to get the relative path components
    let relative = tree_path.strip_prefix(new_base).unwrap();
    let components: Vec<_> = relative.components().collect();

    // Build synthetic ancestors from innermost to outermost,
    // wrapping the original tree as we go.
    let tree_path = tree.path.clone();
    let mut current = tree;
    let mut ancestor_path = tree_path;
    for component in components.iter().rev().skip(1) {
        ancestor_path.pop();
        let name = component.as_os_str().to_string_lossy().to_string();
        let mut ancestor = Tree::new(name, ancestor_path.clone(), NodeStatus::DirectoryMixed);
        ancestor.add_child(current);
        current = ancestor;
    }

    // When `new_base` is the filesystem root `/`, the loop above produces a root
    // named after the first path component (e.g. `"home"`). We need to wrap that
    // under a synthetic root node named `"/"` so the returned tree is actually
    // rooted at the filesystem root.
    if new_base == Path::new("/") && current.path != Path::new("/") {
        let mut root = Tree::new(
            "/".to_string(),
            PathBuf::from("/"),
            NodeStatus::DirectoryMixed,
        );
        root.add_child(current);
        current = root;
    }

    current
}

/// Load trees from JSON files and re-root them to a common ancestor.
///
/// Each tree's root path is canonicalized (if the path exists on disk).
/// The common ancestor of all root paths is computed, and each tree is
/// re-rooted under that ancestor with synthetic intermediate nodes.
/// This allows comparing trees that were saved with different `--base-path` values.
pub fn load_trees(paths: &[PathBuf]) -> Result<Vec<Tree>, String> {
    if paths.len() < 2 {
        return Err("At least 2 trees are required for comparison".to_string());
    }

    let mut trees = Vec::with_capacity(paths.len());

    for path in paths {
        let tree = Tree::load_from_file(path)
            .map_err(|e| format!("Failed to load tree from {}: {}", path.display(), e))?;
        trees.push(tree);
    }

    // Extract root paths and compute the common ancestor
    let root_paths: Vec<PathBuf> = trees.iter().map(|t| t.path.clone()).collect();
    let common_ancestor = find_common_ancestor(&root_paths);

    // Use the common ancestor's parent as the re-root base so that all trees
    // get wrapped under a synthetic parent with the common ancestor's name.
    let reroot_base = common_ancestor
        .parent()
        .unwrap_or(&common_ancestor)
        .to_path_buf();

    // Re-root each tree to the reroot base
    let mut rerooted = Vec::with_capacity(trees.len());
    for tree in trees {
        rerooted.push(reroot_tree(tree, &reroot_base));
    }

    Ok(rerooted)
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
