use log::{debug, error, info};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

/// Execute rsync command and capture output
pub fn execute_rsync_with_output(
    args: Vec<String>,
    save_output: Option<&PathBuf>,
) -> Result<String, Box<dyn std::error::Error>> {
    let program = &args[0];
    let rsync_args = &args[1..];

    info!("Executing: {} {}", program, rsync_args.join(" "));
    let start_time = Instant::now();

    let mut command = Command::new(program);
    command.args(rsync_args);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut child = command.spawn()?;

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    let (tx, rx) = mpsc::channel();
    let tx_stderr = tx.clone();

    // Thread to read stdout
    let tx_stdout = tx.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if tx_stdout.send(("stdout", line)).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx_stdout.send(("error", format!("Error reading stdout: {}", e)));
                    break;
                }
            }
        }
    });

    // Thread to read stderr
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if tx_stderr.send(("stderr", line)).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx_stderr.send(("error", format!("Error reading stderr: {}", e)));
                    break;
                }
            }
        }
    });

    let mut output_lines = Vec::new();
    let mut error_lines = Vec::new();

    // Read output from both streams
    drop(tx); // Close the sender so the loop can terminate
    while let Ok((stream, line)) = rx.recv() {
        match stream {
            "stdout" => {
                debug!("rsync stdout: {}", line);
                output_lines.push(line);
            }
            "stderr" => {
                debug!("rsync stderr: {}", line);
                error_lines.push(line);
            }
            "error" => {
                error!("{}", line);
            }
            _ => {}
        }
    }

    let exit_status = child.wait()?;
    let duration = start_time.elapsed();

    info!(
        "Rsync completed in {:.2}s with exit code: {}",
        duration.as_secs_f64(),
        exit_status.code().unwrap_or(-1)
    );

    let full_output = output_lines.join("\n");
    let full_errors = error_lines.join("\n");

    // Save output to file if requested
    if let Some(output_path) = save_output {
        std::fs::write(output_path, &full_output)?;
        info!("Saved rsync output to: {}", output_path.display());
    }

    if !exit_status.success() {
        info!("Rsync exited with non-zero status: {}", exit_status);
        if !full_errors.is_empty() {
            info!("Rsync stderr:\n{}", full_errors);
        }
        // Don't treat non-zero exit as fatal - rsync often exits with status 24
        // for "Partial transfer due to vanished source files" which is normal
    }

    if !full_errors.is_empty() {
        debug!("Rsync stderr:\n{}", full_errors);
    }

    debug!("Captured {} lines of output", output_lines.len());

    Ok(full_output)
}
