//! Post-fetch / pre-push entry filtering.

use regex::Regex;

use super::Entry;

/// Matches a free-form string either exactly or via regex.
///
/// Exact matching never treats metacharacters specially. Regex matching uses
/// [`Regex::is_match`].
#[derive(Clone, Debug)]
pub enum TextMatcher {
    /// Full-string equality.
    Exact(String),
    /// Substring / pattern match via a compiled regular expression.
    Regex(Regex),
}

impl PartialEq for TextMatcher {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Exact(left), Self::Exact(right)) => left == right,
            (Self::Regex(left), Self::Regex(right)) => left.as_str() == right.as_str(),
            _ => false,
        }
    }
}

impl Eq for TextMatcher {}

impl TextMatcher {
    /// Creates an exact matcher.
    #[must_use]
    pub fn exact(value: impl Into<String>) -> Self {
        Self::Exact(value.into())
    }

    /// Compiles `pattern` into a regex matcher.
    ///
    /// # Errors
    ///
    /// Returns [`regex::Error`] when `pattern` is not a valid regular expression.
    pub fn regex(pattern: &str) -> Result<Self, regex::Error> {
        Ok(Self::Regex(Regex::new(pattern)?))
    }

    /// Returns `true` when `haystack` matches this matcher.
    #[must_use]
    pub fn matches(&self, haystack: &str) -> bool {
        match self {
            Self::Exact(expected) => haystack == expected,
            Self::Regex(regex) => regex.is_match(haystack),
        }
    }
}

/// Criteria for keeping worklog entries after fetch or before push.
///
/// Unset fields do not constrain. All set fields are combined with AND.
/// Client / project / task matchers compare against the display name when
/// present, otherwise the ID's [`Display`](std::fmt::Display) form.
///
/// # Examples
///
/// ```
/// use minutes_core::{
///     ClientId, Entry, EntryFilter, ProjectId, TaskId, TextMatcher, WorkInterval,
/// };
/// use std::time::{Duration, UNIX_EPOCH};
///
/// let entries = vec![
///     Entry::new(
///         ClientId::from("acme"),
///         ProjectId::from(1),
///         TaskId::from("api"),
///         WorkInterval::new(UNIX_EPOCH, Duration::from_secs(60), true),
///     )
///     .with_client_name("Acme Corp"),
///     Entry::new(
///         ClientId::from("other"),
///         ProjectId::from(2),
///         TaskId::from("ops"),
///         WorkInterval::new(UNIX_EPOCH, Duration::from_secs(30), false),
///     ),
/// ];
///
/// let filter = EntryFilter {
///     client: Some(TextMatcher::exact("Acme Corp")),
///     project: None,
///     task: None,
///     billable: Some(true),
/// };
///
/// assert_eq!(filter.apply(entries).len(), 1);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EntryFilter {
    /// Match against client name (or ID when name is missing).
    pub client: Option<TextMatcher>,
    /// Match against project name (or ID when name is missing).
    pub project: Option<TextMatcher>,
    /// Match against task name (or ID when name is missing).
    pub task: Option<TextMatcher>,
    /// When `Some(true)`, keep only billable entries; when `Some(false)`, only
    /// non-billable. `None` does not constrain billability.
    pub billable: Option<bool>,
}

impl EntryFilter {
    /// Creates an empty filter that matches every entry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            client: None,
            project: None,
            task: None,
            billable: None,
        }
    }

    /// Returns `true` when `entry` satisfies every set criterion.
    #[must_use]
    pub fn matches(&self, entry: &Entry) -> bool {
        if let Some(matcher) = &self.client
            && !matcher.matches(&display_or_id(entry.client_name.as_deref(), &entry.client))
        {
            return false;
        }

        if let Some(matcher) = &self.project
            && !matcher.matches(&display_or_id(
                entry.project_name.as_deref(),
                &entry.project,
            ))
        {
            return false;
        }

        if let Some(matcher) = &self.task
            && !matcher.matches(&display_or_id(entry.task_name.as_deref(), &entry.task))
        {
            return false;
        }

        if let Some(billable) = self.billable
            && entry.interval.is_billable() != billable
        {
            return false;
        }

        true
    }

    /// Keeps entries that match this filter, preserving order.
    #[must_use]
    pub fn apply(&self, entries: impl IntoIterator<Item = Entry>) -> Vec<Entry> {
        entries
            .into_iter()
            .filter(|entry| self.matches(entry))
            .collect()
    }

    /// Clones and keeps entries from `entries` that match this filter.
    #[must_use]
    pub fn filter_entries(&self, entries: &[Entry]) -> Vec<Entry> {
        entries
            .iter()
            .filter(|entry| self.matches(entry))
            .cloned()
            .collect()
    }
}

fn display_or_id(name: Option<&str>, id: &impl std::fmt::Display) -> String {
    match name {
        Some(name) if !name.is_empty() => name.to_owned(),
        _ => id.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClientId, ProjectId, TaskId, WorkInterval};
    use std::time::{Duration, UNIX_EPOCH};

    fn entry(client: &str, project: &str, task: &str, billable: bool) -> Entry {
        Entry::new(
            ClientId::from(client),
            ProjectId::from(project),
            TaskId::from(task),
            WorkInterval::new(UNIX_EPOCH, Duration::from_secs(60), billable),
        )
    }

    #[test]
    fn empty_filter_keeps_all() {
        let entries = vec![entry("a", "1", "t1", true), entry("b", "2", "t2", false)];
        assert_eq!(EntryFilter::new().apply(entries.clone()).len(), 2);
        assert_eq!(EntryFilter::default().filter_entries(&entries).len(), 2);
    }

    #[test]
    fn exact_matches_full_string_only() {
        let matcher = TextMatcher::exact("Acme");
        assert!(matcher.matches("Acme"));
        assert!(!matcher.matches("Acme Corp"));
        assert!(!matcher.matches("acme"));
    }

    #[test]
    fn regex_matches_pattern() {
        let matcher = TextMatcher::regex(r"(?i)^acme").expect("regex");
        assert!(matcher.matches("Acme"));
        assert!(matcher.matches("acme-corp"));
        assert!(!matcher.matches("Beta"));
    }

    #[test]
    fn filter_prefers_display_name_over_id() {
        let named = entry("acme-id", "1", "t", true).with_client_name("Acme Corp");
        let by_name = EntryFilter {
            client: Some(TextMatcher::exact("Acme Corp")),
            ..EntryFilter::new()
        };
        let by_id = EntryFilter {
            client: Some(TextMatcher::exact("acme-id")),
            ..EntryFilter::new()
        };

        assert!(by_name.matches(&named));
        assert!(!by_id.matches(&named));
    }

    #[test]
    fn filter_falls_back_to_id_when_name_missing() {
        let unnamed = entry("acme-id", "proj-7", "task-a", true);
        let filter = EntryFilter {
            client: Some(TextMatcher::exact("acme-id")),
            project: Some(TextMatcher::exact("proj-7")),
            task: Some(TextMatcher::exact("task-a")),
            billable: None,
        };
        assert!(filter.matches(&unnamed));
    }

    #[test]
    fn filter_billable_only() {
        let entries = vec![entry("a", "1", "t", true), entry("a", "1", "t", false)];
        let filter = EntryFilter {
            billable: Some(true),
            ..EntryFilter::new()
        };
        let kept = filter.apply(entries);
        assert_eq!(kept.len(), 1);
        assert!(kept[0].interval.is_billable());
    }

    #[test]
    fn filter_non_billable_only() {
        let entries = vec![entry("a", "1", "t", true), entry("a", "1", "t", false)];
        let filter = EntryFilter {
            billable: Some(false),
            ..EntryFilter::new()
        };
        let kept = filter.apply(entries);
        assert_eq!(kept.len(), 1);
        assert!(!kept[0].interval.is_billable());
    }

    #[test]
    fn filter_ands_client_project_and_billable() {
        let keep = entry("acme", "web", "api", true)
            .with_client_name("Acme")
            .with_project_name("Website");
        let drop_client = entry("other", "web", "api", true)
            .with_client_name("Other")
            .with_project_name("Website");
        let drop_project = entry("acme", "ops", "api", true)
            .with_client_name("Acme")
            .with_project_name("Ops");
        let drop_billable = entry("acme", "web", "api", false)
            .with_client_name("Acme")
            .with_project_name("Website");

        let filter = EntryFilter {
            client: Some(TextMatcher::regex(r"^Acme$").expect("regex")),
            project: Some(TextMatcher::exact("Website")),
            task: None,
            billable: Some(true),
        };

        assert!(filter.matches(&keep));
        assert!(!filter.matches(&drop_client));
        assert!(!filter.matches(&drop_project));
        assert!(!filter.matches(&drop_billable));
        assert_eq!(
            filter
                .apply([keep, drop_client, drop_project, drop_billable])
                .len(),
            1
        );
    }

    #[test]
    fn empty_display_name_falls_back_to_id() {
        let entry = entry("acme", "1", "t", true).with_client_name("");
        let filter = EntryFilter {
            client: Some(TextMatcher::exact("acme")),
            ..EntryFilter::new()
        };
        assert!(filter.matches(&entry));
    }
}
