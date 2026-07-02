use log::{debug, info, warn};
use std::path::PathBuf;

/// Parse a space-separated command string into individual arguments
pub fn parse_command(command: &str) -> Vec<String> {
    command.split_whitespace().map(|s| s.to_string()).collect()
}

/// Remove specified flags from command arguments
pub fn remove_flags(mut args: Vec<String>, flags_to_remove: &[&str]) -> Vec<String> {
    let original_len = args.len();
    args.retain(|arg| {
        !flags_to_remove.iter().any(|&flag| {
            arg == flag || (flag.starts_with("--") && arg.starts_with(&format!("{}=", flag)))
        })
    });

    let removed_count = original_len - args.len();
    if removed_count > 0 {
        info!("Removed {} flag(s) from rsync command", removed_count);
    }

    args
}

/// Validate that required flags are present in the rsync command, and remove
/// conflicting flags. Returns an error if `--dry-run` (or `-n`) or
/// `--itemize-changes` (or `-i`) are missing.
pub fn validate_flags(args: &[String]) -> Result<Vec<String>, String> {
    let mut cleaned = args.to_vec();

    // Remove --out-format flag if present (conflicts with --itemize-changes)
    cleaned = remove_flags(cleaned, &["--out-format"]);
    // Remove --verbose flag if present (we want only itemized output)
    cleaned = remove_flags(cleaned, &["-v", "--verbose"]);

    // Check --dry-run / -n is present
    let has_dry_run = cleaned.iter().any(|arg| arg == "--dry-run" || arg == "-n");
    if !has_dry_run {
        return Err("Missing required flag: --dry-run (or -n). \
             This flag is required to prevent accidental file transfers."
            .into());
    }
    info!("--dry-run flag is present");

    // Check --itemize-changes / -i is present
    let has_itemize = cleaned
        .iter()
        .any(|arg| arg == "--itemize-changes" || arg == "-i");
    if !has_itemize {
        return Err("Missing required flag: --itemize-changes (or -i). \
             This flag is required to produce the itemized output that rsync_tree parses."
            .into());
    }
    info!("--itemize-changes flag is present");

    Ok(cleaned)
}

/// Determine the base path from rsync arguments or use provided path
pub fn determine_base_path(args: &[String], provided_base: Option<&PathBuf>) -> PathBuf {
    if let Some(base) = provided_base {
        return base.clone();
    }

    // Try to extract source path from rsync arguments
    // Look for the first argument that looks like a source path (not a flag)
    for (i, arg) in args.iter().enumerate() {
        if i == 0 {
            continue; // Skip the program name
        }
        if arg.starts_with('-') {
            continue; // Skip flags
        }

        // This might be a source path
        let path = PathBuf::from(arg);
        if path.exists() && path.is_dir() {
            debug!("Detected source directory: {}", path.display());
            return path;
        }
    }

    // Fallback to current directory
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    warn!(
        "Could not determine source directory from rsync command, using current directory: {}",
        current_dir.display()
    );
    current_dir
}
