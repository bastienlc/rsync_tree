use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::formatting::{
    compute_child_prefix, connector_glyph, format_collapsed_summary, format_debug_info,
    format_size_info, style_node_name, write_node_line,
};

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
        let json = serde_json::to_string_pretty(self)
            .map_err(io::Error::other)?;
        fs::write(path, json)
    }

    /// Deserialize the tree from a JSON file.
    pub fn load_from_file(path: &Path) -> io::Result<Self> {
        let json = fs::read_to_string(path)?;
        serde_json::from_str(&json).map_err(io::Error::other)
    }

    /// Render the tree as ASCII art to any writer, with color, collapsing, optional debug info (status), and optional size info.
    pub fn render_ascii<W: std::io::Write>(
        &self,
        w: &mut W,
        color: bool,
        collapse: bool,
        debug: bool,
        show_sizes: bool,
    ) -> std::io::Result<()> {
        self.render_ascii_core(w, "", true, true, color, collapse, debug, show_sizes, None)
    }

    #[allow(clippy::too_many_arguments)]
    fn render_ascii_core<W: std::io::Write>(
        &self,
        w: &mut W,
        prefix: &str,
        is_last: bool,
        is_first: bool,
        color: bool,
        collapse: bool,
        debug: bool,
        show_sizes: bool,
        parent_size: Option<u64>,
    ) -> std::io::Result<()> {
        let connector = connector_glyph(is_first, is_last);
        let styled_name = style_node_name(&self.name, self.status, color);
        let debug_info = if debug {
            format!(" {}", format_debug_info(self.status))
        } else {
            String::new()
        };
        let size_info = if show_sizes {
            format_size_info(self.size, parent_size, color)
        } else {
            String::new()
        };

        write_node_line(
            w,
            "",
            prefix,
            connector,
            &styled_name,
            &debug_info,
            &size_info,
            color,
        )?;

        // ---- collapse ----
        let should_collapse = !self.children.is_empty()
            && (self.status == NodeStatus::DirectoryExcluded
                || (collapse
                    && (self.status == NodeStatus::DirectoryIncluded
                        || self.status == NodeStatus::DirectoryStandalone)));

        if should_collapse {
            let summary = format_collapsed_summary(self.children.len(), color);
            let indent = compute_child_prefix(prefix, is_first, is_last, color);
            write_node_line(w, "", &indent, "└── ", &summary, "", "", color)?;
            return Ok(());
        }

        // ---- children ----
        let new_prefix = compute_child_prefix(prefix, is_first, is_last, color);

        let mut iter = self.children.values().peekable();
        while let Some(child) = iter.next() {
            let is_last_child = iter.peek().is_none();
            child.render_ascii_core(
                w,
                &new_prefix,
                is_last_child,
                false,
                color,
                collapse,
                debug,
                show_sizes,
                self.size,
            )?;
        }
        Ok(())
    }
}
