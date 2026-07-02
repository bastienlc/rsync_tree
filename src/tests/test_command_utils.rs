use crate::command_utils::{parse_command, remove_flags, validate_flags};

#[test]
fn test_parse_command() {
    let result = parse_command("rsync -av src/ dest/");
    assert_eq!(result, vec!["rsync", "-av", "src/", "dest/"]);
}

#[test]
fn test_validate_flags() {
    let args = vec!["rsync".to_string(), "-av".to_string()];
    let result = validate_flags(&args);
    assert!(result.is_err(), "expected error when flags are missing");
    assert!(
        result.unwrap_err().contains("--dry-run"),
        "error should mention --dry-run"
    );
}

#[test]
fn test_validate_flags_already_present() {
    let args = vec![
        "rsync".to_string(),
        "-av".to_string(),
        "--dry-run".to_string(),
        "--itemize-changes".to_string(),
    ];
    let result = validate_flags(&args).unwrap();
    // Should have exactly the same flags, no duplicates
    assert_eq!(result.len(), args.len());
    assert!(result.contains(&"--dry-run".to_string()));
    assert!(result.contains(&"--itemize-changes".to_string()));
}

#[test]
fn test_remove_flags() {
    let args = vec![
        "rsync".to_string(),
        "-av".to_string(),
        "--out-format=%i %n%L".to_string(),
        "--delete".to_string(),
    ];
    let result = remove_flags(args, &["--out-format"]);
    assert!(!result.iter().any(|arg| arg.starts_with("--out-format")));
    assert!(result.contains(&"--delete".to_string()));
    assert_eq!(result.len(), 3);
}

#[test]
fn test_validate_flags_removes_out_format() {
    let args = vec![
        "rsync".to_string(),
        "-av".to_string(),
        "--dry-run".to_string(),
        "--itemize-changes".to_string(),
        "--out-format=%i %n%L".to_string(),
    ];
    let result = validate_flags(&args).unwrap();
    assert!(!result.iter().any(|arg| arg.starts_with("--out-format")));
    assert!(result.contains(&"--dry-run".to_string()));
    assert!(result.contains(&"--itemize-changes".to_string()));
}
