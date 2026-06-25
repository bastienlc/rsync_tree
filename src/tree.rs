use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
pub enum NodeStatus {
    FileExcluded,
    FileIncluded,
    DirectoryExcluded,
    DirectoryStandalone,
    DirectoryMixed,
    DirectoryIncluded,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Tree {
    pub name: String,
    pub path: PathBuf,
    pub children: BTreeMap<String, Tree>,
    pub status: NodeStatus,
    /// Size in bytes. None if size collection is disabled or failed
    pub size: Option<u64>,
}

impl Tree {
    /// Create a new tree node.
    pub fn new(name: String, path: PathBuf, status: NodeStatus) -> Self {
        Self {
            name,
            path,
            children: BTreeMap::new(),
            status,
            size: None,
        }
    }

    /// Add a child node and return a mutable reference to it.
    pub fn add_child(&mut self, child: Tree) -> &mut Tree {
        let name = child.name.clone();
        self.children.insert(name.clone(), child);
        self.children.get_mut(&name).unwrap()
    }

    /// Compute folder sizes recursively based on included children.
    /// Only computes sizes for directories that are included or mixed.
    pub fn compute_folder_sizes(&mut self) {
        // First, recursively compute sizes for all children
        for child in self.children.values_mut() {
            child.compute_folder_sizes();
        }

        // If this is a directory and we don't have a size yet, compute it
        if self.is_directory() && self.size.is_none() {
            match self.status {
                NodeStatus::DirectoryIncluded
                | NodeStatus::DirectoryMixed
                | NodeStatus::DirectoryStandalone => {
                    // Sum up sizes of all included children
                    let total_size: u64 = self
                        .children
                        .values()
                        .filter_map(|child| {
                            match child.status {
                                NodeStatus::FileIncluded
                                | NodeStatus::DirectoryIncluded
                                | NodeStatus::DirectoryMixed
                                | NodeStatus::DirectoryStandalone => child.size,
                                _ => None, // Don't count excluded items
                            }
                        })
                        .sum();

                    self.size = Some(total_size);
                }
                _ => {
                    // For excluded directories, don't compute size
                }
            }
        }
    }

    /// Check if this node represents a directory
    pub fn is_directory(&self) -> bool {
        matches!(
            self.status,
            NodeStatus::DirectoryExcluded
                | NodeStatus::DirectoryIncluded
                | NodeStatus::DirectoryMixed
                | NodeStatus::DirectoryStandalone
        )
    }

    /// Serialize the tree to a JSON file.
    pub fn save_to_file(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(path, json)
    }

    /// Deserialize the tree from a JSON file.
    pub fn load_from_file(path: &Path) -> io::Result<Self> {
        let json = fs::read_to_string(path)?;
        serde_json::from_str(&json).map_err(io::Error::other)
    }

    }
