use crate::display::formatting::{
    compute_child_prefix, connector_glyph, format_collapsed_summary, format_debug_info,
    format_size_info, style_node_name, write_node_line,
};
use crate::tree::{NodeStatus, Tree};

/// Render a single-mode tree as ASCII art to any writer.
pub fn render_tree<W: std::io::Write>(
    tree: &Tree,
    w: &mut W,
    color: bool,
    collapse: bool,
    debug: bool,
    show_sizes: bool,
) -> std::io::Result<()> {
    render_tree_core(tree, w, "", true, true, color, collapse, debug, show_sizes, None)
}

#[allow(clippy::too_many_arguments)]
fn render_tree_core<W: std::io::Write>(
    node: &Tree,
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
    let styled_name = style_node_name(&node.name, node.status, color);
    let debug_info = if debug {
        format!(" {}", format_debug_info(node.status))
    } else {
        String::new()
    };
    let size_info = if show_sizes {
        format_size_info(node.size, parent_size, color)
    } else {
        String::new()
    };

    write_node_line(w, "", prefix, connector, &styled_name, &debug_info, &size_info, color)?;

    let should_collapse = !node.children.is_empty()
        && (node.status == NodeStatus::DirectoryExcluded
            || (collapse
                && (node.status == NodeStatus::DirectoryIncluded
                    || node.status == NodeStatus::DirectoryStandalone)));

    if should_collapse {
        let summary = format_collapsed_summary(node.children.len(), color);
        let indent = compute_child_prefix(prefix, is_first, is_last, color);
        write_node_line(w, "", &indent, "└── ", &summary, "", "", color)?;
        return Ok(());
    }

    let new_prefix = compute_child_prefix(prefix, is_first, is_last, color);
    let mut iter = node.children.values().peekable();
    while let Some(child) = iter.next() {
        let is_last_child = iter.peek().is_none();
        render_tree_core(
            child,
            w,
            &new_prefix,
            is_last_child,
            false,
            color,
            collapse,
            debug,
            show_sizes,
            node.size,
        )?;
    }
    Ok(())
}
