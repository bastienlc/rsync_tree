use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

use crate::rsync_types::{FileType, ParseResult, RsyncItem, UpdateType};
use crate::tree::NodeStatus;
use crate::tree_builder::build_tree_from_rsync_output;

#[test]
fn test_build_simple_tree() {
    // Create a temporary directory structure
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    // Create some files and directories
    fs::create_dir_all(base_path.join("included_dir")).unwrap();
    fs::write(base_path.join("included_dir/file1.txt"), "content1").unwrap();
    fs::write(base_path.join("included_dir/file2.txt"), "content2").unwrap();

    fs::create_dir_all(base_path.join("excluded_dir")).unwrap();
    fs::write(base_path.join("excluded_dir/file3.txt"), "content3").unwrap();

    // Create parse results that only include some items
    let parse_results = vec![
        ParseResult::Item(RsyncItem {
            update_type: UpdateType::Received,
            file_type: Some(FileType::Directory),
            attributes: None,
            path: PathBuf::from("included_dir/"),
            link_target: None,
            message: None,
        }),
        ParseResult::Item(RsyncItem {
            update_type: UpdateType::Received,
            file_type: Some(FileType::File),
            attributes: None,
            path: PathBuf::from("included_dir/file1.txt"),
            link_target: None,
            message: None,
        }),
    ];

    let tree = build_tree_from_rsync_output(parse_results, base_path, false).unwrap();

    // Check that the tree has the expected structure
    assert_eq!(tree.children.len(), 2); // included_dir and excluded_dir
    assert!(tree.children.contains_key("included_dir"));
    assert!(tree.children.contains_key("excluded_dir"));

    // Check included directory
    let included_dir = &tree.children["included_dir"];
    assert_eq!(included_dir.status, NodeStatus::DirectoryMixed); // Has both included and excluded files

    // Check excluded directory
    let excluded_dir = &tree.children["excluded_dir"];
    assert_eq!(excluded_dir.status, NodeStatus::DirectoryExcluded);
}

#[test]
fn test_empty_parse_results() {
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    fs::write(base_path.join("file.txt"), "content").unwrap();

    let parse_results = vec![];
    let tree = build_tree_from_rsync_output(parse_results, base_path, false).unwrap();

    // Should have excluded items from filesystem scan
    assert_eq!(tree.children.len(), 1);
    assert!(tree.children.contains_key("file.txt"));
    assert_eq!(tree.children["file.txt"].status, NodeStatus::FileExcluded);
}

#[test]
fn test_standalone_directory() {
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    fs::create_dir_all(base_path.join("included_dir")).unwrap();
    fs::write(base_path.join("included_dir/file.txt"), "content").unwrap();

    let parse_results = vec![ParseResult::Item(RsyncItem {
        update_type: UpdateType::Received,
        file_type: Some(FileType::Directory),
        attributes: None,
        path: PathBuf::from("included_dir/"),
        link_target: None,
        message: None,
    })];
    let tree = build_tree_from_rsync_output(parse_results, base_path, false).unwrap();

    let included_dir = &tree.children["included_dir"];
    assert_eq!(included_dir.status, NodeStatus::DirectoryStandalone);
    assert_eq!(included_dir.children.len(), 1);
    assert_eq!(
        included_dir.children["file.txt"].status,
        NodeStatus::FileExcluded
    );
}

#[test]
fn test_size_collection() {
    // Create a temporary directory structure with files of known sizes
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    // Create some files with specific content
    fs::create_dir_all(base_path.join("test_dir")).unwrap();
    fs::write(base_path.join("test_dir/small.txt"), "12345").unwrap(); // 5 bytes
    fs::write(base_path.join("test_dir/large.txt"), "x".repeat(1000)).unwrap(); // 1000 bytes
    fs::write(base_path.join("excluded.txt"), "excluded").unwrap(); // Should not be counted

    // Create parse results that include only some files
    let parse_results = vec![
        ParseResult::Item(RsyncItem {
            update_type: UpdateType::Received,
            file_type: Some(FileType::Directory),
            attributes: None,
            path: PathBuf::from("test_dir/"),
            link_target: None,
            message: None,
        }),
        ParseResult::Item(RsyncItem {
            update_type: UpdateType::Received,
            file_type: Some(FileType::File),
            attributes: None,
            path: PathBuf::from("test_dir/small.txt"),
            link_target: None,
            message: None,
        }),
        ParseResult::Item(RsyncItem {
            update_type: UpdateType::Received,
            file_type: Some(FileType::File),
            attributes: None,
            path: PathBuf::from("test_dir/large.txt"),
            link_target: None,
            message: None,
        }),
    ];

    // Test with size collection enabled
    let tree_with_sizes =
        build_tree_from_rsync_output(parse_results.clone(), base_path, true).unwrap();

    // Check that sizes were collected
    let test_dir = &tree_with_sizes.children["test_dir"];
    assert_eq!(test_dir.size, Some(1005)); // 5 + 1000 bytes

    let small_file = &test_dir.children["small.txt"];
    assert_eq!(small_file.size, Some(5));

    let large_file = &test_dir.children["large.txt"];
    assert_eq!(large_file.size, Some(1000));

    // Excluded file should not have size collected (it's not included in rsync output)
    let excluded_file = &tree_with_sizes.children["excluded.txt"];
    assert_eq!(excluded_file.size, None);

    // Test with size collection disabled
    let tree_without_sizes = build_tree_from_rsync_output(parse_results, base_path, false).unwrap();

    // Check that sizes were not collected
    let test_dir = &tree_without_sizes.children["test_dir"];
    assert_eq!(test_dir.size, None);

    let small_file = &test_dir.children["small.txt"];
    assert_eq!(small_file.size, None);
}

#[test]
fn test_percentage_display() {
    // Create a temporary directory structure with files of known sizes
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    // Create files with specific sizes for easy percentage calculation
    fs::create_dir_all(base_path.join("test_dir")).unwrap();
    fs::write(base_path.join("test_dir/file1.txt"), "a".repeat(25)).unwrap(); // 25 bytes = 25%
    fs::write(base_path.join("test_dir/file2.txt"), "b".repeat(75)).unwrap(); // 75 bytes = 75%
    // Total: 100 bytes

    // Create parse results that include these files
    let parse_results = vec![
        ParseResult::Item(RsyncItem {
            update_type: UpdateType::Received,
            file_type: Some(FileType::Directory),
            attributes: None,
            path: PathBuf::from("test_dir/"),
            link_target: None,
            message: None,
        }),
        ParseResult::Item(RsyncItem {
            update_type: UpdateType::Received,
            file_type: Some(FileType::File),
            attributes: None,
            path: PathBuf::from("test_dir/file1.txt"),
            link_target: None,
            message: None,
        }),
        ParseResult::Item(RsyncItem {
            update_type: UpdateType::Received,
            file_type: Some(FileType::File),
            attributes: None,
            path: PathBuf::from("test_dir/file2.txt"),
            link_target: None,
            message: None,
        }),
    ];

    let tree = build_tree_from_rsync_output(parse_results, base_path, true).unwrap();

    // Render the tree with sizes
    let mut output = Vec::new();
    crate::display::render_tree(&tree, &mut output, false, false, false, true)
        .unwrap();
    let rendered = String::from_utf8_lossy(&output);

    // Check that percentages are displayed correctly
    assert!(
        rendered.contains("25.0%"),
        "Should show 25% for file1.txt, got: {}",
        rendered
    );
    assert!(
        rendered.contains("75.0%"),
        "Should show 75% for file2.txt, got: {}",
        rendered
    );
    assert!(
        rendered.contains("100 B"),
        "Should show total size, got: {}",
        rendered
    );
}
