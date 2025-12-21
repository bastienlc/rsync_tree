use anstyle::{AnsiColor, Color, Style};

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

/// Get color for percentage based on a white-orange scale
/// Higher percentages get more intense orange/red colors
pub fn get_percentage_color(percentage: f64) -> Color {
    if percentage >= 80.0 {
        Color::Ansi(AnsiColor::BrightRed) // 80-100%: Bright Red
    } else if percentage >= 60.0 {
        Color::Ansi(AnsiColor::Red) // 60-80%: Red
    } else if percentage >= 40.0 {
        Color::Ansi(AnsiColor::Yellow) // 40-60%: Yellow (orange-ish)
    } else if percentage >= 20.0 {
        Color::Ansi(AnsiColor::BrightYellow) // 20-40%: Bright Yellow
    } else if percentage >= 5.0 {
        Color::Ansi(AnsiColor::White) // 5-20%: White
    } else {
        Color::Ansi(AnsiColor::BrightBlack) // 0-5%: Dark gray
    }
}

/// Create styled text with colors based on node status
pub fn style_node_name(name: &str, status: NodeStatus, use_color: bool) -> String {
    if use_color {
        match status {
            NodeStatus::FileIncluded => {
                let style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));
                format!("{}{}{}", style.render(), name, style.render_reset())
            }
            NodeStatus::DirectoryIncluded | NodeStatus::DirectoryStandalone => {
                let style = Style::new()
                    .fg_color(Some(Color::Ansi(AnsiColor::Green)))
                    .bold();
                format!("{}{}{}", style.render(), name, style.render_reset())
            }
            NodeStatus::FileExcluded => {
                let style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red)));
                format!("{}{}{}", style.render(), name, style.render_reset())
            }
            NodeStatus::DirectoryExcluded => {
                let style = Style::new()
                    .fg_color(Some(Color::Ansi(AnsiColor::Red)))
                    .bold();
                format!("{}{}{}", style.render(), name, style.render_reset())
            }
            NodeStatus::DirectoryMixed => {
                let style = Style::new().bold();
                format!("{}{}{}", style.render(), name, style.render_reset())
            }
        }
    } else {
        match status {
            NodeStatus::DirectoryIncluded
            | NodeStatus::DirectoryStandalone
            | NodeStatus::DirectoryExcluded
            | NodeStatus::DirectoryMixed => {
                let style = Style::new().bold();
                format!("{}{}{}", style.render(), name, style.render_reset())
            }
            _ => name.to_string(),
        }
    }
}

/// Format debug information for node status
pub fn format_debug_info(status: NodeStatus) -> String {
    match status {
        NodeStatus::FileIncluded => "[FI]",
        NodeStatus::FileExcluded => "[FE]",
        NodeStatus::DirectoryExcluded => "[DE]",
        NodeStatus::DirectoryStandalone => "[DS]",
        NodeStatus::DirectoryIncluded => "[DI]",
        NodeStatus::DirectoryMixed => "[DM]",
    }
    .to_string()
}

/// Format size information with optional percentage
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

/// Format tree connectors with optional colors
pub fn format_connector(connector: &str, use_color: bool) -> String {
    if use_color && !connector.is_empty() {
        let white_style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::White)));
        format!(
            "{}{}{}",
            white_style.render(),
            connector,
            Style::new().render_reset()
        )
    } else {
        connector.to_string()
    }
}

use crate::tree::NodeStatus;
