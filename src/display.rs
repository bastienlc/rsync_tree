/// Display help information and legends for the tree output
pub fn display_legends(debug: bool, show_sizes: bool, color: bool) {
    if debug {
        println!("\nLegend:");
        println!("  [FI] - File Included");
        println!("  [FE] - File Excluded");
        println!("  [DI] - Directory Included (all children included)");
        println!("  [DE] - Directory Excluded");
        println!("  [DS] - Directory Standalone (included but no children)");
        println!("  [DM] - Directory Mixed (some children included)");
    }

    if show_sizes {
        println!("\nSize Information:");
        println!("  Sizes shown in parentheses for included files and directories");
        println!("  Directory sizes are computed as sum of included children");
        println!("  Percentages show relative size compared to parent directory");
    }

    if color {
        println!("\x1b[0m\nColor Legend:");
        println!("  \x1b[32mGreen\x1b[0m - Included files/directories");
        println!("  \x1b[31mRed\x1b[0m   - Excluded files/directories");
        println!("  White - Mixed directories");
    }
}
