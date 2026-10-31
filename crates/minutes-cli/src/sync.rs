//! End-to-end sync orchestration: fetch → transform → preview → upload.

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Local};
use dialoguer::Confirm;
use minutes_client::FetchOpts;
use minutes_core::{Entry, EntryFilter, TextMatcher};

use crate::backends::Backend;
use crate::config::Config;
use crate::progress::upload_with_progress;
use crate::table::render_overview;
use crate::transform::{force_billed_duration, merge_by_day, round_to_closest_minute};
/// Runs a full sync using the resolved `config`.
///
/// # Errors
///
/// Returns an error when validation, fetch, confirmation abort, or upload fails.
pub async fn run(config: Config) -> Result<()> {
    config.validate()?;

    let source_kind = config.source.expect("validated");
    let target_kind = config.target.expect("validated");
    let (start, end) = config.parse_window()?;

    let source = Backend::build(source_kind, &config)
        .with_context(|| format!("failed to configure source `{source_kind}`"))?;
    let target = Backend::build(target_kind, &config)
        .with_context(|| format!("failed to configure target `{target_kind}`"))?;

    let filter = build_filter(&config)?;
    let opts = FetchOpts::new(start, end);

    eprintln!(
        "Fetching from {source_kind} → {target_kind} ({})",
        format_window(start, end)
    );

    let fetched = source.fetch_entries(&opts).await?;
    let filtered = filter.apply(fetched);
    let mut entries = merge_by_day(filtered);

    if config.round_to_closest_minute {
        entries = round_to_closest_minute(entries);
    }
    if config.force_billed_duration {
        entries = force_billed_duration(entries);
    }

    let (complete, incomplete): (Vec<Entry>, Vec<Entry>) =
        entries.into_iter().partition(Entry::is_complete);

    let start_label = format_local(start);
    let end_label = format_local(end);
    print!(
        "{}",
        render_overview(
            &complete,
            &incomplete,
            &start_label,
            &end_label,
            &config.table_sort_by,
            &config.table_hide_column,
        )
    );

    if complete.is_empty() {
        eprintln!("Nothing to upload.");
        return Ok(());
    }

    if config.dry_run {
        eprintln!("Dry run: skipping upload.");
        return Ok(());
    }

    if !config.yes {
        // `interact_opt` restores the cursor on Esc/q; Ctrl+C is handled by
        // the process-wide interrupt handler in `terminal`.
        let proceed = Confirm::new()
            .with_prompt("Continue uploading complete entries?")
            .default(false)
            .interact_opt()
            .context("failed to read confirmation")?;
        crate::terminal::restore_cursor();
        match proceed {
            Some(true) => {}
            Some(false) | None => bail!("aborted by user"),
        }
    }

    upload_with_progress(&target, &complete).await
}

fn build_filter(config: &Config) -> Result<EntryFilter> {
    let mut filter = EntryFilter::new();

    if let Some(pattern) = &config.filter_client {
        filter.client = Some(
            TextMatcher::regex(pattern)
                .with_context(|| format!("invalid filter-client `{pattern}`"))?,
        );
    }
    if let Some(pattern) = &config.filter_project {
        filter.project = Some(
            TextMatcher::regex(pattern)
                .with_context(|| format!("invalid filter-project `{pattern}`"))?,
        );
    }

    Ok(filter)
}

fn format_window(start: std::time::SystemTime, end: std::time::SystemTime) -> String {
    format!("{} .. {}", format_local(start), format_local(end))
}

fn format_local(time: std::time::SystemTime) -> String {
    DateTime::<Local>::from(time)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

#[cfg(test)]
mod tests {
    use crate::tracker::Tracker;

    #[test]
    fn trackers_display_as_kebab_names() {
        assert_eq!(Tracker::Toggl.to_string(), "toggl");
        assert_eq!(Tracker::Tempo.to_string(), "tempo");
    }
}
