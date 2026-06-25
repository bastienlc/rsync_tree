use std::collections::BTreeMap;

use crate::compare::{
    ComparedNode, PerTreeStatus, compare_trees, filter_diff, load_trees, status_to_compare_status,
};
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
        status_to_compare_status(NodeStatus::DirectoryStandalone),
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
    let tree_b = make_dir("root", NodeStatus::DirectoryStandalone, vec![]);

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
fn test_load_trees_root_name_mismatch() {
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let path_a = dir.path().join("a.json");
    let path_b = dir.path().join("b.json");

    let tree_a = make_dir("root_a", NodeStatus::DirectoryIncluded, vec![]);
    let tree_b = make_dir("root_b", NodeStatus::DirectoryIncluded, vec![]);

    tree_a.save_to_file(&path_a).unwrap();
    tree_b.save_to_file(&path_b).unwrap();

    let result = load_trees(&[path_a, path_b]);
    assert!(result.is_err(), "Root name mismatch should be an error");
    assert!(
        result.unwrap_err().contains("Root name mismatch"),
        "Error should mention root name mismatch"
    );
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
