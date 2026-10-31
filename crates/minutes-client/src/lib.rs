//! Shared client abstractions for Minutes backends.

pub mod cmd;
pub mod http;

use std::error::Error;
use std::time::SystemTime;

use async_trait::async_trait;

use minutes_core::{Entry, EntryFilter};

pub use cmd::{CmdClient, CmdClientError, CommandRunner, TokioCommandRunner};
pub use http::{DEFAULT_TIMEOUT, HttpClient, HttpClientError};

/// Options controlling a worklog fetch from a [`WorklogSource`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FetchOpts {
    /// Inclusive start of the fetch window.
    pub start: SystemTime,
    /// Inclusive end of the fetch window.
    pub end: SystemTime,
}

impl FetchOpts {
    /// Creates fetch options for the given time window.
    ///
    /// Callers should ensure `start <= end`; backends may reject inverted windows.
    #[must_use]
    pub const fn new(start: SystemTime, end: SystemTime) -> Self {
        Self { start, end }
    }

    /// Returns `true` when `start` is after `end`.
    #[must_use]
    pub fn is_inverted(self) -> bool {
        self.start > self.end
    }
}

/// A time-tracking backend that can fetch worklog entries.
#[async_trait]
pub trait WorklogSource: Send + Sync {
    /// Error type produced by this backend.
    type Error: Error + Send + Sync + 'static;

    /// Fetch worklog entries from the backend for the window in `opts`.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend request fails.
    async fn fetch_entries(&self, opts: &FetchOpts) -> Result<Vec<Entry>, Self::Error>;
}

/// A time-tracking backend that can push worklog entries.
#[async_trait]
pub trait WorklogSink: Send + Sync {
    /// Error type produced by this backend.
    type Error: Error + Send + Sync + 'static;

    /// Push worklog entries to the backend.
    ///
    /// # Errors
    ///
    /// Returns an error when the backend request fails.
    async fn push_entries(&self, entries: &[Entry]) -> Result<(), Self::Error>;
}

/// Fetches entries from `source`, then keeps those matching `filter`.
///
/// Filtering is applied locally after the backend returns, so every source
/// gets the same client / project / billable semantics.
///
/// # Errors
///
/// Returns the backend error from [`WorklogSource::fetch_entries`].
pub async fn fetch_filtered<S: WorklogSource>(
    source: &S,
    opts: &FetchOpts,
    filter: &EntryFilter,
) -> Result<Vec<Entry>, S::Error> {
    let entries = source.fetch_entries(opts).await?;
    Ok(filter.apply(entries))
}

/// Pushes only the entries that match `filter` to `sink`.
///
/// # Errors
///
/// Returns the backend error from [`WorklogSink::push_entries`].
pub async fn push_filtered<S: WorklogSink>(
    sink: &S,
    entries: &[Entry],
    filter: &EntryFilter,
) -> Result<(), S::Error> {
    let filtered = filter.filter_entries(entries);
    sink.push_entries(&filtered).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use minutes_core::{ClientId, ProjectId, TaskId, TextMatcher, WorkInterval};
    use std::sync::Mutex;
    use std::time::{Duration, UNIX_EPOCH};

    #[derive(Debug)]
    struct FakeError;

    impl std::fmt::Display for FakeError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "fake error")
        }
    }

    impl Error for FakeError {}

    struct FakeSource {
        entries: Vec<Entry>,
    }

    #[async_trait]
    impl WorklogSource for FakeSource {
        type Error = FakeError;

        async fn fetch_entries(&self, _opts: &FetchOpts) -> Result<Vec<Entry>, Self::Error> {
            Ok(self.entries.clone())
        }
    }

    struct FakeSink {
        pushed: Mutex<Vec<Entry>>,
    }

    #[async_trait]
    impl WorklogSink for FakeSink {
        type Error = FakeError;

        async fn push_entries(&self, entries: &[Entry]) -> Result<(), Self::Error> {
            self.pushed.lock().expect("lock").extend_from_slice(entries);
            Ok(())
        }
    }

    fn sample_entry(client: &str, billable: bool) -> Entry {
        Entry::new(
            ClientId::from(client),
            ProjectId::from("proj"),
            TaskId::from("task"),
            WorkInterval::new(UNIX_EPOCH, Duration::from_secs(60), billable),
        )
        .with_client_name(client)
    }

    #[test]
    fn fetch_opts_detects_inverted_window() {
        let start = UNIX_EPOCH + Duration::from_secs(10);
        let end = UNIX_EPOCH + Duration::from_secs(5);
        assert!(FetchOpts::new(start, end).is_inverted());
        assert!(!FetchOpts::new(end, start).is_inverted());
        assert!(!FetchOpts::new(start, start).is_inverted());
    }

    #[tokio::test]
    async fn fetch_filtered_keeps_matching_entries() {
        let source = FakeSource {
            entries: vec![
                sample_entry("Acme", true),
                sample_entry("Other", true),
                sample_entry("Acme", false),
            ],
        };
        let filter = EntryFilter {
            client: Some(TextMatcher::exact("Acme")),
            project: None,
            task: None,
            billable: Some(true),
        };
        let opts = FetchOpts::new(UNIX_EPOCH, UNIX_EPOCH + Duration::from_secs(1));

        let kept = fetch_filtered(&source, &opts, &filter)
            .await
            .expect("fetch");
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].client_name.as_deref(), Some("Acme"));
        assert!(kept[0].interval.is_billable());
    }

    #[tokio::test]
    async fn push_filtered_uploads_only_matching_entries() {
        let sink = FakeSink {
            pushed: Mutex::new(Vec::new()),
        };
        let entries = vec![
            sample_entry("Acme", true),
            sample_entry("Acme", false),
            sample_entry("Other", true),
        ];
        let filter = EntryFilter {
            billable: Some(true),
            ..EntryFilter::new()
        };

        push_filtered(&sink, &entries, &filter).await.expect("push");

        let pushed = sink.pushed.lock().expect("lock");
        assert_eq!(pushed.len(), 2);
        assert!(pushed.iter().all(|entry| entry.interval.is_billable()));
    }
}
