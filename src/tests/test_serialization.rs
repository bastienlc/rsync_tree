use tempfile::tempdir;

use crate::tree::{NodeStatus, Tree};

/// Helper: build a small in-memory tree with a variety of statuses and sizes.
fn sample_tree() -> Tree {
    let mut root = Tree::new("root".into(), "root".into(), NodeStatus::DirectoryMixed);
    root.size = Some(300);

    let mut child_a = Tree::new("a".into(), "root/a".into(), NodeStatus::DirectoryIncluded);
    child_a.size = Some(200);

    let mut file_1 = Tree::new(
        "f1.txt".into(),
        "root/a/f1.txt".into(),
        NodeStatus::FileIncluded,
    );
    file_1.size = Some(100);
    child_a.add_child(file_1);

    let file_2 = Tree::new(
        "f2.txt".into(),
        "root/a/f2.txt".into(),
        NodeStatus::FileExcluded,
    );
    child_a.add_child(file_2);

    root.add_child(child_a);

    let mut child_b = Tree::new("b".into(), "root/b".into(), NodeStatus::DirectoryExcluded);
    child_b.size = None; // excluded directories don't compute sizes
    root.add_child(child_b);

    let standalone = Tree::new(
        "standalone".into(),
        "root/standalone".into(),
        NodeStatus::DirectoryStandalone,
    );
    root.add_child(standalone);

    root
}

#[test]
fn roundtrip_json_equality() {
    let tree = sample_tree();
    let json = serde_json::to_string(&tree).unwrap();
    let restored: Tree = serde_json::from_str(&json).unwrap();
    assert_eq!(tree, restored);
}

#[test]
fn save_and_load_file() {
    let tree = sample_tree();
    let dir = tempdir().unwrap();
    let path = dir.path().join("tree.json");

    tree.save_to_file(&path).unwrap();
    assert!(path.exists());

    let loaded = Tree::load_from_file(&path).unwrap();
    assert_eq!(tree, loaded);
}

#[test]
fn load_nonexistent_file_errors() {
    let result = Tree::load_from_file("nonexistent_file_42.json".as_ref());
    assert!(result.is_err());
}

#[test]
fn all_node_statuses_survive_roundtrip() {
    let statuses = vec![
        NodeStatus::FileExcluded,
        NodeStatus::FileIncluded,
        NodeStatus::DirectoryExcluded,
        NodeStatus::DirectoryStandalone,
        NodeStatus::DirectoryMixed,
        NodeStatus::DirectoryIncluded,
    ];

    for status in statuses {
        let node = Tree::new("test".into(), "test".into(), status);
        let json = serde_json::to_string(&node).unwrap();
        let restored: Tree = serde_json::from_str(&json).unwrap();
        assert_eq!(node.status, restored.status);
        assert_eq!(node, restored);
    }
}

#[test]
fn sizes_survive_roundtrip() {
    let mut node = Tree::new("f".into(), "f".into(), NodeStatus::FileIncluded);
    node.size = Some(42);
    let json = serde_json::to_string(&node).unwrap();
    let restored: Tree = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.size, Some(42));

    node.size = None;
    let json = serde_json::to_string(&node).unwrap();
    let restored: Tree = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.size, None);
}

#[test]
fn deep_nesting_survives_roundtrip() {
    let mut root = Tree::new("root".into(), "root".into(), NodeStatus::DirectoryMixed);
    let mut parent = Tree::new("a".into(), "root/a".into(), NodeStatus::DirectoryIncluded);
    let child = Tree::new("b".into(), "root/a/b".into(), NodeStatus::FileIncluded);
    parent.add_child(child);
    root.add_child(parent);

    let json = serde_json::to_string(&root).unwrap();
    let restored: Tree = serde_json::from_str(&json).unwrap();

    let b = &restored.children["a"].children["b"];
    assert_eq!(b.name, "b");
    assert_eq!(b.status, NodeStatus::FileIncluded);
}

#[test]
fn invalid_json_errors() {
    let result: Result<Tree, _> = serde_json::from_str("not valid json");
    assert!(result.is_err());
}
