/// Display help information and legends for the tree output.
///
/// When `num_trees > 0` we are in compare mode and the legend is
/// enriched with Missing / Bold / label explanations.
pub fn display_legends(debug: bool, show_sizes: bool, color: bool, num_trees: usize) {
    // ------- debug legend -------
    if debug && num_trees == 0 {
        println!("\nLegend:");
        println!("  [FI] - File Included");
        println!("  [FE] - File Excluded");
        println!("  [DI] - Directory Included (all children included)");
        println!("  [DE] - Directory Excluded");
        println!("  [DS] - Directory Standalone (included but no children)");
        println!("  [DM] - Directory Mixed (some children included)");
    }

    if debug && num_trees > 0 {
        println!("\nDebug legend:");
        println!("  [I:N E:N M:N -:N] = counts of Included, Excluded, Mixed, Missing");
    }

    // ------- size legend (single-tree only) -------
    if show_sizes && num_trees == 0 {
        println!("\nSize Information:");
        println!("  Sizes shown in parentheses for included files and directories");
        println!("  Directory sizes are computed as sum of included children");
        println!("  Percentages show relative size compared to parent directory");
    }

    // ------- colour legend -------
    let has_colour = color && num_trees == 0;
    let has_colour_compare = color && num_trees > 0;

    if has_colour || has_colour_compare {
        println!("\x1b[0m\nColor Legend:");
        println!("  \x1b[32mGreen\x1b[0m   – Included files/directories");
        println!("  \x1b[31mRed\x1b[0m     – Excluded files/directories");
        println!("  \x1b[1mWhite\x1b[0m   – Mixed directories");
        if num_trees > 0 {
            println!("  \x1b[90mGray\x1b[0m    – Missing (file/dir does not exist in this tree)");
            println!("  \x1b[1mBold\x1b[0m     – Directory");
        }
    }

    // ------- compare-specific explanation -------
    if num_trees > 0 {
        println!("\n  Labels (JSON file names) are shown at the left when tree statuses differ.");
        println!("  When all trees agree on a node, no labels are shown.");
        println!(
            "\nMode: Showing only the subtree where some trees differ. \
             Directories with no Mixed status can be collapsed because \
             the difference is at the directory level itself."
        );
    }
}
