use anstyle::{AnsiColor, Color, Style};

use crate::tree::NodeStatus;

/// Format file size in human-readable format
pub fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: f64 = 1024.0;

    if size == 0 {
        return "0 B".to_string();
    }

    let mut size_f = size as f64;
    let mut unit_index = 0;

    while size_f >= THRESHOLD && unit_index < UNITS.len() - 1 {
        size_f /= THRESHOLD;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size_f, UNITS[unit_index])
    }
}

/// Get color for percentage based on a white-orange scale.
/// Higher percentages get more intense orange/red colors.
pub fn get_percentage_color(percentage: f64) -> Color {
    if percentage >= 80.0 {
        Color::Ansi(AnsiColor::BrightRed)
    } else if percentage >= 60.0 {
        Color::Ansi(AnsiColor::Red)
    } else if percentage >= 40.0 {
        Color::Ansi(AnsiColor::Yellow)
    } else if percentage >= 20.0 {
        Color::Ansi(AnsiColor::BrightYellow)
    } else if percentage >= 5.0 {
        Color::Ansi(AnsiColor::White)
    } else {
        Color::Ansi(AnsiColor::BrightBlack)
    }
}

/// Format debug information for node status.
pub fn format_debug_info(status: NodeStatus) -> String {
    match status {
        NodeStatus::FileIncluded => "[FI]",
        NodeStatus::FileExcluded => "[FE]",
        NodeStatus::DirectoryExcluded => "[DE]",
        NodeStatus::DirectoryIncluded => "[DI]",
        NodeStatus::DirectoryMixed => "[DM]",
    }
    .to_string()
}

/// Format size information with optional percentage.
pub fn format_size_info(size: Option<u64>, parent_size: Option<u64>, use_color: bool) -> String {
    match size {
        Some(size) => {
            let percentage = if let Some(parent_size) = parent_size {
                if parent_size > 0 {
                    let pct = (size as f64 / parent_size as f64) * 100.0;
                    let pct_text = format!(" {:.1}%", pct);

                    if use_color {
                        let pct_color = get_percentage_color(pct);
                        let pct_style = Style::new().fg_color(Some(pct_color));
                        format!(
                            "{}{}{}",
                            pct_style.render(),
                            pct_text,
                            pct_style.render_reset()
                        )
                    } else {
                        pct_text
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            format!(" ({}{})", format_size(size), percentage)
        }
        None => String::new(),
    }
}

/// Build an `anstyle::Style` for the given "colour category".
///
/// - `is_green = true`  → Green (included)
/// - `is_green = false` → Red   (excluded)
/// - `is_dir = true`    → bold variant
/// - `no_colour = true` → always returns `Style::new()` (plain)
pub fn status_style(is_green: bool, is_dir: bool, no_colour: bool) -> Style {
    if no_colour {
        return if is_dir {
            Style::new().bold()
        } else {
            Style::new()
        };
    }
    let base = if is_green {
        Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)))
    } else {
        Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red)))
    };
    if is_dir { base.bold() } else { base }
}

/// Style for "Mixed" directories (bold only, no colour).
pub fn mixed_style() -> Style {
    Style::new().bold()
}

/// Style for "Missing" tree entries (dim gray).
pub fn missing_style() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlack)))
}

/// DIM style for collapsed summaries.
pub fn dim_style() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlack)))
}

/// Apply `style` to `text`, wrapping with ANSI reset.
pub fn apply_style(text: &str, style: Style) -> String {
    format!("{}{}{}", style.render(), text, style.render_reset())
}

/// Style a node name based on `NodeStatus`.
pub fn style_node_name(name: &str, status: NodeStatus, use_color: bool) -> String {
    let style = match status {
        NodeStatus::FileIncluded => status_style(true, false, !use_color),
        NodeStatus::DirectoryIncluded => {
            status_style(true, true, !use_color)
        }
        NodeStatus::FileExcluded => status_style(false, false, !use_color),
        NodeStatus::DirectoryExcluded => status_style(false, true, !use_color),
        NodeStatus::DirectoryMixed => mixed_style(),
    };
    apply_style(name, style)
}

/// Format the ASCII connector (`├──`, `└──`, `│`, `    `) with optional color.
pub fn format_connector(connector: &str, use_color: bool) -> String {
    if use_color && !connector.is_empty() {
        let white_style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::White)));
        apply_style(connector, white_style)
    } else {
        connector.to_string()
    }
}

/// The "connector" glyph for a tree node.
pub fn connector_glyph(is_first: bool, is_last: bool) -> &'static str {
    if is_first {
        ""
    } else if is_last {
        "└── "
    } else {
        "├── "
    }
}

/// Compute the prefix for children given the current node's position.
pub fn compute_child_prefix(
    current_prefix: &str,
    is_first: bool,
    is_last: bool,
    color: bool,
) -> String {
    if is_first {
        current_prefix.to_string()
    } else if is_last {
        format!("{}    ", current_prefix)
    } else if color {
        format!("{}{}", current_prefix, format_connector("│   ", color))
    } else {
        format!("{}│   ", current_prefix)
    }
}

/// Format a collapsed summary line "(N items)" with optional dim color.
pub fn format_collapsed_summary(count: usize, color: bool) -> String {
    let text = format!("({} items)", count);
    if color {
        apply_style(&text, dim_style())
    } else {
        text
    }
}

/// Write a fully-assembled line for one tree node.
#[allow(clippy::too_many_arguments)]
pub fn write_node_line<W: std::io::Write>(
    w: &mut W,
    left_margin: &str,
    prefix: &str,
    connector: &str,
    styled_name: &str,
    debug_info: &str,
    size_info: &str,
    color: bool,
) -> std::io::Result<()> {
    let styled_connector = format_connector(connector, color);
    writeln!(
        w,
        "{}{}{}{}{}{}",
        left_margin, prefix, styled_connector, styled_name, debug_info, size_info
    )
}
