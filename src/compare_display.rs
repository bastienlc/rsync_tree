use anstyle::Style;

use crate::compare::{ComparedNode, PerTreeStatus};
use crate::formatting::{
    apply_style, compute_child_prefix, connector_glyph, format_collapsed_summary, missing_style,
    mixed_style, status_style, write_node_line,
};

// ---------------------------------------------------------------------------
// Per-tree-status styling helpers
// ---------------------------------------------------------------------------

/// Build a `Style` for a `PerTreeStatus` / directory combination.
fn per_tree_style(status: PerTreeStatus, is_dir: bool, no_color: bool) -> Style {
    match status {
        PerTreeStatus::Included => status_style(true, is_dir, no_color),
        PerTreeStatus::Excluded => status_style(false, is_dir, no_color),
        PerTreeStatus::Mixed => mixed_style(),
        PerTreeStatus::Missing => missing_style(),
    }
}

/// Style a single tree label (filename) based on its per-tree status.
fn style_label(label: &str, status: PerTreeStatus, is_directory: bool, use_color: bool) -> String {
    let style = per_tree_style(status, is_directory, !use_color);
    apply_style(label, style)
}

/// Style the node name when **all** trees agree on the status.
fn style_node_name_common(
    name: &str,
    status: PerTreeStatus,
    is_directory: bool,
    use_color: bool,
) -> String {
    style_label(name, status, is_directory, use_color)
}

/// Return `true` when every entry in `statuses` is the same variant.
fn all_statuses_equal(statuses: &[PerTreeStatus]) -> bool {
    statuses.len() <= 1 || statuses.windows(2).all(|w| w[0] == w[1])
}

// ---------------------------------------------------------------------------
// ComparedNode rendering (uses shared helpers from formatting)
// ---------------------------------------------------------------------------

impl ComparedNode {
    /// Render the compared tree as ASCII art to any writer.
    pub fn render_ascii<W: std::io::Write>(
        &self,
        w: &mut W,
        color: bool,
        debug: bool,
        tree_labels: &[String],
    ) -> std::io::Result<()> {
        let label_area_width = if tree_labels.is_empty() {
            0
        } else {
            tree_labels.join(" ").len() + 1
        };
        self.render_ascii_core(
            w,
            "",
            true,
            true,
            color,
            debug,
            tree_labels,
            label_area_width,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_ascii_core<W: std::io::Write>(
        &self,
        w: &mut W,
        prefix: &str,
        is_last: bool,
        is_first: bool,
        color: bool,
        debug: bool,
        tree_labels: &[String],
        label_area_width: usize,
    ) -> std::io::Result<()> {
        let connector = connector_glyph(is_first, is_last);

        // Debug info: counts per status
        let debug_info = if debug {
            let included = self
                .per_tree_status
                .iter()
                .filter(|s| **s == PerTreeStatus::Included)
                .count();
            let excluded = self
                .per_tree_status
                .iter()
                .filter(|s| **s == PerTreeStatus::Excluded)
                .count();
            let mixed = self
                .per_tree_status
                .iter()
                .filter(|s| **s == PerTreeStatus::Mixed)
                .count();
            let missing = self
                .per_tree_status
                .iter()
                .filter(|s| **s == PerTreeStatus::Missing)
                .count();
            format!(" [I:{} E:{} M:{} -:{}]", included, excluded, mixed, missing)
        } else {
            String::new()
        };

        let label_padding = " ".repeat(label_area_width);
        let show_labels = !all_statuses_equal(&self.per_tree_status);

        let left_part = if show_labels {
            let labels: Vec<String> = tree_labels
                .iter()
                .zip(self.per_tree_status.iter())
                .map(|(label, status)| style_label(label, *status, self.is_directory, color))
                .collect();
            format!("{} ", labels.join(" "))
        } else {
            label_padding.clone()
        };

        // Build the styled name
        let name_part = if show_labels {
            self.name.clone()
        } else {
            let common_status = self
                .per_tree_status
                .first()
                .copied()
                .unwrap_or(PerTreeStatus::Missing);
            style_node_name_common(&self.name, common_status, self.is_directory, color)
        };

        // Write the node line (root gets special casing: no connector)
        if is_first {
            writeln!(w, "{}{}{}", left_part, name_part, debug_info)?;
        } else {
            write_node_line(
                w,
                &left_part,
                prefix,
                connector,
                &name_part,
                &debug_info,
                "",
                color,
            )?;
        }

        // ---- collapse ----
        if self.is_directory && self.can_collapse() && !self.children.is_empty() {
            let summary = format_collapsed_summary(self.children.len(), color);
            let indent = compute_child_prefix(prefix, is_first, is_last, color);
            write_node_line(w, &label_padding, &indent, "└── ", &summary, "", "", color)?;
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
                debug,
                tree_labels,
                label_area_width,
            )?;
        }

        Ok(())
    }
}
