//! Upload progress using [`indicatif::MultiProgress`].

use anyhow::{Result, bail};
use futures::stream::{self, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use minutes_core::Entry;

use crate::backends::Backend;

/// Maximum number of concurrent uploads.
const MAX_CONCURRENT_UPLOADS: usize = 5;

/// Pushes entries concurrently (up to [`MAX_CONCURRENT_UPLOADS`]) while showing
/// one full-width progress bar per entry.
///
/// Finished bars stay on screen (success: green `uploaded!`, failure: yellow
/// `failed!`) above the final summary line.
///
/// # Errors
///
/// Returns an error when one or more uploads fail. Successful uploads are kept;
/// failures are listed in the error message.
pub async fn upload_with_progress(backend: &Backend, entries: &[Entry]) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }

    let multi = MultiProgress::new();
    let pending_style = bar_style("{msg} [{wide_bar:.cyan/blue}] {elapsed_precise} {prefix}");
    let done_style = bar_style("{msg} [{wide_bar:.green/green}] {elapsed_precise} {prefix:.green}");
    let fail_style =
        bar_style("{msg} [{wide_bar:.yellow/yellow}] {elapsed_precise} {prefix:.yellow}");

    let mut bars = Vec::with_capacity(entries.len());
    for entry in entries {
        let bar = multi.add(ProgressBar::new(1));
        bar.set_style(pending_style.clone());
        bar.set_message(entry_label(entry));
        bar.set_prefix(String::new());
        bars.push(bar);
    }

    let failures: Vec<String> = stream::iter(entries.iter().zip(bars.iter()))
        .map(|(entry, bar)| {
            let done_style = done_style.clone();
            let fail_style = fail_style.clone();
            async move {
                bar.enable_steady_tick(std::time::Duration::from_millis(80));
                let result = backend.push_entry(entry).await;
                bar.disable_steady_tick();
                match result {
                    Ok(()) => {
                        bar.set_style(done_style);
                        bar.set_prefix("uploaded!");
                        bar.inc(1);
                        bar.finish();
                        None
                    }
                    Err(err) => {
                        let label = entry_label(entry);
                        bar.set_style(fail_style);
                        bar.set_prefix("failed!");
                        bar.abandon();
                        Some(format!("{label}: {err:#}"))
                    }
                }
            }
        })
        .buffer_unordered(MAX_CONCURRENT_UPLOADS)
        .filter_map(async |failure| failure)
        .collect()
        .await;

    if failures.is_empty() {
        println!(
            "Successfully uploaded {} entr{}.",
            entries.len(),
            plural(entries.len())
        );
        Ok(())
    } else {
        let succeeded = entries.len() - failures.len();
        eprintln!(
            "Uploaded {succeeded} entr{}, {} failed:",
            plural(succeeded),
            failures.len()
        );
        for failure in &failures {
            eprintln!("  - {failure}");
        }
        bail!("{} of {} uploads failed", failures.len(), entries.len());
    }
}

fn bar_style(template: &str) -> ProgressStyle {
    ProgressStyle::with_template(template)
        .unwrap_or_else(|_| ProgressStyle::default_bar())
        .progress_chars("=>-")
}

fn entry_label(entry: &Entry) -> String {
    entry
        .summary
        .clone()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| entry.task.to_string())
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "y" } else { "ies" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minutes_core::{ClientId, ProjectId, TaskId, WorkInterval};
    use std::time::{Duration, UNIX_EPOCH};

    fn entry(task: &str, summary: Option<&str>) -> Entry {
        let mut entry = Entry::new(
            ClientId::from("c"),
            ProjectId::from("p"),
            TaskId::from(task),
            WorkInterval::new(UNIX_EPOCH, Duration::from_secs(60), true),
        );
        if let Some(summary) = summary {
            entry = entry.with_summary(summary);
        }
        entry
    }

    #[test]
    fn entry_label_prefers_non_empty_summary() {
        assert_eq!(entry_label(&entry("TASK-1", Some("Ship fix"))), "Ship fix");
    }

    #[test]
    fn entry_label_falls_back_to_task_when_summary_missing() {
        assert_eq!(entry_label(&entry("TASK-1", None)), "TASK-1");
    }

    #[test]
    fn entry_label_falls_back_to_task_when_summary_empty() {
        assert_eq!(entry_label(&entry("TASK-1", Some(""))), "TASK-1");
    }

    #[test]
    fn max_concurrent_uploads_is_five() {
        assert_eq!(MAX_CONCURRENT_UPLOADS, 5);
    }
}
