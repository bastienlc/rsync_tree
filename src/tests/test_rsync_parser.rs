use std::path::PathBuf;

use crate::rsync_parser::{parse_line, parse_rsync_output};
use crate::rsync_types::{AttributeStatus, FileType, ParseResult, UpdateType};

#[test]
fn test_parse_directory_creation() {
    let line = "cd+++++++++ home/user/";
    let result = parse_line(line);

    match result {
        ParseResult::Item(item) => {
            assert_eq!(item.update_type, UpdateType::LocalChange);
            assert_eq!(item.file_type, Some(FileType::Directory));
            assert_eq!(item.path, PathBuf::from("home/user/"));

            let attrs = item.attributes.unwrap();
            assert_eq!(attrs.checksum, AttributeStatus::NewlyCreated);
            assert_eq!(attrs.size, AttributeStatus::NewlyCreated);
            assert_eq!(attrs.time, AttributeStatus::NewlyCreated);
        }
        _ => panic!("Expected successful parse"),
    }
}

#[test]
fn test_parse_file_transfer() {
    let line = ">f+++++++++ home/user/.some_folder";
    let result = parse_line(line);

    match result {
        ParseResult::Item(item) => {
            assert_eq!(item.update_type, UpdateType::Received);
            assert_eq!(item.file_type, Some(FileType::File));
            assert_eq!(item.path, PathBuf::from("home/user/.some_folder"));
        }
        _ => panic!("Expected successful parse"),
    }
}

#[test]
fn test_parse_deletion_message() {
    let line = "*deleting   some/file.txt";
    let result = parse_line(line);

    match result {
        ParseResult::Item(item) => {
            assert_eq!(item.update_type, UpdateType::Message);
            assert_eq!(item.file_type, None);
            assert_eq!(item.message, Some("some/file.txt".to_string()));
        }
        _ => panic!("Expected successful parse"),
    }
}

#[test]
fn test_parse_no_changes() {
    let line = ".f..t...... some/file.txt";
    let result = parse_line(line);

    match result {
        ParseResult::Item(item) => {
            assert_eq!(item.update_type, UpdateType::NotUpdated);
            assert_eq!(item.file_type, Some(FileType::File));

            let attrs = item.attributes.unwrap();
            assert_eq!(attrs.checksum, AttributeStatus::NoChange);
            assert_eq!(attrs.size, AttributeStatus::NoChange);
            assert_eq!(attrs.time, AttributeStatus::Changed); // 't' indicates time change
            assert_eq!(attrs.permissions, AttributeStatus::NoChange);
        }
        _ => panic!("Expected successful parse"),
    }
}

#[test]
fn test_parse_invalid_format() {
    let line = "invalid format";
    let result = parse_line(line);
    assert_eq!(
        result,
        ParseResult::InvalidFormat("invalid format".to_string())
    );
}

#[test]
fn test_parse_empty_line() {
    let line = "";
    let result = parse_line(line);
    assert_eq!(result, ParseResult::Empty);
}

#[test]
fn test_parse_multiple_lines() {
    let output = r#"cd+++++++++ home/user/
>f+++++++++ home/user/.some_folder
>f+++++++++ home/user/.some_file
*deleting   old/file.txt"#;

    let results = parse_rsync_output(output);
    assert_eq!(results.len(), 4);

    // Check first item (directory)
    match &results[0] {
        ParseResult::Item(item) => {
            assert_eq!(item.update_type, UpdateType::LocalChange);
            assert_eq!(item.file_type, Some(FileType::Directory));
        }
        _ => panic!("Expected item"),
    }

    // Check last item (deletion message)
    match &results[3] {
        ParseResult::Item(item) => {
            assert_eq!(item.update_type, UpdateType::Message);
            assert_eq!(item.message, Some("old/file.txt".to_string()));
        }
        _ => panic!("Expected item"),
    }
}

#[test]
fn test_parse_symlink() {
    let line = "cL+++++++++ path/to/symlink -> path/to/target";
    let result = parse_line(line);

    match result {
        ParseResult::Item(item) => {
            assert_eq!(item.update_type, UpdateType::LocalChange);
            assert_eq!(item.file_type, Some(FileType::Symlink));
            assert_eq!(item.path, PathBuf::from("path/to/symlink"));
            assert_eq!(item.link_target, Some(PathBuf::from("path/to/target")));
            assert_eq!(item.message, None);

            let attrs = item.attributes.unwrap();
            assert_eq!(attrs.checksum, AttributeStatus::NewlyCreated);
        }
        _ => panic!("Expected successful parse"),
    }
}

#[test]
fn test_parse_symlink_no_target() {
    // Test case where symlink doesn't have " -> " format (edge case)
    let line = "cL+++++++++ path/to/symlink";
    let result = parse_line(line);

    match result {
        ParseResult::Item(item) => {
            assert_eq!(item.update_type, UpdateType::LocalChange);
            assert_eq!(item.file_type, Some(FileType::Symlink));
            assert_eq!(item.path, PathBuf::from("path/to/symlink"));
            assert_eq!(item.link_target, None);
            assert_eq!(item.message, None);
        }
        _ => panic!("Expected successful parse"),
    }
}

#[test]
fn test_parse_regular_file_with_arrow() {
    // Ensure regular files with " -> " in name don't get treated as symlinks
    let line = "cf+++++++++ path/to/file -> not a target";
    let result = parse_line(line);

    match result {
        ParseResult::Item(item) => {
            assert_eq!(item.update_type, UpdateType::LocalChange);
            assert_eq!(item.file_type, Some(FileType::File));
            assert_eq!(item.path, PathBuf::from("path/to/file -> not a target"));
            assert_eq!(item.link_target, None);
            assert_eq!(item.message, None);
        }
        _ => panic!("Expected successful parse"),
    }
}

#[test]
fn test_parse_hardlink() {
    let line = "hf+++++++++ path/to/hardlink => path/to/target";
    let result = parse_line(line);

    match result {
        ParseResult::Item(item) => {
            assert_eq!(item.update_type, UpdateType::HardLink);
            assert_eq!(item.file_type, Some(FileType::File));
            assert_eq!(item.path, PathBuf::from("path/to/hardlink"));
            assert_eq!(item.link_target, Some(PathBuf::from("path/to/target")));
            assert_eq!(item.message, None);

            let attrs = item.attributes.unwrap();
            assert_eq!(attrs.checksum, AttributeStatus::NewlyCreated);
        }
        _ => panic!("Expected successful parse"),
    }
}

#[test]
fn test_parse_hardlink_no_target() {
    // Test case where hardlink doesn't have " => " format (edge case)
    let line = "hf+++++++++ path/to/hardlink";
    let result = parse_line(line);

    match result {
        ParseResult::Item(item) => {
            assert_eq!(item.update_type, UpdateType::HardLink);
            assert_eq!(item.file_type, Some(FileType::File));
            assert_eq!(item.path, PathBuf::from("path/to/hardlink"));
            assert_eq!(item.link_target, None);
            assert_eq!(item.message, None);
        }
        _ => panic!("Expected successful parse"),
    }
}

#[test]
fn test_parse_regular_file_with_hardlink_arrow() {
    // Ensure regular files with " => " in name don't get treated as hardlinks when not marked as hardlink
    let line = "cf+++++++++ path/to/file => not a target";
    let result = parse_line(line);

    match result {
        ParseResult::Item(item) => {
            assert_eq!(item.update_type, UpdateType::LocalChange);
            assert_eq!(item.file_type, Some(FileType::File));
            assert_eq!(item.path, PathBuf::from("path/to/file => not a target"));
            assert_eq!(item.link_target, None);
            assert_eq!(item.message, None);
        }
        _ => panic!("Expected successful parse"),
    }
}
