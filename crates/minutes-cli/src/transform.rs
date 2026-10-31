//! Post-fetch entry transforms: merge, round, and force-billable.

use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Local};
use minutes_core::{Entry, WorkInterval};

/// Merges entries that share client, project, task, summary, and local calendar day.
///
/// Durations are summed via [`Entry::merge`]. Order of first appearance is preserved
/// for the merged result list.
#[must_use]
pub fn merge_by_day(entries: Vec<Entry>) -> Vec<Entry> {
    let mut order: Vec<MergeKey> = Vec::new();
    let mut groups: HashMap<MergeKey, Entry> = HashMap::new();

    for entry in entries {
        let key = MergeKey::from(&entry);
        if let Some(existing) = groups.remove(&key) {
            // MergeKey includes client/project/task ids, so merge cannot fail here.
            let merged = existing
                .merge(entry)
                .expect("merge key guarantees matching client/project/task");
            groups.insert(key, merged);
        } else {
            order.push(key.clone());
            groups.insert(key, entry);
        }
    }

    order
        .into_iter()
        .filter_map(|key| groups.remove(&key))
        .collect()
}

/// Rounds each entry's elapsed time to the closest minute.
///
/// Rule: fractional minutes with `secs % 60 >= 30` round up; otherwise down
/// (which may yield zero).
#[must_use]
pub fn round_to_closest_minute(entries: Vec<Entry>) -> Vec<Entry> {
    entries
        .into_iter()
        .map(|mut entry| {
            let secs = entry.interval.elapsed().as_secs();
            let rounded = round_secs_to_minute(secs);
            entry.interval = WorkInterval::new(
                entry.interval.start(),
                Duration::from_secs(rounded),
                entry.interval.is_billable(),
            );
            entry
        })
        .collect()
}

/// Marks every entry as billable, preserving start and elapsed time.
#[must_use]
pub fn force_billed_duration(entries: Vec<Entry>) -> Vec<Entry> {
    entries
        .into_iter()
        .map(|mut entry| {
            entry.interval =
                WorkInterval::new(entry.interval.start(), entry.interval.elapsed(), true);
            entry
        })
        .collect()
}

fn round_secs_to_minute(secs: u64) -> u64 {
    let rem = secs % 60;
    let whole = secs / 60;
    if rem >= 30 {
        (whole + 1) * 60
    } else {
        whole * 60
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct MergeKey {
    client: String,
    project: String,
    task: String,
    summary: String,
    day: String,
}

impl MergeKey {
    fn from(entry: &Entry) -> Self {
        let day = DateTime::<Local>::from(entry.interval.start())
            .format("%Y-%m-%d")
            .to_string();
        Self {
            client: entry.client.to_string(),
            project: entry.project.to_string(),
            task: entry.task.to_string(),
            summary: entry.summary.clone().unwrap_or_default(),
            day,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minutes_core::{ClientId, ProjectId, TaskId};
    use std::time::{Duration, UNIX_EPOCH};

    fn entry(task: &str, start_offset: u64, elapsed: u64, billable: bool, summary: &str) -> Entry {
        Entry::new(
            ClientId::from("acme"),
            ProjectId::from("web"),
            TaskId::from(task),
            WorkInterval::new(
                UNIX_EPOCH + Duration::from_secs(start_offset),
                Duration::from_secs(elapsed),
                billable,
            ),
        )
        .with_summary(summary)
    }

    #[test]
    fn merge_by_day_sums_same_key() {
        // Two entries on the same UTC day near epoch.
        let entries = vec![
            entry("TASK-1", 0, 60, true, "work"),
            entry("TASK-1", 120, 90, true, "work"),
        ];
        let merged = merge_by_day(entries);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].interval.elapsed(), Duration::from_secs(150));
    }

    #[test]
    fn merge_by_day_keeps_different_summaries_apart() {
        let entries = vec![
            entry("TASK-1", 0, 60, true, "a"),
            entry("TASK-1", 0, 60, true, "b"),
        ];
        assert_eq!(merge_by_day(entries).len(), 2);
    }

    #[test]
    fn round_to_closest_minute_up_and_down() {
        assert_eq!(round_secs_to_minute(29), 0);
        assert_eq!(round_secs_to_minute(30), 60);
        assert_eq!(round_secs_to_minute(89), 60);
        assert_eq!(round_secs_to_minute(90), 120);
    }

    #[test]
    fn force_billed_marks_non_billable() {
        let entries = force_billed_duration(vec![entry("TASK-1", 0, 60, false, "x")]);
        assert!(entries[0].interval.is_billable());
    }
}
