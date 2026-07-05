use std::io::{BufRead, BufReader, Write, stderr};

use log::warn;

use crate::rsync_parser::parse_line;
use crate::rsync_types::ParseResult;

/// Read stdin line-by-line, parse each line as rsync itemize-changes output.
///
/// Displays a live progress counter on stderr that updates in-place.
/// Parse warnings are emitted via `log::warn!` (styled by env_logger),
/// with the progress line cleared before each warning and restored after.
pub fn read_and_parse_stdin() -> Result<Vec<ParseResult>, Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let reader = BufReader::new(stdin);
    let mut results = Vec::new();
    let mut line_count = 0u64;
    let mut parse_errors = 0u64;
    let mut err = stderr();

    for line in reader.lines() {
        let line = line?;
        line_count += 1;

        let parsed = parse_line(&line);
        match &parsed {
            ParseResult::Item(_) => {
                results.push(parsed);
            }
            ParseResult::Empty => {}
            ParseResult::InvalidFormat(_) => {
                parse_errors += 1;
                // Clear progress line, emit warning, restore progress
                write!(err, "\r\x1b[K")?;
                err.flush()?;
                warn!("could not parse line {}: {}", line_count, line);
                write!(
                    err,
                    "\rProcessed {} lines ({} warnings)...",
                    line_count, parse_errors
                )?;
                err.flush()?;
            }
        }

        // Update live progress every 500 lines
        if line_count % 500 == 0 {
            write!(
                err,
                "\rProcessed {} lines ({} warnings)...",
                line_count, parse_errors
            )?;
            err.flush()?;
        }
    }

    // Final summary
    if line_count > 0 {
        writeln!(
            err,
            "\rDone: processed {} lines, {} parse warnings.",
            line_count, parse_errors
        )?;
    }

    Ok(results)
}
