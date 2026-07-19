//! Follow the systemd journal via `journalctl -f`.

use std::collections::VecDeque;

use cosmic::iced::futures::{SinkExt, Stream};
use cosmic::iced::stream;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Debug,
}

#[derive(Debug, Clone)]
pub struct LogLine {
    pub text: String,
    pub severity: Severity,
}

/// Infer a rough severity from a short-iso journal line for tinting.
pub fn classify(line: &str) -> Severity {
    let lower = line.to_ascii_lowercase();
    if lower.contains(" error")
        || lower.contains(":error")
        || lower.contains(" err:")
        || lower.contains("critical")
        || lower.contains("fatal")
        || lower.contains(" failed")
        || lower.contains("failure")
    {
        Severity::Error
    } else if lower.contains(" warn")
        || lower.contains(":warn")
        || lower.contains("warning")
    {
        Severity::Warning
    } else if lower.contains(" debug") || lower.contains(":debug") || lower.contains(" trace")
    {
        Severity::Debug
    } else {
        Severity::Info
    }
}

pub fn push_line(buffer: &mut VecDeque<LogLine>, line: LogLine, max_lines: usize) {
    buffer.push_back(line);
    while buffer.len() > max_lines {
        buffer.pop_front();
    }
}

/// Build the `journalctl` argv: defaults plus optional config extras.
pub fn journalctl_args(config: &Config) -> Vec<String> {
    let mut args = vec![
        "-f".into(),
        "-o".into(),
        "short-iso".into(),
        "--no-pager".into(),
        "-n".into(),
        config.max_lines.to_string(),
    ];
    args.extend(config.journal_args.iter().cloned());
    args
}

/// Endless subscription stream of journal lines (and status messages).
pub fn follow(config: Config) -> impl Stream<Item = LogLine> {
    stream::channel(64, async move |mut output| {
        loop {
            let args = journalctl_args(&config);
            let mut child = match Command::new("journalctl")
                .args(&args)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true)
                .spawn()
            {
                Ok(child) => child,
                Err(err) => {
                    let _ = output
                        .send(LogLine {
                            text: format!("failed to start journalctl: {err}"),
                            severity: Severity::Error,
                        })
                        .await;
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            let Some(stdout) = child.stdout.take() else {
                let _ = output
                    .send(LogLine {
                        text: "journalctl produced no stdout".into(),
                        severity: Severity::Error,
                    })
                    .await;
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            };

            let mut lines = BufReader::new(stdout).lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(text)) => {
                        if text.is_empty() {
                            continue;
                        }
                        let severity = classify(&text);
                        if output
                            .send(LogLine { text, severity })
                            .await
                            .is_err()
                        {
                            let _ = child.kill().await;
                            return;
                        }
                    }
                    Ok(None) => break,
                    Err(err) => {
                        let _ = output
                            .send(LogLine {
                                text: format!("journal read error: {err}"),
                                severity: Severity::Error,
                            })
                            .await;
                        break;
                    }
                }
            }

            let _ = child.kill().await;
            let _ = output
                .send(LogLine {
                    text: "journalctl exited; restarting…".into(),
                    severity: Severity::Warning,
                })
                .await;
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    })
}
