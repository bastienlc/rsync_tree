use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::compare::{
    ComparedNode, PerTreeStatus, compare_trees, filter_diff, load_trees, reroot_tree,
    status_to_compare_status,
};
use crate::path_utils::find_common_ancestor;
use crate::tree::{NodeStatus, Tree};

fn make_file(name: &str, status: NodeStatus) -> Tree {
    let mut t = Tree::new(name.into(), name.into(), status);
    if status == NodeStatus::FileIncluded {
        t.size = Some(100);
    }
    t
}

fn make_dir(name: &str, status: NodeStatus, children: Vec<Tree>) -> Tree {
    let mut t = Tree::new(name.into(), name.into(), status);
    for child in children {
        t.add_child(child);
    }
    t
}

#[test]
fn test_status_mapping() {
    assert_eq!(
        status_to_compare_status(NodeStatus::FileIncluded),
        PerTreeStatus::Included
    );
    assert_eq!(
        status_to_compare_status(NodeStatus::FileExcluded),
        PerTreeStatus::Excluded
    );
    assert_eq!(
        status_to_compare_status(NodeStatus::DirectoryIncluded),
        PerTreeStatus::Included
    );
    assert_eq!(
        status_to_compare_status(NodeStatus::DirectoryExcluded),
        PerTreeStatus::Excluded
    );
    assert_eq!(
        status_to_compare_status(NodeStatus::DirectoryMixed),
        PerTreeStatus::Mixed
    );
}

#[test]
fn test_two_identical_trees_all_equal() {
    let files = vec![make_file("f1.txt", NodeStatus::FileIncluded)];
    let dir = make_dir("src", NodeStatus::DirectoryIncluded, files);

    let tree_a = make_dir("root", NodeStatus::DirectoryIncluded, vec![dir.clone()]);
    let tree_b = make_dir("root", NodeStatus::DirectoryIncluded, vec![dir]);

    let compared = compare_trees(&[tree_a, tree_b]);
    assert!(compared.all_equal, "Root should be all_equal");
    let src = compared.children.get("src").unwrap();
    assert!(src.all_equal, "src dir should be all_equal");
    let f1 = src.children.get("f1.txt").unwrap();
    assert!(f1.all_equal, "f1.txt should be all_equal");
}

#[test]
fn test_trees_different_status() {
    // Tree A: f1.txt is Included
    // Tree B: f1.txt is Excluded
    let tree_a = make_dir(
        "root",
        NodeStatus::DirectoryMixed,
        vec![make_file("f1.txt", NodeStatus::FileIncluded)],
    );
    let tree_b = make_dir(
        "root",
        NodeStatus::DirectoryMixed,
        vec![make_file("f1.txt", NodeStatus::FileExcluded)],
    );

    let compared = compare_trees(&[tree_a, tree_b]);

    assert!(!compared.all_equal);
    let f1 = compared.children.get("f1.txt").unwrap();
    assert!(!f1.all_equal);
    assert_eq!(
        f1.per_tree_status,
        vec![PerTreeStatus::Included, PerTreeStatus::Excluded]
    );
}

#[test]
fn test_missing_file_in_one_tree() {
    let tree_a = make_dir(
        "root",
        NodeStatus::DirectoryMixed,
        vec![make_file("f1.txt", NodeStatus::FileIncluded)],
    );
    let tree_b = make_dir("root", NodeStatus::DirectoryIncluded, vec![]);

    let compared = compare_trees(&[tree_a, tree_b]);

    assert!(!compared.all_equal);
    let f1 = compared.children.get("f1.txt").unwrap();
    assert!(!f1.all_equal);
    assert_eq!(
        f1.per_tree_status,
        vec![PerTreeStatus::Included, PerTreeStatus::Missing]
    );
}

#[test]
fn test_ignore_size_equality() {
    let mut f_a = make_file("f1.txt", NodeStatus::FileIncluded);
    f_a.size = Some(100);
    let mut f_b = make_file("f1.txt", NodeStatus::FileIncluded);
    f_b.size = Some(200);

    let tree_a = make_dir("root", NodeStatus::DirectoryIncluded, vec![f_a]);
    let tree_b = make_dir("root", NodeStatus::DirectoryIncluded, vec![f_b]);

    let compared = compare_trees(&[tree_a, tree_b]);
    assert!(
        compared.all_equal,
        "Should be equal: size maps to same PerTreeStatus"
    );
}

#[test]
fn test_three_trees_merge() {
    // Tree A: f1 included, f2 excluded
    // Tree B: f1 included, f2 included
    // Tree C: f1 excluded, (f2 missing)
    let tree_a = make_dir(
        "root",
        NodeStatus::DirectoryMixed,
        vec![
            make_file("f1.txt", NodeStatus::FileIncluded),
            make_file("f2.txt", NodeStatus::FileExcluded),
        ],
    );
    let tree_b = make_dir(
        "root",
        NodeStatus::DirectoryMixed,
        vec![
            make_file("f1.txt", NodeStatus::FileIncluded),
            make_file("f2.txt", NodeStatus::FileIncluded),
        ],
    );
    let tree_c = make_dir(
        "root",
        NodeStatus::DirectoryMixed,
        vec![make_file("f1.txt", NodeStatus::FileExcluded)],
    );

    let compared = compare_trees(&[tree_a, tree_b, tree_c]);

    // Check per-tree statuses
    let f1 = compared.children.get("f1.txt").unwrap();
    assert_eq!(
        f1.per_tree_status,
        vec![
            PerTreeStatus::Included,
            PerTreeStatus::Included,
            PerTreeStatus::Excluded
        ]
    );

    let f2 = compared.children.get("f2.txt").unwrap();
    assert_eq!(
        f2.per_tree_status,
        vec![
            PerTreeStatus::Excluded,
            PerTreeStatus::Included,
            PerTreeStatus::Missing
        ]
    );
}

#[test]
fn test_diff_mode_filters_equal_nodes() {
    // All nodes equal → filter_diff returns None
    let tree = make_dir(
        "root",
        NodeStatus::DirectoryIncluded,
        vec![make_file("f1.txt", NodeStatus::FileIncluded)],
    );
    let compared = compare_trees(&[tree.clone(), tree]);
    let filtered = filter_diff(&compared);
    assert!(
        filtered.is_none(),
        "All equal tree should produce None in --diff"
    );
}

#[test]
fn test_diff_mode_keeps_different_nodes() {
    let tree_a = make_dir(
        "root",
        NodeStatus::DirectoryMixed,
        vec![make_file("f1.txt", NodeStatus::FileIncluded)],
    );
    let tree_b = make_dir(
        "root",
        NodeStatus::DirectoryMixed,
        vec![make_file("f1.txt", NodeStatus::FileExcluded)],
    );

    let compared = compare_trees(&[tree_a, tree_b]);
    let filtered = filter_diff(&compared).unwrap();

    assert_eq!(filtered.name, "root");
    // f1.txt should be kept (it differs)
    let f1 = filtered.children.get("f1.txt").unwrap();
    assert!(!f1.all_equal);
}

#[test]
fn test_collapse_diff_non_mixed() {
    // Directory where all per-tree statuses are Included/Excluded (no Mixed)
    let tree_a = make_dir(
        "root",
        NodeStatus::DirectoryMixed,
        vec![make_dir(
            "dir",
            NodeStatus::DirectoryIncluded,
            vec![make_file("f1.txt", NodeStatus::FileIncluded)],
        )],
    );
    let tree_b = make_dir(
        "root",
        NodeStatus::DirectoryMixed,
        vec![make_dir(
            "dir",
            NodeStatus::DirectoryExcluded,
            vec![make_file("f1.txt", NodeStatus::FileExcluded)],
        )],
    );

    let compared = compare_trees(&[tree_a, tree_b]);
    let filtered = filter_diff(&compared).unwrap();
    let dir = filtered.children.get("dir").unwrap();

    assert!(!dir.all_equal);
    assert!(
        dir.can_collapse(),
        "dir with only Inc/Exc should be collapsible"
    );
}

#[test]
fn test_collapse_diff_mixed() {
    let tree_a = make_dir(
        "root",
        NodeStatus::DirectoryMixed,
        vec![make_dir(
            "dir",
            NodeStatus::DirectoryMixed,
            vec![
                make_file("f1.txt", NodeStatus::FileIncluded),
                make_file("f2.txt", NodeStatus::FileExcluded),
            ],
        )],
    );
    let tree_b = make_dir(
        "root",
        NodeStatus::DirectoryMixed,
        vec![make_dir(
            "dir",
            NodeStatus::DirectoryIncluded,
            vec![
                make_file("f1.txt", NodeStatus::FileIncluded),
                make_file("f2.txt", NodeStatus::FileIncluded),
            ],
        )],
    );

    let compared = compare_trees(&[tree_a, tree_b]);
    let filtered = filter_diff(&compared).unwrap();
    let dir = filtered.children.get("dir").unwrap();

    assert!(
        !dir.can_collapse(),
        "dir with Mixed status should NOT be collapsible"
    );
}

#[test]
fn test_find_common_ancestor_identical() {
    let a = PathBuf::from("/home/user/docs");
    let b = PathBuf::from("/home/user/docs");
    assert_eq!(
        find_common_ancestor(&[a, b]),
        PathBuf::from("/home/user/docs")
    );
}

#[test]
fn test_find_common_ancestor_parent_child() {
    let a = PathBuf::from("/home/user/docs");
    let b = PathBuf::from("/home/user");
    assert_eq!(find_common_ancestor(&[a, b]), PathBuf::from("/home/user"));
}

#[test]
fn test_find_common_ancestor_siblings() {
    let a = PathBuf::from("/home/user/docs");
    let b = PathBuf::from("/home/user/photos");
    assert_eq!(find_common_ancestor(&[a, b]), PathBuf::from("/home/user"));
}

#[test]
fn test_find_common_ancestor_disjoint() {
    let a = PathBuf::from("/etc");
    let b = PathBuf::from("/home");
    assert_eq!(find_common_ancestor(&[a, b]), PathBuf::from("/"));
}

#[test]
fn test_find_common_ancestor_three_way() {
    let a = PathBuf::from("/a/b/c");
    let b = PathBuf::from("/a/b/d");
    let c = PathBuf::from("/a/b/e/f");
    assert_eq!(find_common_ancestor(&[a, b, c]), PathBuf::from("/a/b"));
}

#[test]
fn test_reroot_tree_same_path() {
    let mut tree = make_dir("docs", NodeStatus::DirectoryMixed, vec![]);
    tree.path = PathBuf::from("/home/user/docs");
    let rerooted = reroot_tree(tree, &PathBuf::from("/home/user/docs"));
    assert_eq!(rerooted.name, "docs");
    assert_eq!(rerooted.path, PathBuf::from("/home/user/docs"));
}

#[test]
fn test_reroot_tree_narrower_under_wider() {
    let mut tree = make_dir("docs", NodeStatus::DirectoryMixed, vec![]);
    tree.path = PathBuf::from("/home/user/docs");
    let rerooted = reroot_tree(tree, &PathBuf::from("/home"));

    assert_eq!(rerooted.name, "user");
    assert_eq!(rerooted.path, PathBuf::from("/home/user"));
    assert_eq!(rerooted.status, NodeStatus::DirectoryMixed);

    let child = rerooted.children.get("docs").unwrap();
    assert_eq!(child.name, "docs");
    assert_eq!(child.path, PathBuf::from("/home/user/docs"));
    assert_eq!(child.status, NodeStatus::DirectoryMixed);
}

#[test]
fn test_reroot_tree_deep_nesting() {
    let mut tree = make_dir("target", NodeStatus::DirectoryIncluded, vec![]);
    tree.path = PathBuf::from("/a/b/c/target");
    let rerooted = reroot_tree(tree, &PathBuf::from("/a"));

    assert_eq!(rerooted.name, "b");
    let c = rerooted.children.get("c").unwrap();
    assert_eq!(c.name, "c");
    let target = c.children.get("target").unwrap();
    assert_eq!(target.name, "target");
    assert_eq!(target.status, NodeStatus::DirectoryIncluded);
}

#[test]
fn test_load_trees_different_base_paths() {
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let path_a = dir.path().join("narrow.json");
    let path_b = dir.path().join("wide.json");

    // Tree A: rooted at /home/user/docs with a file
    let mut tree_a = make_dir(
        "docs",
        NodeStatus::DirectoryMixed,
        vec![make_file("f1.txt", NodeStatus::FileIncluded)],
    );
    tree_a.path = PathBuf::from("/home/user/docs");

    // Tree B: rooted at /home with the same file
    let mut tree_b = make_dir(
        "home",
        NodeStatus::DirectoryMixed,
        vec![make_dir(
            "user",
            NodeStatus::DirectoryMixed,
            vec![make_dir(
                "docs",
                NodeStatus::DirectoryMixed,
                vec![make_file("f1.txt", NodeStatus::FileIncluded)],
            )],
        )],
    );
    tree_b.path = PathBuf::from("/home");

    tree_a.save_to_file(&path_a).unwrap();
    tree_b.save_to_file(&path_b).unwrap();

    let trees = load_trees(&[path_a, path_b]).unwrap();
    assert_eq!(trees.len(), 2);
    // Both should now be rooted at "/" (the common ancestor's parent)
    assert_eq!(trees[0].name, "/");
    assert_eq!(trees[1].name, "/");
}

#[test]
fn test_load_trees_root_name_mismatch() {
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let path_a = dir.path().join("a.json");
    let path_b = dir.path().join("b.json");

    let tree_a = make_dir("/root_a", NodeStatus::DirectoryIncluded, vec![]);
    let tree_b = make_dir("/root_b", NodeStatus::DirectoryIncluded, vec![]);

    tree_a.save_to_file(&path_a).unwrap();
    tree_b.save_to_file(&path_b).unwrap();

    // Both trees are absolute paths, so they share "/" as common ancestor.
    // After re-rooting, both are wrapped under a synthetic "/" root node.
    let trees = load_trees(&[path_a, path_b]).unwrap();
    assert_eq!(trees.len(), 2);
    assert_eq!(trees[0].name, "/");
    assert_eq!(trees[1].name, "/");
}

// ── Bug-reproduction tests for re-rooting when common ancestor is `/` ──

/// A tree with path `/home/user/Documents` re-rooted to `/` should produce
/// a root with name `"/"` (matching the root component), not `"home"`.
#[test]
fn test_reroot_tree_under_root() {
    let mut tree = make_dir("Documents", NodeStatus::DirectoryMixed, vec![]);
    tree.path = PathBuf::from("/home/user/Documents");

    let rerooted = reroot_tree(tree, &PathBuf::from("/"));

    // The root should be named "/" because the new_base is the filesystem root
    assert_eq!(
        rerooted.name, "/",
        "Tree re-rooted under '/' should have root name '/', got '{}'",
        rerooted.name
    );
    assert_eq!(
        rerooted.path,
        PathBuf::from("/"),
        "Tree re-rooted under '/' should have path '/'"
    );

    // The original tree content should be nested underneath
    let home = rerooted.children.get("home").unwrap();
    assert_eq!(home.name, "home");
    assert_eq!(home.path, PathBuf::from("/home"));

    let user = home.children.get("user").unwrap();
    assert_eq!(user.name, "user");
    assert_eq!(user.path, PathBuf::from("/home/user"));

    let docs = user.children.get("Documents").unwrap();
    assert_eq!(docs.name, "Documents");
    assert_eq!(docs.path, PathBuf::from("/home/user/Documents"));
}

/// A tree already at path `/` passed through `reroot_tree` with new_base `/`
/// should be returned unchanged (root name `"/"`, path `"/"`).
#[test]
fn test_reroot_tree_root_identity() {
    let mut tree = make_dir("/", NodeStatus::DirectoryMixed, vec![]);
    tree.path = PathBuf::from("/");

    let rerooted = reroot_tree(tree, &PathBuf::from("/"));

    assert_eq!(rerooted.name, "/");
    assert_eq!(rerooted.path, PathBuf::from("/"));
}

/// Two trees re-rooted under `/` should have the same root name.
/// One tree starts at `/`, the other at `/home/user/Documents`.
#[test]
fn test_reroot_tree_under_root_matches_identity() {
    // Tree A: already at "/"
    let mut tree_root = make_dir("/", NodeStatus::DirectoryMixed, vec![]);
    tree_root.path = PathBuf::from("/");

    // Tree B: at "/home/user/Documents"
    let mut tree_nested = make_dir("Documents", NodeStatus::DirectoryMixed, vec![]);
    tree_nested.path = PathBuf::from("/home/user/Documents");

    let rerooted_a = reroot_tree(tree_root, &PathBuf::from("/"));
    let rerooted_b = reroot_tree(tree_nested, &PathBuf::from("/"));

    assert_eq!(
        rerooted_a.name, rerooted_b.name,
        "Both re-rooted trees should share the same root name, but got '{}' vs '{}'",
        rerooted_a.name, rerooted_b.name
    );
}

/// Integration test reproducing the exact bug scenario:
/// - 1.json: root path = "/home/user/Documents"
/// - 2.json: root path = "/"
/// load_trees should succeed because both trees overlap at "/".
#[test]
fn test_load_trees_with_root_ancestor() {
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let path_a = dir.path().join("1.json");
    let path_b = dir.path().join("2.json");

    // Tree with root path "/home/user/Documents" (narrower view)
    let mut tree_a = make_dir(
        "Documents",
        NodeStatus::DirectoryMixed,
        vec![make_file("f1.txt", NodeStatus::FileIncluded)],
    );
    tree_a.path = PathBuf::from("/home/user/Documents");

    // Tree with root path "/" (full filesystem view)
    let mut tree_b = make_dir(
        "/",
        NodeStatus::DirectoryMixed,
        vec![make_dir(
            "home",
            NodeStatus::DirectoryMixed,
            vec![make_dir(
                "user",
                NodeStatus::DirectoryMixed,
                vec![make_dir(
                    "Documents",
                    NodeStatus::DirectoryMixed,
                    vec![make_file("f1.txt", NodeStatus::FileIncluded)],
                )],
            )],
        )],
    );
    tree_b.path = PathBuf::from("/");

    tree_a.save_to_file(&path_a).unwrap();
    tree_b.save_to_file(&path_b).unwrap();

    // This is the call that fails with the bug — it should succeed
    let trees = load_trees(&[path_a, path_b]).unwrap();

    assert_eq!(trees.len(), 2);
    // Both should have the same root name (the common ancestor /)
    assert_eq!(
        trees[0].name, trees[1].name,
        "Both re-rooted trees must have the same root name, but got '{}' vs '{}'",
        trees[0].name, trees[1].name
    );
    // The common root name should be "/" since both paths share "/" as ancestor
    assert_eq!(trees[0].name, "/");
}

#[test]
fn test_less_than_two_trees_errors() {
    let result = load_trees(&[]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("At least 2 trees"));
}

#[test]
fn test_empty_children_display() {
    // Edge case: a ComparedNode with no children
    let node = ComparedNode {
        name: "empty".to_string(),
        per_tree_status: vec![PerTreeStatus::Included, PerTreeStatus::Missing],
        children: BTreeMap::new(),
        all_equal: false,
        is_directory: false,
    };

    assert!(!node.all_equal);
    assert_eq!(node.per_tree_status.len(), 2);
    assert!(node.children.is_empty());
}

#[test]
fn test_empty_dir_vs_mixed_dir_in_compare() {
    // Tree A: empty included directory
    // Tree B: mixed directory (has an excluded child)
    let tree_a = make_dir("root", NodeStatus::DirectoryIncluded, vec![]);
    let tree_b = make_dir(
        "root",
        NodeStatus::DirectoryMixed,
        vec![make_file("f1.txt", NodeStatus::FileExcluded)],
    );

    let compared = compare_trees(&[tree_a, tree_b]);

    assert!(!compared.all_equal);
    assert_eq!(
        compared.per_tree_status,
        vec![PerTreeStatus::Included, PerTreeStatus::Mixed]
    );
}

#[test]
fn test_empty_dir_vs_included_dir_in_compare() {
    // Tree A: empty included directory
    // Tree B: included directory with an included child
    let tree_a = make_dir("root", NodeStatus::DirectoryIncluded, vec![]);
    let tree_b = make_dir(
        "root",
        NodeStatus::DirectoryIncluded,
        vec![make_file("f1.txt", NodeStatus::FileIncluded)],
    );

    let compared = compare_trees(&[tree_a, tree_b]);

    // Both roots are DirectoryIncluded → both map to Included → statuses equal
    // But children differ: tree_a has no f1.txt, tree_b does
    assert!(!compared.all_equal);
    let f1 = compared.children.get("f1.txt").unwrap();
    assert_eq!(
        f1.per_tree_status,
        vec![PerTreeStatus::Missing, PerTreeStatus::Included]
    );
}

#[test]
fn test_both_empty_dirs_equal_in_compare() {
    let tree_a = make_dir("root", NodeStatus::DirectoryIncluded, vec![]);
    let tree_b = make_dir("root", NodeStatus::DirectoryIncluded, vec![]);

    let compared = compare_trees(&[tree_a, tree_b]);

    assert!(compared.all_equal);
    assert!(compared.children.is_empty());
}
