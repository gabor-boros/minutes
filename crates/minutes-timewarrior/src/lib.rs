//! Timewarrior backend for Minutes.
//!
//! Implements [`WorklogSource`] and [`WorklogSink`] against the Timewarrior
//! CLI via [`CmdClient`]:
//! - fetch: `timew export from <start> to <end>`
//! - push: `timew track <start> - <end> <tags…>` then optional `timew annotate @1 <text>`
//!
//! Client, project, task, and billable state are encoded in tags; annotations
//! map to summary/notes.

use std::sync::Arc;
use std::time::SystemTime;

use async_trait::async_trait;
use chrono::{DateTime, Local, NaiveDateTime, Utc};
use minutes_client::{
    CmdClient, CmdClientError, CommandRunner, FetchOpts, WorklogSink, WorklogSource,
};
use minutes_core::{ClientId, Entry, ProjectId, TaskId, WorkInterval};
use regex::Regex;
use serde::Deserialize;
use thiserror::Error;

/// Default Timewarrior executable name.
const DEFAULT_PROGRAM: &str = "timew";

/// Compact UTC timestamp format used by Timewarrior export/track.
const COMPACT_UTC: &str = "%Y%m%dT%H%M%SZ";

/// Local timestamp format used in export range filters.
const LOCAL_FILTER: &str = "%Y-%m-%dT%H:%M:%S";

/// Errors produced by the Timewarrior backend adapter.
#[derive(Error, Debug)]
pub enum TimewarriorError {
    /// Underlying CLI client failure.
    #[error(transparent)]
    Cmd(#[from] CmdClientError),
    /// Export stdout could not be decoded as JSON.
    #[error("failed to decode export JSON: {0}")]
    Decode(#[from] serde_json::Error),
    /// Program name was empty.
    #[error("command program is required")]
    CommandRequired,
    /// A tag regex failed to compile.
    #[error("invalid regex: {0}")]
    InvalidRegex(#[from] regex::Error),
    /// [`FetchOpts`] has `start` after `end`.
    #[error("fetch window is inverted: start is after end")]
    InvertedFetchWindow,
    /// An interval timestamp could not be parsed or was inverted.
    #[error("invalid interval: {0}")]
    InvalidInterval(String),
}

/// Timewarrior client that fetches and pushes intervals through the `timew` CLI.
#[derive(Debug)]
pub struct TimewarriorClient {
    cmd: CmdClient,
    unbillable_tag: String,
    client_tag_regex: Option<Regex>,
    project_tag_regex: Option<Regex>,
    tags_as_tasks: Option<Regex>,
}

impl TimewarriorClient {
    /// Creates a client using the default `timew` program.
    ///
    /// # Errors
    ///
    /// Returns [`TimewarriorError::CommandRequired`] when the default program
    /// name cannot be used (should not happen).
    pub fn new() -> Result<Self, TimewarriorError> {
        Self::with_program(DEFAULT_PROGRAM)
    }

    /// Creates a client for an explicit CLI program name.
    ///
    /// # Errors
    ///
    /// Returns [`TimewarriorError::CommandRequired`] when `program` is empty.
    pub fn with_program(program: impl Into<String>) -> Result<Self, TimewarriorError> {
        let cmd = CmdClient::new(program).map_err(|err| match err {
            CmdClientError::CommandRequired => TimewarriorError::CommandRequired,
            other => TimewarriorError::Cmd(other),
        })?;
        Ok(Self {
            cmd,
            unbillable_tag: String::new(),
            client_tag_regex: None,
            project_tag_regex: None,
            tags_as_tasks: None,
        })
    }

    /// Replaces the underlying [`CmdClient`] (useful for tests).
    #[must_use]
    pub fn with_cmd(mut self, cmd: CmdClient) -> Self {
        self.cmd = cmd;
        self
    }

    /// Replaces extra CLI arguments prepended on every `timew` invocation.
    #[must_use]
    pub fn with_default_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.cmd = self.cmd.with_replaced_default_args(args);
        self
    }

    /// Injects a custom command runner while keeping the current program name.
    #[must_use]
    pub fn with_runner(mut self, runner: Arc<dyn CommandRunner>) -> Self {
        self.cmd = self.cmd.with_runner(runner);
        self
    }

    /// Tag that marks an interval as non-billable when present on fetch/push.
    #[must_use]
    pub fn with_unbillable_tag(mut self, tag: impl Into<String>) -> Self {
        self.unbillable_tag = tag.into();
        self
    }

    /// Tags matching `pattern` are treated as the client on fetch.
    ///
    /// # Errors
    ///
    /// Returns [`TimewarriorError::InvalidRegex`] when `pattern` is invalid.
    pub fn with_client_tag_regex(mut self, pattern: &str) -> Result<Self, TimewarriorError> {
        self.client_tag_regex = Some(Regex::new(pattern)?);
        Ok(self)
    }

    /// Tags matching `pattern` are treated as the project on fetch.
    ///
    /// # Errors
    ///
    /// Returns [`TimewarriorError::InvalidRegex`] when `pattern` is invalid.
    pub fn with_project_tag_regex(mut self, pattern: &str) -> Result<Self, TimewarriorError> {
        self.project_tag_regex = Some(Regex::new(pattern)?);
        Ok(self)
    }

    /// Tags matching `regex` are treated as tasks when fetching.
    ///
    /// Matching tags replace the entry task and split the duration evenly
    /// across matches. When the regex is set and tags are present but none
    /// match, the interval is dropped (same semantics as other backends).
    #[must_use]
    pub fn with_tags_as_tasks(mut self, regex: Regex) -> Self {
        self.tags_as_tasks = Some(regex);
        self
    }

    fn parse_entries(&self, fetched: Vec<FetchEntry>) -> Result<Vec<Entry>, TimewarriorError> {
        let mut entries = Vec::new();
        for raw in fetched {
            entries.extend(self.parse_entry(raw)?);
        }
        Ok(entries)
    }

    fn parse_entry(&self, raw: FetchEntry) -> Result<Vec<Entry>, TimewarriorError> {
        let Some(end_raw) = raw.end else {
            return Ok(Vec::new());
        };

        let start = parse_compact_utc(&raw.start)?;
        let end = parse_compact_utc(&end_raw)?;
        let elapsed = end
            .duration_since(start)
            .map_err(|_| TimewarriorError::InvalidInterval("end before start".to_owned()))?;

        let annotation = raw.annotation.unwrap_or_default();
        let mut billable = true;
        let mut client = String::new();
        let mut project = String::new();
        let mut task = String::new();

        for tag in &raw.tags {
            if !self.unbillable_tag.is_empty() && tag == &self.unbillable_tag {
                billable = false;
            } else if self
                .client_tag_regex
                .as_ref()
                .is_some_and(|re| re.is_match(tag))
            {
                client.clone_from(tag);
            } else if self
                .project_tag_regex
                .as_ref()
                .is_some_and(|re| re.is_match(tag))
            {
                project.clone_from(tag);
            } else if self
                .tags_as_tasks
                .as_ref()
                .is_some_and(|re| re.is_match(tag))
            {
                task.clone_from(tag);
            }
        }

        if task.is_empty() {
            task.clone_from(&annotation);
        }

        let interval = WorkInterval::new(start, elapsed, billable);
        let mut entry = Entry::new(
            ClientId::from(client.as_str()),
            ProjectId::from(project.as_str()),
            TaskId::from(task.as_str()),
            interval,
        );

        if !client.is_empty() {
            entry = entry.with_client_name(client);
        }
        if !project.is_empty() {
            entry = entry.with_project_name(project);
        }
        if !task.is_empty() {
            entry = entry.with_task_name(task);
        }
        if !annotation.is_empty() {
            entry = entry
                .with_summary(annotation.clone())
                .with_notes(annotation.clone());
        }

        if let Some(regex) = &self.tags_as_tasks
            && !raw.tags.is_empty()
        {
            return Ok(split_by_tags_as_tasks(
                &entry,
                &annotation,
                regex,
                &raw.tags,
            ));
        }

        Ok(vec![entry])
    }

    fn push_tags(entry: &Entry, unbillable_tag: &str) -> Vec<String> {
        let mut tags = Vec::new();

        push_label_tag(&mut tags, entry.client_name.as_deref(), &entry.client);
        push_label_tag(&mut tags, entry.project_name.as_deref(), &entry.project);
        push_label_tag(&mut tags, entry.task_name.as_deref(), &entry.task);

        if !entry.interval.is_billable() && !unbillable_tag.is_empty() {
            tags.push(unbillable_tag.to_owned());
        }

        tags
    }
}

impl Default for TimewarriorClient {
    fn default() -> Self {
        Self::new().expect("default timew program")
    }
}

#[async_trait]
impl WorklogSource for TimewarriorClient {
    type Error = TimewarriorError;

    async fn fetch_entries(&self, opts: &FetchOpts) -> Result<Vec<Entry>, Self::Error> {
        if opts.is_inverted() {
            return Err(TimewarriorError::InvertedFetchWindow);
        }

        let stdout = self
            .cmd
            .run([
                "export",
                "from",
                &format_local_filter(opts.start),
                "to",
                &format_local_filter(opts.end),
            ])
            .await?;

        let fetched: Vec<FetchEntry> = serde_json::from_slice(&stdout)?;
        self.parse_entries(fetched)
    }
}

#[async_trait]
impl WorklogSink for TimewarriorClient {
    type Error = TimewarriorError;

    async fn push_entries(&self, entries: &[Entry]) -> Result<(), Self::Error> {
        for entry in entries {
            let start = format_compact_utc(entry.interval.start());
            let end = entry
                .interval
                .end()
                .ok_or_else(|| TimewarriorError::InvalidInterval("interval end overflow".into()))?;
            let end = format_compact_utc(end);

            let mut args = vec!["track".to_owned(), start, "-".to_owned(), end];
            args.extend(Self::push_tags(entry, &self.unbillable_tag));
            self.cmd.run(&args).await?;

            let annotation = entry
                .summary
                .as_deref()
                .filter(|s| !s.is_empty())
                .or_else(|| entry.notes.as_deref().filter(|s| !s.is_empty()));

            if let Some(text) = annotation {
                self.cmd.run(["annotate", "@1", text]).await?;
            }
        }

        Ok(())
    }
}

/// Interval object returned by `timew export`.
#[derive(Debug, Deserialize)]
struct FetchEntry {
    #[serde(default)]
    start: String,
    #[serde(default)]
    end: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    annotation: Option<String>,
}

fn format_local_filter(time: SystemTime) -> String {
    DateTime::<Local>::from(time)
        .format(LOCAL_FILTER)
        .to_string()
}

fn format_compact_utc(time: SystemTime) -> String {
    DateTime::<Utc>::from(time).format(COMPACT_UTC).to_string()
}

fn parse_compact_utc(value: &str) -> Result<SystemTime, TimewarriorError> {
    let naive = NaiveDateTime::parse_from_str(value, COMPACT_UTC).map_err(|err| {
        TimewarriorError::InvalidInterval(format!("cannot parse '{value}': {err}"))
    })?;
    let utc = DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc);
    Ok(SystemTime::from(utc))
}

fn push_label_tag(tags: &mut Vec<String>, name: Option<&str>, id: &impl std::fmt::Display) {
    if let Some(name) = name.filter(|name| !name.is_empty()) {
        tags.push(name.to_owned());
        return;
    }
    let id = id.to_string();
    if !id.is_empty() && id != "0" {
        tags.push(id);
    }
}

/// Splits `entry` into one entry per tag whose name matches `regex`.
///
/// Duration is divided evenly across matching tags. Tags that do not match are
/// ignored; if none match, an empty list is returned.
fn split_by_tags_as_tasks(
    entry: &Entry,
    summary: &str,
    regex: &Regex,
    tags: &[String],
) -> Vec<Entry> {
    let tasks: Vec<&String> = tags.iter().filter(|tag| regex.is_match(tag)).collect();
    if tasks.is_empty() {
        return Vec::new();
    }

    let parts = u32::try_from(tasks.len()).unwrap_or(1).max(1);
    let split_elapsed = entry.interval.elapsed() / parts;

    tasks
        .into_iter()
        .map(|tag| {
            let mut split = Entry::new(
                entry.client.clone(),
                entry.project.clone(),
                TaskId::from(tag.as_str()),
                WorkInterval::new(
                    entry.interval.start(),
                    split_elapsed,
                    entry.interval.is_billable(),
                ),
            );

            if let Some(name) = &entry.client_name {
                split = split.with_client_name(name.clone());
            }
            if let Some(name) = &entry.project_name {
                split = split.with_project_name(name.clone());
            }
            if !tag.is_empty() {
                split = split.with_task_name(tag.clone());
            }
            if !summary.is_empty() {
                split = split.with_summary(summary.to_owned());
            }
            if let Some(notes) = &entry.notes {
                split = split.with_notes(notes.clone());
            }

            split
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use minutes_client::CommandRunner;
    use std::sync::Mutex;
    use std::time::Duration as StdDuration;

    struct FakeRunner {
        calls: Mutex<Vec<Vec<String>>>,
        responses: Mutex<Vec<Result<Vec<u8>, CmdClientError>>>,
    }

    impl FakeRunner {
        fn new(responses: Vec<Result<Vec<u8>, CmdClientError>>) -> Arc<Self> {
            Arc::new(Self {
                calls: Mutex::new(Vec::new()),
                responses: Mutex::new(responses),
            })
        }
    }

    #[async_trait]
    impl CommandRunner for FakeRunner {
        async fn output(&self, _program: &str, args: &[String]) -> Result<Vec<u8>, CmdClientError> {
            self.calls.lock().expect("lock").push(args.to_vec());
            let mut responses = self.responses.lock().expect("lock");
            if responses.is_empty() {
                return Ok(Vec::new());
            }
            responses.remove(0)
        }
    }

    fn sample_window() -> (SystemTime, SystemTime) {
        let start = Utc
            .with_ymd_and_hms(2021, 10, 2, 9, 0, 0)
            .single()
            .expect("start");
        let end = Utc
            .with_ymd_and_hms(2021, 10, 2, 10, 0, 0)
            .single()
            .expect("end");
        (SystemTime::from(start), SystemTime::from(end))
    }

    fn client_with_runner(runner: Arc<dyn CommandRunner>) -> TimewarriorClient {
        let cmd = CmdClient::new("timew").expect("cmd").with_runner(runner);
        TimewarriorClient::with_program("timew")
            .expect("client")
            .with_cmd(cmd)
            .with_unbillable_tag("unbillable")
            .with_client_tag_regex(r"^client:.*")
            .expect("client regex")
            .with_project_tag_regex(r"^project:.*")
            .expect("project regex")
    }

    #[tokio::test]
    async fn fetch_entries_rejects_inverted_window() {
        let runner = FakeRunner::new(vec![]);
        let client = client_with_runner(runner.clone());
        let (start, end) = sample_window();

        let err = client
            .fetch_entries(&FetchOpts::new(end, start))
            .await
            .expect_err("inverted");
        assert!(matches!(err, TimewarriorError::InvertedFetchWindow));
        assert!(runner.calls.lock().expect("lock").is_empty());
    }

    #[tokio::test]
    async fn fetch_entries_maps_export_payload() {
        let export = br#"[
            {
                "id": 1,
                "start": "20211002T090000Z",
                "end": "20211002T100000Z",
                "tags": ["client:acme", "project:website", "TASK-1", "unbillable"],
                "annotation": "Ship billing fix"
            }
        ]"#;
        let runner = FakeRunner::new(vec![Ok(export.to_vec())]);
        let client = client_with_runner(runner.clone())
            .with_tags_as_tasks(Regex::new(r"^TASK-\d+$").expect("regex"));

        let (start, end) = sample_window();
        let entries = client
            .fetch_entries(&FetchOpts::new(start, end))
            .await
            .expect("fetch");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].client, ClientId::from("client:acme"));
        assert_eq!(entries[0].project, ProjectId::from("project:website"));
        assert_eq!(entries[0].task, TaskId::from("TASK-1"));
        assert_eq!(entries[0].summary.as_deref(), Some("Ship billing fix"));
        assert_eq!(entries[0].notes.as_deref(), Some("Ship billing fix"));
        assert!(!entries[0].interval.is_billable());
        assert_eq!(entries[0].interval.elapsed(), StdDuration::from_secs(3600));

        let calls = runner.calls.lock().expect("lock");
        assert_eq!(calls[0][0], "export");
        assert_eq!(calls[0][1], "from");
        assert_eq!(calls[0][3], "to");
    }

    #[tokio::test]
    async fn fetch_entries_tags_as_tasks_splits_matching_tags() {
        let export = br#"[
            {
                "id": 1,
                "start": "20211002T090000Z",
                "end": "20211002T100000Z",
                "tags": ["client:acme", "project:website", "TASK-1", "TASK-2"],
                "annotation": "Pair work"
            }
        ]"#;
        let runner = FakeRunner::new(vec![Ok(export.to_vec())]);
        let client = client_with_runner(runner)
            .with_tags_as_tasks(Regex::new(r"^TASK-\d+$").expect("regex"));

        let (start, end) = sample_window();
        let entries = client
            .fetch_entries(&FetchOpts::new(start, end))
            .await
            .expect("fetch");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].task, TaskId::from("TASK-1"));
        assert_eq!(entries[1].task, TaskId::from("TASK-2"));
        assert_eq!(entries[0].interval.elapsed(), StdDuration::from_mins(30));
        assert_eq!(entries[1].interval.elapsed(), StdDuration::from_mins(30));
    }

    #[tokio::test]
    async fn push_entries_tracks_and_annotates() {
        let runner = FakeRunner::new(vec![Ok(Vec::new()), Ok(Vec::new())]);
        let client = client_with_runner(runner.clone());
        let (start, end) = sample_window();

        let entry = Entry::new(
            ClientId::from("client:acme"),
            ProjectId::from("project:website"),
            TaskId::from("TASK-1"),
            WorkInterval::new(start, end.duration_since(start).expect("duration"), true),
        )
        .with_client_name("client:acme")
        .with_project_name("project:website")
        .with_task_name("TASK-1")
        .with_summary("Ship billing fix")
        .with_notes("details");

        client.push_entries(&[entry]).await.expect("push");

        let calls = runner.calls.lock().expect("lock");
        assert_eq!(calls.len(), 2);
        assert_eq!(
            calls[0],
            vec![
                "track".to_owned(),
                "20211002T090000Z".to_owned(),
                "-".to_owned(),
                "20211002T100000Z".to_owned(),
                "client:acme".to_owned(),
                "project:website".to_owned(),
                "TASK-1".to_owned(),
            ]
        );
        assert_eq!(
            calls[1],
            vec![
                "annotate".to_owned(),
                "@1".to_owned(),
                "Ship billing fix".to_owned(),
            ]
        );
    }

    #[tokio::test]
    async fn push_entries_adds_unbillable_tag_and_skips_annotate() {
        let runner = FakeRunner::new(vec![Ok(Vec::new())]);
        let client = client_with_runner(runner.clone());
        let (start, end) = sample_window();

        let entry = Entry::new(
            ClientId::from("client:acme"),
            ProjectId::from("project:website"),
            TaskId::from("TASK-1"),
            WorkInterval::new(start, end.duration_since(start).expect("duration"), false),
        )
        .with_client_name("client:acme")
        .with_project_name("project:website")
        .with_task_name("TASK-1");

        client.push_entries(&[entry]).await.expect("push");

        let calls = runner.calls.lock().expect("lock");
        assert_eq!(calls.len(), 1);
        assert!(calls[0].contains(&"unbillable".to_owned()));
    }
}
