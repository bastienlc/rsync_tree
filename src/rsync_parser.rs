use std::path::PathBuf;
use std::str::FromStr;

use crate::rsync_types::*;

/// Parse a character into an AttributeStatus
fn parse_attribute_char(c: char) -> AttributeStatus {
    match c {
        '.' => AttributeStatus::NoChange,
        '+' => AttributeStatus::NewlyCreated,
        ' ' => AttributeStatus::Identical,
        '?' => AttributeStatus::Unknown,
        _ => AttributeStatus::Changed,
    }
}

/// Parse the attributes portion of the itemize string
fn parse_attributes(attr_str: &str) -> Result<Attributes, String> {
    if attr_str.len() != 9 {
        return Err(format!(
            "Attributes string must be 9 characters, got {}",
            attr_str.len()
        ));
    }

    let chars: Vec<char> = attr_str.chars().collect();

    Ok(Attributes {
        checksum: parse_attribute_char(chars[0]),
        size: parse_attribute_char(chars[1]),
        time: parse_attribute_char(chars[2]),
        permissions: parse_attribute_char(chars[3]),
        owner: parse_attribute_char(chars[4]),
        group: parse_attribute_char(chars[5]),
        reserved: parse_attribute_char(chars[6]),
        acl: parse_attribute_char(chars[7]),
        extended_attrs: parse_attribute_char(chars[8]),
    })
}

/// Parse symlink and hardlink targets from path string
fn parse_link_path(
    path_str: &str,
    file_type: Option<FileType>,
    update_type: UpdateType,
) -> (PathBuf, Option<PathBuf>) {
    // Handle symlinks which have format "path/to/symlink -> path/to/target"
    if file_type == Some(FileType::Symlink) && path_str.contains(" -> ") {
        let parts: Vec<&str> = path_str.splitn(2, " -> ").collect();
        if parts.len() == 2 {
            return (PathBuf::from(parts[0]), Some(PathBuf::from(parts[1])));
        }
    }

    // Handle hardlinks "path/to/hardlink => path/to/target"
    if update_type == UpdateType::HardLink && path_str.contains(" => ") {
        let parts: Vec<&str> = path_str.splitn(2, " => ").collect();
        if parts.len() == 2 {
            return (PathBuf::from(parts[0]), Some(PathBuf::from(parts[1])));
        }
    }

    (PathBuf::from(path_str), None)
}

/// Parse a single line of rsync --itemize-changes output
pub fn parse_line(line: &str) -> ParseResult {
    let line = line.trim();

    if line.is_empty() {
        return ParseResult::Empty;
    }

    // rsync itemize format: YXcstpoguax path
    // Where Y = update type, X = file type, cstpoguax = attributes
    if line.len() < 11 {
        return ParseResult::InvalidFormat(line.to_string());
    }

    let itemize_part = &line[..11];
    let remaining = &line[11..];

    // Parse update type (first character)
    let update_type_char = itemize_part.chars().next().unwrap();
    let update_type = match UpdateType::from_str(&update_type_char.to_string()) {
        Ok(ut) => ut,
        Err(_) => return ParseResult::InvalidFormat(line.to_string()),
    };

    // Handle special case for messages (like "*deleting")
    if update_type == UpdateType::Message {
        let message = remaining.trim().to_string();
        return ParseResult::Item(RsyncItem {
            update_type,
            file_type: None,
            attributes: None,
            path: PathBuf::new(),
            link_target: None,
            message: Some(message),
        });
    }

    // Parse file type (second character)
    let file_type_char = itemize_part.chars().nth(1).unwrap();
    let file_type = match FileType::from_str(&file_type_char.to_string()) {
        Ok(ft) => Some(ft),
        Err(_) => return ParseResult::InvalidFormat(line.to_string()),
    };

    // Parse attributes (characters 2-10)
    let attr_str = &itemize_part[2..];
    let attributes = match parse_attributes(attr_str) {
        Ok(attrs) => Some(attrs),
        Err(_) => return ParseResult::InvalidFormat(line.to_string()),
    };

    // Parse the path (everything after the itemize part)
    let path_str = remaining.trim();
    if path_str.is_empty() {
        return ParseResult::InvalidFormat(line.to_string());
    }

    let (path, link_target) = parse_link_path(path_str, file_type.clone(), update_type.clone());

    ParseResult::Item(RsyncItem {
        update_type,
        file_type,
        attributes,
        path,
        link_target,
        message: None,
    })
}

/// Parse multiple lines of rsync output
pub fn parse_rsync_output(output: &str) -> Vec<ParseResult> {
    output.lines().map(parse_line).collect()
}
