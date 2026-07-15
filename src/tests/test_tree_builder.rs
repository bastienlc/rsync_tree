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
fn test_included_dir_with_only_excluded_children() {
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
    assert_eq!(included_dir.status, NodeStatus::DirectoryMixed);
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
    crate::display::render_tree(&tree, &mut output, false, false, false, true).unwrap();
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

#[test]
fn test_empty_included_directory_is_included() {
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    // Create an empty directory
    fs::create_dir_all(base_path.join("empty_dir")).unwrap();

    let parse_results = vec![ParseResult::Item(RsyncItem {
        update_type: UpdateType::Received,
        file_type: Some(FileType::Directory),
        attributes: None,
        path: PathBuf::from("empty_dir/"),
        link_target: None,
        message: None,
    })];
    let tree = build_tree_from_rsync_output(parse_results, base_path, false).unwrap();

    let empty_dir = &tree.children["empty_dir"];
    assert_eq!(
        empty_dir.status,
        NodeStatus::DirectoryIncluded,
        "An empty included directory should be DirectoryIncluded, not DirectoryMixed"
    );
    assert!(empty_dir.children.is_empty());
}

#[test]
fn test_included_dir_with_all_excluded_children_is_mixed() {
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    // Create a directory with a file that will be excluded
    fs::create_dir_all(base_path.join("dir")).unwrap();
    fs::write(base_path.join("dir/file.txt"), "content").unwrap();

    // Only include the directory, not the file
    let parse_results = vec![ParseResult::Item(RsyncItem {
        update_type: UpdateType::Received,
        file_type: Some(FileType::Directory),
        attributes: None,
        path: PathBuf::from("dir/"),
        link_target: None,
        message: None,
    })];
    let tree = build_tree_from_rsync_output(parse_results, base_path, false).unwrap();

    let dir = &tree.children["dir"];
    assert_eq!(
        dir.status,
        NodeStatus::DirectoryMixed,
        "A directory with all children excluded should be DirectoryMixed"
    );
    assert_eq!(dir.children.len(), 1);
    assert_eq!(dir.children["file.txt"].status, NodeStatus::FileExcluded);
}

#[test]
#[cfg(unix)] // symlink creation requires Unix
fn test_symlink_with_links_flag() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    // Create a regular file and a symlink to it
    fs::write(base_path.join("regular.txt"), "hello world").unwrap();
    symlink(base_path.join("regular.txt"), base_path.join("link.txt")).unwrap();

    // Also create a directory and a symlink pointing to it
    fs::create_dir_all(base_path.join("target_dir")).unwrap();
    fs::write(base_path.join("target_dir/nested.txt"), "nested").unwrap();
    symlink(base_path.join("target_dir"), base_path.join("link_to_dir")).unwrap();

    // Simulate rsync --links output: symlinks reported with FileType::Symlink
    let parse_results = vec![
        ParseResult::Item(RsyncItem {
            update_type: UpdateType::LocalChange,
            file_type: Some(FileType::Symlink),
            attributes: None,
            path: PathBuf::from("link.txt"),
            link_target: Some(PathBuf::from("regular.txt")),
            message: None,
        }),
        ParseResult::Item(RsyncItem {
            update_type: UpdateType::LocalChange,
            file_type: Some(FileType::Symlink),
            attributes: None,
            path: PathBuf::from("link_to_dir"),
            link_target: Some(PathBuf::from("target_dir")),
            message: None,
        }),
        // Regular file and directory are also included to ensure they show up
        ParseResult::Item(RsyncItem {
            update_type: UpdateType::Received,
            file_type: Some(FileType::File),
            attributes: None,
            path: PathBuf::from("regular.txt"),
            link_target: None,
            message: None,
        }),
        ParseResult::Item(RsyncItem {
            update_type: UpdateType::Received,
            file_type: Some(FileType::Directory),
            attributes: None,
            path: PathBuf::from("target_dir/"),
            link_target: None,
            message: None,
        }),
        ParseResult::Item(RsyncItem {
            update_type: UpdateType::Received,
            file_type: Some(FileType::File),
            attributes: None,
            path: PathBuf::from("target_dir/nested.txt"),
            link_target: None,
            message: None,
        }),
    ];

    let tree = build_tree_from_rsync_output(parse_results, base_path, true).unwrap();

    // ── Symlink to a regular file ──
    let link_to_file = &tree.children["link.txt"];
    assert_eq!(link_to_file.status, NodeStatus::FileIncluded);
    assert_eq!(link_to_file.link_target, Some(PathBuf::from("regular.txt")));
    // Size should come from symlink_metadata (the inode), not the target file
    let symlink_meta = fs::symlink_metadata(base_path.join("link.txt")).unwrap();
    assert_eq!(link_to_file.size, Some(symlink_meta.len()));
    // The symlink must be a leaf
    assert!(link_to_file.children.is_empty());

    // ── Symlink to a directory ──
    let link_to_dir = &tree.children["link_to_dir"];
    assert_eq!(link_to_dir.status, NodeStatus::FileIncluded);
    assert_eq!(link_to_dir.link_target, Some(PathBuf::from("target_dir")));
    let dir_symlink_meta = fs::symlink_metadata(base_path.join("link_to_dir")).unwrap();
    assert_eq!(link_to_dir.size, Some(dir_symlink_meta.len()));
    // Even though it points to a directory, it must NOT be recursed into
    assert!(link_to_dir.children.is_empty());

    // ── Regular file and directory are unaffected ──
    assert!(tree.children.contains_key("regular.txt"));
    assert_eq!(
        tree.children["regular.txt"].status,
        NodeStatus::FileIncluded
    );
    assert!(tree.children.contains_key("target_dir"));
    assert!(
        tree.children["target_dir"]
            .children
            .contains_key("nested.txt")
    );
}

#[test]
fn test_symlink_not_included_remains_excluded() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    fs::write(base_path.join("data.txt"), "content").unwrap();
    symlink(base_path.join("data.txt"), base_path.join("my_link")).unwrap();

    // No parse results at all — nothing is included
    let tree = build_tree_from_rsync_output(vec![], base_path, true).unwrap();

    let link_node = &tree.children["my_link"];
    assert_eq!(link_node.status, NodeStatus::FileExcluded);
    assert_eq!(link_node.link_target, None);
    // Excluded items don't get sizes collected
    assert_eq!(link_node.size, None);
}
