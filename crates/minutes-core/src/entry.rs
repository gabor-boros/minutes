//! Worklog entries and grouping.

use std::collections::HashMap;
use thiserror::Error;

use super::{ClientId, ProjectId, TaskId, WorkInterval};

/// Errors that can occur when merging worklog entries.
///
/// # Examples
///
/// ```
/// use minutes_core::{
///     ClientId, Entry, Error, ProjectId, TaskId, WorkInterval,
/// };
///
/// let entry1 = Entry::new(
///     ClientId::from("acme"),
///     ProjectId::from(7),
///     TaskId::from("api"),
///     WorkInterval::default(),
/// );
/// let entry2 = Entry::new(
///     ClientId::from("emca"),
///     ProjectId::from(7),
///     TaskId::from("api"),
///     WorkInterval::default(),
/// );
///
/// assert_eq!(
///     entry1.merge(entry2),
///     Err(Error::ClientsDiffer(
///         ClientId::from("acme"),
///         ClientId::from("emca"),
///     ))
/// );
/// ```
#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("merging entries with different clients {0} and {1}")]
    ClientsDiffer(ClientId, ClientId),
    #[error("merging entries with different projects {0} and {1}")]
    ProjectsDiffer(ProjectId, ProjectId),
    #[error("merging entries with different tasks {0} and {1}")]
    TasksDiffer(TaskId, TaskId),
}

/// A single tracked interval of work.
///
/// An entry ties a time span to a client/project/task hierarchy. Optional
/// display names, `summary`, and `notes` hold free-form text.
///
/// # Examples
///
/// ```
/// use minutes_core::{ClientId, Entry, ProjectId, TaskId, WorkInterval};
/// use std::time::{Duration, UNIX_EPOCH};
///
/// let entry = Entry::new(
///     ClientId::from("acme"),
///     ProjectId::from(7),
///     TaskId::from("api"),
///     WorkInterval::new(UNIX_EPOCH, Duration::from_secs(125), true),
/// )
/// .with_summary("Ship billing fix");
///
/// assert_eq!(entry.summary.as_deref(), Some("Ship billing fix"));
/// assert_eq!(entry.to_string(), "acme: 7 - api :: 0h 2m 5s (billable)");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Entry {
    /// Client the work was performed for.
    pub client: ClientId,
    /// Human-readable client name, if known.
    pub client_name: Option<String>,
    /// Project under the client.
    pub project: ProjectId,
    /// Human-readable project name, if known.
    pub project_name: Option<String>,
    /// Task within the project.
    pub task: TaskId,
    /// Human-readable task name, if known.
    pub task_name: Option<String>,
    /// Short human-readable title, if any.
    pub summary: Option<String>,
    /// Longer free-form notes, if any.
    pub notes: Option<String>,
    /// When the work ran and whether it is billable.
    pub interval: WorkInterval,
}

impl std::fmt::Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} - {} :: {}",
            format_labeled(self.client_name.as_deref(), &self.client),
            format_labeled(self.project_name.as_deref(), &self.project),
            format_labeled(self.task_name.as_deref(), &self.task),
            self.interval
        )
    }
}

/// Formats `Name (id)` when a display name is present; otherwise just the id.
fn format_labeled(name: Option<&str>, id: &impl std::fmt::Display) -> String {
    match name {
        Some(name) if !name.is_empty() => format!("{name} ({id})"),
        _ => id.to_string(),
    }
}

impl Entry {
    /// Creates an entry with no summary or notes.
    ///
    /// # Examples
    ///
    /// ```
    /// use minutes_core::{ClientId, Entry, ProjectId, TaskId, WorkInterval};
    /// use std::time::Duration;
    ///
    /// let entry = Entry::new(
    ///     ClientId::from(1),
    ///     ProjectId::from(2),
    ///     TaskId::from(3),
    ///     WorkInterval::default(),
    /// );
    ///
    /// assert!(entry.summary.is_none());
    /// assert!(entry.notes.is_none());
    /// assert_eq!(entry.interval.elapsed(), Duration::ZERO);
    /// ```
    #[must_use]
    pub fn new(client: ClientId, project: ProjectId, task: TaskId, interval: WorkInterval) -> Self {
        Self {
            client,
            client_name: None,
            project,
            project_name: None,
            task,
            task_name: None,
            summary: None,
            notes: None,
            interval,
        }
    }

    /// Sets the human-readable client name.
    #[must_use]
    pub fn with_client_name(mut self, name: impl Into<String>) -> Self {
        self.client_name = Some(name.into());
        self
    }

    /// Sets the human-readable project name.
    #[must_use]
    pub fn with_project_name(mut self, name: impl Into<String>) -> Self {
        self.project_name = Some(name.into());
        self
    }

    /// Sets the human-readable task name.
    #[must_use]
    pub fn with_task_name(mut self, name: impl Into<String>) -> Self {
        self.task_name = Some(name.into());
        self
    }

    /// Sets the short human-readable title.
    #[must_use]
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Sets the longer free-form notes.
    #[must_use]
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    /// Checks if the entry is complete.
    ///
    /// An entry is complete if it has a client, project, task, and a non-zero interval.
    ///
    /// # Examples
    ///
    /// ```
    /// use minutes_core::{ClientId, Entry, ProjectId, TaskId, WorkInterval};
    /// use std::time::{Duration, UNIX_EPOCH};
    ///
    /// let entry = Entry::new(
    ///     ClientId::from("acme"),
    ///     ProjectId::from(7),
    ///     TaskId::from("api"),
    ///     WorkInterval::default(),
    /// );
    /// assert!(!entry.is_complete());
    /// ```
    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self.client.is_empty()
            && !self.project.is_empty()
            && !self.task.is_empty()
            && self.interval.elapsed() > std::time::Duration::ZERO
    }

    /// Merges the current entry with another one.
    ///
    /// Combines intervals by taking the earlier start, summing elapsed time,
    /// and marking the result billable only if both sides are billable.
    /// Missing display names / summary / notes on `self` are filled from `other`.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when client, project, or task IDs differ.
    pub fn merge(mut self, other: Self) -> Result<Self, Error> {
        ensure_eq(&self.project, &other.project, Error::ProjectsDiffer)?;
        ensure_eq(&self.client, &other.client, Error::ClientsDiffer)?;
        ensure_eq(&self.task, &other.task, Error::TasksDiffer)?;

        self.interval = WorkInterval::new(
            self.interval.start().min(other.interval.start()),
            self.interval.elapsed() + other.interval.elapsed(),
            self.interval.is_billable() && other.interval.is_billable(),
        );

        self.client_name = self.client_name.or(other.client_name);
        self.project_name = self.project_name.or(other.project_name);
        self.task_name = self.task_name.or(other.task_name);
        self.summary = self.summary.or(other.summary);
        self.notes = self.notes.or(other.notes);

        Ok(self)
    }
}

fn ensure_eq<T>(left: &T, right: &T, err: impl FnOnce(T, T) -> Error) -> Result<(), Error>
where
    T: PartialEq + Clone,
{
    if left == right {
        Ok(())
    } else {
        Err(err(left.clone(), right.clone()))
    }
}

/// Groups worklog entries by their task ID.
///
/// Returned keys and values borrow from `entries`; the map is valid only for
/// as long as the input slice lives. Entries that share a task ID appear in
/// the same vector, in the order they occur in `entries`.
///
/// # Examples
///
/// ```
/// use minutes_core::{
///     group_by_task, ClientId, Entry, ProjectId, TaskId, WorkInterval,
/// };
/// use std::time::{Duration, UNIX_EPOCH};
///
/// let entries = [
///     Entry::new(
///         ClientId::from(1),
///         ProjectId::from(10),
///         TaskId::from("review"),
///         WorkInterval::new(UNIX_EPOCH, Duration::from_secs(10), true),
///     ),
///     Entry::new(
///         ClientId::from(1),
///         ProjectId::from(10),
///         TaskId::from("other"),
///         WorkInterval::new(UNIX_EPOCH, Duration::from_secs(20), true),
///     ),
///     Entry::new(
///         ClientId::from(1),
///         ProjectId::from(10),
///         TaskId::from("review"),
///         WorkInterval::new(UNIX_EPOCH, Duration::from_secs(30), true),
///     ),
/// ];
///
/// let groups = group_by_task(&entries);
/// assert_eq!(groups.len(), 2);
/// assert_eq!(
///     groups[&TaskId::from("review")],
///     vec![&entries[0], &entries[2]]
/// );
/// ```
#[must_use]
pub fn group_by_task(entries: &[Entry]) -> HashMap<&TaskId, Vec<&Entry>> {
    let mut groups: HashMap<&TaskId, Vec<&Entry>> = HashMap::with_capacity(entries.len());
    for entry in entries {
        groups.entry(&entry.task).or_default().push(entry);
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration as StdDuration, UNIX_EPOCH};

    fn entry_with_task(task: TaskId, secs: u64) -> Entry {
        Entry::new(
            ClientId::from(1),
            ProjectId::from(10),
            task,
            WorkInterval::new(
                UNIX_EPOCH + StdDuration::from_secs(1_700_000_000),
                StdDuration::from_secs(secs),
                true,
            ),
        )
    }

    fn mergeable(start_offset_secs: u64, elapsed_secs: u64, billable: bool) -> Entry {
        Entry::new(
            ClientId::from("acme"),
            ProjectId::from(7),
            TaskId::from("api"),
            WorkInterval::new(
                UNIX_EPOCH + StdDuration::from_secs(start_offset_secs),
                StdDuration::from_secs(elapsed_secs),
                billable,
            ),
        )
    }

    #[test]
    fn entry_new_leaves_summary_and_notes_empty() {
        let interval = WorkInterval::new(UNIX_EPOCH, StdDuration::from_secs(30), false);
        let entry = Entry::new(
            ClientId::from("client"),
            ProjectId::from(2),
            TaskId::from("task"),
            interval,
        );
        assert_eq!(entry.client, ClientId::from("client"));
        assert_eq!(entry.project, ProjectId::from(2));
        assert_eq!(entry.task, TaskId::from("task"));
        assert_eq!(entry.interval, interval);
        assert!(!entry.interval.is_billable());
        assert_eq!(entry.summary, None);
        assert_eq!(entry.notes, None);
        assert_eq!(entry.client_name, None);
        assert_eq!(entry.project_name, None);
        assert_eq!(entry.task_name, None);
    }

    #[test]
    fn entry_builders_set_summary_and_notes() {
        let entry = Entry::new(
            ClientId::from("acme"),
            ProjectId::from(7),
            TaskId::from("api"),
            WorkInterval::default(),
        )
        .with_summary("Title")
        .with_notes("Details");

        assert_eq!(entry.summary.as_deref(), Some("Title"));
        assert_eq!(entry.notes.as_deref(), Some("Details"));
    }

    #[test]
    fn entry_builders_set_display_names() {
        let entry = Entry::new(
            ClientId::from("acme"),
            ProjectId::from(7),
            TaskId::from("api"),
            WorkInterval::default(),
        )
        .with_client_name("Acme Corp")
        .with_project_name("Website")
        .with_task_name("API work");

        assert_eq!(entry.client_name.as_deref(), Some("Acme Corp"));
        assert_eq!(entry.project_name.as_deref(), Some("Website"));
        assert_eq!(entry.task_name.as_deref(), Some("API work"));
    }

    #[test]
    fn entry_display_includes_hierarchy_and_interval() {
        let entry = Entry::new(
            ClientId::from("acme"),
            ProjectId::from(7),
            TaskId::from("api"),
            WorkInterval::new(UNIX_EPOCH, StdDuration::from_secs(125), true),
        );
        assert_eq!(entry.to_string(), "acme: 7 - api :: 0h 2m 5s (billable)");
    }

    #[test]
    fn entry_display_prefers_names_when_present() {
        let entry = Entry::new(
            ClientId::from("acme"),
            ProjectId::from(7),
            TaskId::from("api"),
            WorkInterval::new(UNIX_EPOCH, StdDuration::from_secs(125), true),
        )
        .with_client_name("Acme")
        .with_project_name("Website");

        assert_eq!(
            entry.to_string(),
            "Acme (acme): Website (7) - api :: 0h 2m 5s (billable)"
        );
    }

    #[test]
    fn entry_is_complete_when_all_fields_are_present() {
        let entry = Entry::new(
            ClientId::from("acme"),
            ProjectId::from(7),
            TaskId::from("api"),
            WorkInterval::new(UNIX_EPOCH, StdDuration::from_secs(125), true),
        );
        assert!(entry.is_complete());
    }

    #[test]
    fn entry_is_not_complete_when_client_is_empty() {
        let entry = Entry::new(
            ClientId::from(""),
            ProjectId::from(7),
            TaskId::from("api"),
            WorkInterval::new(UNIX_EPOCH, StdDuration::from_secs(125), true),
        );
        assert!(!entry.is_complete());
    }

    #[test]
    fn entry_is_not_complete_when_project_is_empty() {
        let entry = Entry::new(
            ClientId::from("acme"),
            ProjectId::from(0),
            TaskId::from("api"),
            WorkInterval::new(UNIX_EPOCH, StdDuration::from_secs(125), true),
        );
        assert!(!entry.is_complete());
    }

    #[test]
    fn entry_is_not_complete_when_task_is_empty() {
        let entry = Entry::new(
            ClientId::from("acme"),
            ProjectId::from(7),
            TaskId::from(""),
            WorkInterval::new(UNIX_EPOCH, StdDuration::from_secs(125), true),
        );
        assert!(!entry.is_complete());
    }

    #[test]
    fn entry_is_not_complete_when_interval_is_zero() {
        let entry = Entry::new(
            ClientId::from("acme"),
            ProjectId::from(7),
            TaskId::from("api"),
            WorkInterval::new(UNIX_EPOCH, StdDuration::from_secs(0), true),
        );
        assert!(!entry.is_complete());
    }

    #[test]
    fn merge_sums_elapsed_and_keeps_earlier_start() {
        let left = mergeable(100, 30, true);
        let right = mergeable(50, 20, true);

        let merged = left.merge(right).expect("same identity");

        assert_eq!(merged.client, ClientId::from("acme"));
        assert_eq!(merged.project, ProjectId::from(7));
        assert_eq!(merged.task, TaskId::from("api"));
        assert_eq!(
            merged.interval.start(),
            UNIX_EPOCH + StdDuration::from_secs(50)
        );
        assert_eq!(merged.interval.elapsed(), StdDuration::from_secs(50));
        assert!(merged.interval.is_billable());
    }

    #[test]
    fn merge_uses_same_start_when_equal() {
        let left = mergeable(10, 5, true);
        let right = mergeable(10, 7, true);

        let merged = left.merge(right).expect("same identity");
        assert_eq!(
            merged.interval.start(),
            UNIX_EPOCH + StdDuration::from_secs(10)
        );
        assert_eq!(merged.interval.elapsed(), StdDuration::from_secs(12));
    }

    #[test]
    fn merge_billable_only_when_both_billable() {
        let both = mergeable(0, 1, true)
            .merge(mergeable(0, 1, true))
            .expect("same identity");
        assert!(both.interval.is_billable());

        let left_non = mergeable(0, 1, false)
            .merge(mergeable(0, 1, true))
            .expect("same identity");
        assert!(!left_non.interval.is_billable());

        let right_non = mergeable(0, 1, true)
            .merge(mergeable(0, 1, false))
            .expect("same identity");
        assert!(!right_non.interval.is_billable());

        let neither = mergeable(0, 1, false)
            .merge(mergeable(0, 1, false))
            .expect("same identity");
        assert!(!neither.interval.is_billable());
    }

    #[test]
    fn merge_prefers_left_summary_and_notes_when_present() {
        let left = mergeable(0, 1, true)
            .with_summary("left-summary")
            .with_notes("left-notes");
        let right = mergeable(0, 2, true)
            .with_summary("right-summary")
            .with_notes("right-notes");

        let merged = left.merge(right).expect("same identity");
        assert_eq!(merged.summary.as_deref(), Some("left-summary"));
        assert_eq!(merged.notes.as_deref(), Some("left-notes"));
    }

    #[test]
    fn merge_fills_summary_and_notes_from_right_when_left_empty() {
        let left = mergeable(0, 1, true);
        let right = mergeable(0, 2, true)
            .with_summary("right-summary")
            .with_notes("right-notes");

        let merged = left.merge(right).expect("same identity");
        assert_eq!(merged.summary.as_deref(), Some("right-summary"));
        assert_eq!(merged.notes.as_deref(), Some("right-notes"));
    }

    #[test]
    fn merge_leaves_summary_and_notes_none_when_both_empty() {
        let merged = mergeable(0, 1, true)
            .merge(mergeable(0, 2, true))
            .expect("same identity");
        assert_eq!(merged.summary, None);
        assert_eq!(merged.notes, None);
    }

    #[test]
    fn merge_fills_only_missing_text_fields() {
        let left = mergeable(0, 1, true).with_summary("kept");
        let right = mergeable(0, 2, true)
            .with_summary("ignored")
            .with_notes("filled");

        let merged = left.merge(right).expect("same identity");
        assert_eq!(merged.summary.as_deref(), Some("kept"));
        assert_eq!(merged.notes.as_deref(), Some("filled"));
    }

    #[test]
    fn merge_fills_display_names_from_right_when_left_empty() {
        let left = mergeable(0, 1, true);
        let right = mergeable(0, 2, true)
            .with_client_name("Acme")
            .with_project_name("Website")
            .with_task_name("API");

        let merged = left.merge(right).expect("same identity");
        assert_eq!(merged.client_name.as_deref(), Some("Acme"));
        assert_eq!(merged.project_name.as_deref(), Some("Website"));
        assert_eq!(merged.task_name.as_deref(), Some("API"));
    }

    #[test]
    fn merge_prefers_left_display_names_when_present() {
        let left = mergeable(0, 1, true)
            .with_client_name("Left Client")
            .with_project_name("Left Project");
        let right = mergeable(0, 2, true)
            .with_client_name("Right Client")
            .with_project_name("Right Project")
            .with_task_name("Right Task");

        let merged = left.merge(right).expect("same identity");
        assert_eq!(merged.client_name.as_deref(), Some("Left Client"));
        assert_eq!(merged.project_name.as_deref(), Some("Left Project"));
        assert_eq!(merged.task_name.as_deref(), Some("Right Task"));
    }

    #[test]
    fn merge_rejects_different_clients() {
        let left = mergeable(0, 1, true);
        let right = Entry::new(
            ClientId::from("other"),
            ProjectId::from(7),
            TaskId::from("api"),
            WorkInterval::default(),
        );

        assert_eq!(
            left.merge(right),
            Err(Error::ClientsDiffer(
                ClientId::from("acme"),
                ClientId::from("other"),
            ))
        );
    }

    #[test]
    fn merge_rejects_different_projects() {
        let left = mergeable(0, 1, true);
        let right = Entry::new(
            ClientId::from("acme"),
            ProjectId::from(99),
            TaskId::from("api"),
            WorkInterval::default(),
        );

        assert_eq!(
            left.merge(right),
            Err(Error::ProjectsDiffer(
                ProjectId::from(7),
                ProjectId::from(99)
            ))
        );
    }

    #[test]
    fn merge_rejects_different_tasks() {
        let left = mergeable(0, 1, true);
        let right = Entry::new(
            ClientId::from("acme"),
            ProjectId::from(7),
            TaskId::from("other"),
            WorkInterval::default(),
        );

        assert_eq!(
            left.merge(right),
            Err(Error::TasksDiffer(
                TaskId::from("api"),
                TaskId::from("other"),
            ))
        );
    }

    #[test]
    fn merge_reports_project_mismatch_before_client_when_both_differ() {
        // `ensure_eq` checks project, then client, then task.
        let left = mergeable(0, 1, true);
        let right = Entry::new(
            ClientId::from("other"),
            ProjectId::from(99),
            TaskId::from("other"),
            WorkInterval::default(),
        );

        assert_eq!(
            left.merge(right),
            Err(Error::ProjectsDiffer(
                ProjectId::from(7),
                ProjectId::from(99)
            ))
        );
    }

    #[test]
    fn merge_reports_client_mismatch_before_task_when_both_differ() {
        let left = mergeable(0, 1, true);
        let right = Entry::new(
            ClientId::from("other"),
            ProjectId::from(7),
            TaskId::from("other"),
            WorkInterval::default(),
        );

        assert_eq!(
            left.merge(right),
            Err(Error::ClientsDiffer(
                ClientId::from("acme"),
                ClientId::from("other"),
            ))
        );
    }

    #[test]
    fn merge_can_be_chained_across_three_entries() {
        let merged = mergeable(30, 10, true)
            .merge(mergeable(10, 20, true))
            .expect("first merge")
            .merge(mergeable(5, 5, false))
            .expect("second merge");

        assert_eq!(
            merged.interval.start(),
            UNIX_EPOCH + StdDuration::from_secs(5)
        );
        assert_eq!(merged.interval.elapsed(), StdDuration::from_secs(35));
        assert!(!merged.interval.is_billable());
    }

    #[test]
    fn merge_zero_elapsed_entries() {
        let merged = mergeable(0, 0, true)
            .merge(mergeable(0, 0, true))
            .expect("same identity");
        assert_eq!(merged.interval.elapsed(), StdDuration::ZERO);
        assert_eq!(merged.interval.start(), UNIX_EPOCH);
    }

    #[test]
    fn group_by_task_empty_slice_returns_empty_map() {
        let groups = group_by_task(&[]);
        assert!(groups.is_empty());
    }

    #[test]
    fn group_by_task_single_entry() {
        let entries = [entry_with_task(TaskId::from(100), 60)];
        let groups = group_by_task(&entries);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[&TaskId::from(100)], vec![&entries[0]]);
    }

    #[test]
    fn group_by_task_merges_same_task_preserving_order() {
        let task = TaskId::from("review");
        let entries = [
            entry_with_task(TaskId::from("review"), 10),
            entry_with_task(TaskId::from("other"), 20),
            entry_with_task(TaskId::from("review"), 30),
        ];
        let groups = group_by_task(&entries);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[&task], vec![&entries[0], &entries[2]]);
        assert_eq!(groups[&TaskId::from("other")], vec![&entries[1]]);
    }

    #[test]
    fn group_by_task_keeps_string_and_integer_task_ids_separate() {
        let entries = [
            entry_with_task(TaskId::from("42"), 5),
            entry_with_task(TaskId::from(42), 15),
        ];
        let groups = group_by_task(&entries);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[&TaskId::from("42")].len(), 1);
        assert_eq!(groups[&TaskId::from(42)].len(), 1);
    }

    #[test]
    fn group_by_task_all_distinct_tasks() {
        let entries = [
            entry_with_task(TaskId::from(1), 1),
            entry_with_task(TaskId::from(2), 2),
            entry_with_task(TaskId::from(3), 3),
        ];
        let groups = group_by_task(&entries);
        assert_eq!(groups.len(), 3);
        for entry in &entries {
            assert_eq!(groups[&entry.task], vec![entry]);
        }
    }
}
