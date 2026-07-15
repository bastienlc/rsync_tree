use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A prefix-based data structure with type-level metadata per path.
///
/// Terminal nodes (explicitly inserted paths) carry `Some(item)` while
/// intermediate routing nodes have `None`.
#[derive(Debug)]
pub struct PathTrie<T> {
    /// Map from path component to child trie
    children: HashMap<String, PathTrie<T>>,
    /// Payload stored at this exact path (None for intermediate routing nodes)
    item: Option<T>,
}

impl<T> PathTrie<T> {
    pub fn new() -> Self {
        Self {
            children: HashMap::new(),
            item: None,
        }
    }

    /// Insert `item` at the terminal node identified by `path`.
    /// Intermediate path components are created as routing nodes with `None`.
    pub fn insert(&mut self, path: &Path, item: T) {
        let mut current = self;
        for component in path.components() {
            if let Some(component_str) = component.as_os_str().to_str() {
                current = current
                    .children
                    .entry(component_str.to_string())
                    .or_insert_with(PathTrie::new);
            }
        }
        current.item = Some(item);
    }

    /// Return a reference to the item stored at the exact path, or `None`.
    pub fn get(&self, path: &Path) -> Option<&T> {
        let mut current = self;
        for component in path.components() {
            if let Some(component_str) = component.as_os_str().to_str() {
                if let Some(child) = current.children.get(component_str) {
                    current = child;
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        current.item.as_ref()
    }

    /// Returns `true` when a value has been inserted for this exact path.
    #[allow(dead_code)]
    pub fn contains(&self, path: &Path) -> bool {
        self.get(path).is_some()
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
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .to_string()
}
