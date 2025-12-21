use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A prefix-based data structure to efficiently check if a path is included
#[derive(Debug)]
pub struct PathTrie {
    /// Map from path component to child trie
    children: HashMap<String, PathTrie>,
    /// Whether this exact path is included
    is_included: bool,
}

impl PathTrie {
    pub fn new() -> Self {
        Self {
            children: HashMap::new(),
            is_included: false,
        }
    }

    pub fn insert(&mut self, path: &Path) {
        let mut current = self;
        for component in path.components() {
            if let Some(component_str) = component.as_os_str().to_str() {
                current = current
                    .children
                    .entry(component_str.to_string())
                    .or_insert_with(PathTrie::new);
            }
        }
        current.is_included = true;
    }

    pub fn is_path_included(&self, path: &Path) -> bool {
        let mut current = self;
        for component in path.components() {
            if let Some(component_str) = component.as_os_str().to_str() {
                if let Some(child) = current.children.get(component_str) {
                    current = child;
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        current.is_included
    }
}

/// Normalize a path by removing trailing slashes and making it relative to base
pub fn normalize_path(path: &Path, base_path: &Path) -> PathBuf {
    // Normalize the path to be relative to base_path
    let path = if path.is_absolute() {
        path.strip_prefix(base_path).unwrap_or(path).to_path_buf()
    } else {
        path.to_path_buf()
    };

    // Remove trailing slash for directories
    if path.to_string_lossy().ends_with('/') {
        PathBuf::from(path.to_string_lossy().trim_end_matches('/'))
    } else {
        path
    }
}

/// Get the display name for a path (file name or full path if no file name)
pub fn get_display_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_else(|| path.as_os_str())
        .to_string_lossy()
        .to_string()
}
