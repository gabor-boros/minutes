//! Clockify backend for Minutes.
//!
//! Implements [`WorklogSource`] and [`WorklogSink`] against the Clockify
//! Time Entries API:
//! - fetch: `GET /workspaces/{workspaceId}/user/{userId}/time-entries`
//! - push: `POST /workspaces/{workspaceId}/time-entries`

use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use minutes_client::{FetchOpts, HttpClient, HttpClientError, WorklogSink, WorklogSource};
use minutes_core::{ClientId, Entry, ProjectId, TaskId, WorkInterval};
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

/// Production Clockify API base URL (trailing slash required for correct joins).
const BASE_URL: &str = "https://api.clockify.me/api/v1/";

/// Header used by Clockify for API key authentication.
const API_KEY_HEADER: &str = "x-api-key";

/// Default page size for paginated fetches.
const DEFAULT_PAGE_SIZE: u32 = 50;

/// Hard cap on pagination pages to avoid unbounded request loops if the API
/// keeps returning full pages.
const MAX_FETCH_PAGES: u32 = 1_000;

/// Errors produced by the Clockify backend adapter.
#[derive(Error, Debug)]
pub enum ClockifyError {
    /// Underlying HTTP client failure (URL, transport, or non-success status).
    #[error(transparent)]
    Http(#[from] HttpClientError),
    /// Response body could not be decoded as JSON.
    #[error("failed to decode response: {0}")]
    Decode(#[from] reqwest::Error),
    /// API key was empty.
    #[error("API key is required")]
    ApiKeyRequired,
    /// An entry's interval end overflowed [`SystemTime`].
    #[error("entry interval end overflowed")]
    IntervalOverflow,
    /// A default request header value was invalid.
    #[error("invalid HTTP header value")]
    InvalidHeaderValue,
    /// [`FetchOpts`] has `start` after `end`.
    #[error("fetch window is inverted: start is after end")]
    InvertedFetchWindow,
    /// Pagination exceeded [`MAX_FETCH_PAGES`].
    #[error("pagination exceeded {MAX_FETCH_PAGES} pages")]
    TooManyPages,
}

/// Clockify client that fetches and pushes time entries for a workspace user.
#[derive(Debug)]
pub struct ClockifyClient {
    workspace_id: String,
    user_id: String,
    client: HttpClient,
    /// When set, tags whose names match are treated as tasks.
    tags_as_tasks: Option<Regex>,
}

impl ClockifyClient {
    /// Creates a client against the production Clockify API.
    ///
    /// # Errors
    ///
    /// Returns [`ClockifyError::ApiKeyRequired`] when `api_key` is empty, or an
    /// HTTP client construction error when the base URL/headers are invalid.
    pub fn new(
        workspace_id: impl Into<String>,
        user_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Result<Self, ClockifyError> {
        Self::with_base_url(BASE_URL, workspace_id, user_id, api_key)
    }

    /// Creates a client against an arbitrary base URL (useful for tests).
    ///
    /// `base_url` should end with `/` so relative path joins keep the API prefix.
    ///
    /// # Errors
    ///
    /// Returns [`ClockifyError::ApiKeyRequired`] when `api_key` is empty.
    pub fn with_base_url(
        base_url: &str,
        workspace_id: impl Into<String>,
        user_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Result<Self, ClockifyError> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(ClockifyError::ApiKeyRequired);
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static(API_KEY_HEADER),
            HeaderValue::from_str(&api_key).map_err(|_| ClockifyError::InvalidHeaderValue)?,
        );

        Ok(Self {
            workspace_id: workspace_id.into(),
            user_id: user_id.into(),
            client: HttpClient::with_default_headers(base_url, headers)?,
            tags_as_tasks: None,
        })
    }

    /// Treats Clockify tags matching `regex` as tasks when fetching.
    ///
    /// Matching tags replace the entry task and split the duration evenly
    /// across matches.
    #[must_use]
    pub fn with_tags_as_tasks(mut self, regex: Regex) -> Self {
        self.tags_as_tasks = Some(regex);
        self
    }

    /// User-scoped list path used by fetch.
    fn fetch_time_entries_path(&self) -> String {
        format!(
            "workspaces/{}/user/{}/time-entries",
            self.workspace_id, self.user_id
        )
    }

    /// Workspace-scoped create path used by push (`POST .../time-entries`).
    fn create_time_entries_path(&self) -> String {
        format!("workspaces/{}/time-entries", self.workspace_id)
    }

    async fn fetch_page(
        &self,
        opts: &FetchOpts,
        page: u32,
    ) -> Result<Vec<FetchEntry>, ClockifyError> {
        let url = self.client.join_url_with_query(
            &self.fetch_time_entries_path(),
            [
                ("start", format_rfc3339(opts.start)),
                ("end", format_rfc3339(opts.end)),
                ("hydrated", "true".to_owned()),
                ("in-progress", "false".to_owned()),
                ("page", page.to_string()),
                ("page-size", DEFAULT_PAGE_SIZE.to_string()),
            ],
        )?;

        let response = self.client.get(url).await?;
        Ok(response.json().await?)
    }

    fn parse_entries(&self, fetched: Vec<FetchEntry>) -> Result<Vec<Entry>, ClockifyError> {
        let mut entries = Vec::with_capacity(fetched.len());
        for raw in fetched {
            entries.extend(self.parse_entry(raw)?);
        }
        Ok(entries)
    }

    fn parse_entry(&self, raw: FetchEntry) -> Result<Vec<Entry>, ClockifyError> {
        let start = SystemTime::from(raw.time_interval.start);
        let elapsed = elapsed_between(raw.time_interval.start, raw.time_interval.end)?;
        let interval = WorkInterval::new(start, elapsed, raw.billable);

        let mut entry = Entry::new(
            ClientId::from(raw.project.client_id),
            ProjectId::from(raw.project.id),
            TaskId::from(raw.task.id.clone()),
            interval,
        );

        if !raw.project.client_name.is_empty() {
            entry = entry.with_client_name(raw.project.client_name);
        }

        if !raw.project.name.is_empty() {
            entry = entry.with_project_name(raw.project.name);
        }

        let task_name = raw.task.name.clone();
        let notes = raw.description.clone();

        if !task_name.is_empty() {
            entry = entry.with_task_name(task_name.clone());
        }

        // Prefer the task name as summary; fall back to notes when empty.
        if !task_name.is_empty() {
            entry = entry.with_summary(task_name);
        } else if !notes.is_empty() {
            entry = entry.with_summary(notes.clone());
        }

        if !notes.is_empty() {
            entry = entry.with_notes(notes);
        }

        if let Some(regex) = &self.tags_as_tasks
            && !raw.tags.is_empty()
        {
            return Ok(split_by_tags_as_tasks(
                &entry,
                &raw.description,
                regex,
                &raw.tags,
            ));
        }

        Ok(vec![entry])
    }

    fn create_payload(entry: &Entry) -> Result<CreateTimeEntry, ClockifyError> {
        let end = entry
            .interval
            .end()
            .ok_or(ClockifyError::IntervalOverflow)?;

        Ok(CreateTimeEntry {
            start: format_rfc3339(entry.interval.start()),
            end: format_rfc3339(end),
            billable: entry.interval.is_billable(),
            description: entry.notes.clone().or_else(|| entry.summary.clone()),
            project_id: entry.project.to_string(),
            task_id: nonempty_id(&entry.task),
            entry_type: "REGULAR".to_string(),
        })
    }
}

#[async_trait]
impl WorklogSource for ClockifyClient {
    type Error = ClockifyError;

    async fn fetch_entries(&self, opts: &FetchOpts) -> Result<Vec<Entry>, Self::Error> {
        if opts.is_inverted() {
            return Err(ClockifyError::InvertedFetchWindow);
        }

        let mut entries = Vec::new();
        let mut page = 1_u32;

        loop {
            if page > MAX_FETCH_PAGES {
                return Err(ClockifyError::TooManyPages);
            }

            let fetched = self.fetch_page(opts, page).await?;
            if fetched.is_empty() {
                break;
            }

            let page_len = fetched.len();
            entries.extend(self.parse_entries(fetched)?);

            if page_len < DEFAULT_PAGE_SIZE as usize {
                break;
            }

            page += 1;
        }

        Ok(entries)
    }
}

#[async_trait]
impl WorklogSink for ClockifyClient {
    type Error = ClockifyError;

    async fn push_entries(&self, entries: &[Entry]) -> Result<(), Self::Error> {
        let url = self.client.join_url(&self.create_time_entries_path())?;

        for entry in entries {
            let payload = Self::create_payload(entry)?;
            self.client.post_json(url.clone(), &payload).await?;
        }

        Ok(())
    }
}

#[derive(Debug, Default, Deserialize)]
struct Project {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(rename = "clientId", default)]
    client_id: String,
    #[serde(rename = "clientName", default)]
    client_name: String,
}

#[derive(Debug, Default, Deserialize)]
struct IdName {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct TimeInterval {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

/// Hydrated time entry returned by Clockify when `hydrated=true`.
#[derive(Debug, Deserialize)]
struct FetchEntry {
    #[serde(default)]
    description: String,
    billable: bool,
    /// Clockify may return JSON `null` when no project is set.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    project: Project,
    #[serde(rename = "timeInterval")]
    time_interval: TimeInterval,
    /// Clockify may return JSON `null` when no task is set.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    task: IdName,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    tags: Vec<IdName>,
}

/// Deserializes `null` or a missing value as [`Default::default`].
///
/// Clockify's hydrated time-entry payload uses JSON `null` for optional
/// nested objects (`task`, sometimes `project`) instead of omitting them.
/// Plain `#[serde(default)]` only covers missing fields, not explicit nulls.
fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + DeserializeOwned,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

/// Body for `POST .../time-entries`.
#[derive(Debug, Serialize)]
struct CreateTimeEntry {
    start: String,
    end: String,
    billable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(rename = "projectId")]
    project_id: String,
    #[serde(rename = "taskId", skip_serializing_if = "Option::is_none")]
    task_id: Option<String>,
    #[serde(rename = "type")]
    entry_type: String,
}

fn format_rfc3339(time: SystemTime) -> String {
    DateTime::<Utc>::from(time).to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn elapsed_between(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Duration, ClockifyError> {
    (end - start)
        .to_std()
        .map_err(|_| ClockifyError::IntervalOverflow)
}

fn nonempty_id(id: &TaskId) -> Option<String> {
    let value = id.to_string();
    if value.is_empty() || value == "0" {
        None
    } else {
        Some(value)
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
    tags: &[IdName],
) -> Vec<Entry> {
    let tasks: Vec<&IdName> = tags
        .iter()
        .filter(|tag| regex.find(&tag.name).is_some())
        .collect();

    if tasks.is_empty() {
        return Vec::new();
    }

    let parts = u32::try_from(tasks.len()).unwrap_or(1).max(1);
    let split_elapsed = entry.interval.elapsed() / parts;

    tasks
        .into_iter()
        .map(|task| {
            let mut split = Entry::new(
                entry.client.clone(),
                entry.project.clone(),
                TaskId::from(task.id.as_str()),
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

            if !task.name.is_empty() {
                split = split.with_task_name(task.name.clone());
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
    use serde_json::json;
    use wiremock::matchers::{body_partial_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_window() -> (SystemTime, SystemTime) {
        let start = DateTime::parse_from_rfc3339("2021-10-02T00:00:00Z")
            .expect("start")
            .with_timezone(&Utc);
        let end = DateTime::parse_from_rfc3339("2021-10-02T23:59:59Z")
            .expect("end")
            .with_timezone(&Utc);
        (SystemTime::from(start), SystemTime::from(end))
    }

    fn fetch_entry_json(
        billable: bool,
        description: &str,
        task: &serde_json::Value,
    ) -> serde_json::Value {
        json!({
            "description": description,
            "billable": billable,
            "project": {
                "id": "123",
                "name": "MARVEL-101",
                "clientId": "456",
                "clientName": "My Awesome Company"
            },
            "timeInterval": {
                "start": "2021-10-02T00:00:00Z",
                "end": "2021-10-02T23:59:59Z"
            },
            "task": task,
            "tags": [
                {"id": "1234", "name": "Coffee"},
                {"id": "5678", "name": "Meeting"},
                {"id": "9876", "name": "TASK-1234"}
            ]
        })
    }

    #[test]
    fn new_rejects_empty_api_key() {
        let err = ClockifyClient::new("ws", "user", "").expect_err("empty key");
        assert!(matches!(err, ClockifyError::ApiKeyRequired));
    }

    #[tokio::test]
    async fn fetch_entries_rejects_inverted_window() {
        let client =
            ClockifyClient::with_base_url("https://api.example.com/", "ws", "user", "token")
                .expect("client");
        let (start, end) = sample_window();
        let err = client
            .fetch_entries(&FetchOpts::new(end, start))
            .await
            .expect_err("inverted window");
        assert!(matches!(err, ClockifyError::InvertedFetchWindow));
    }

    #[test]
    fn fetch_entry_deserializes_null_task_project_and_tags() {
        let raw = json!({
            "description": "Ad-hoc work",
            "billable": true,
            "project": null,
            "timeInterval": {
                "start": "2021-10-02T00:00:00Z",
                "end": "2021-10-02T01:00:00Z"
            },
            "task": null,
            "tags": null
        });

        let entry: FetchEntry = serde_json::from_value(raw).expect("deserialize");
        assert_eq!(entry.task.id, "");
        assert_eq!(entry.project.id, "");
        assert_eq!(entry.tags.len(), 0);
        assert_eq!(entry.description, "Ad-hoc work");
    }

    #[tokio::test]
    async fn fetch_entries_maps_hydrated_payload() {
        let server = MockServer::start().await;
        let (start, end) = sample_window();

        Mock::given(method("GET"))
            .and(path(
                "/workspaces/marvel-studios/user/steve-rogers/time-entries",
            ))
            .and(header("x-api-key", "t-o-k-e-n"))
            .and(query_param("hydrated", "true"))
            .and(query_param("in-progress", "false"))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                fetch_entry_json(
                    true,
                    "Have a coffee with Tony",
                    &json!({
                        "id": "789",
                        "name": "Meet with Iron Man"
                    })
                ),
                fetch_entry_json(
                    false,
                    "Go back for my wallet",
                    &json!({
                        "id": "789",
                        "name": "Meet with Iron Man"
                    })
                ),
            ])))
            .mount(&server)
            .await;

        // Second page empty → stop pagination.
        Mock::given(method("GET"))
            .and(path(
                "/workspaces/marvel-studios/user/steve-rogers/time-entries",
            ))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&server)
            .await;

        let client = ClockifyClient::with_base_url(
            &format!("{}/", server.uri()),
            "marvel-studios",
            "steve-rogers",
            "t-o-k-e-n",
        )
        .expect("client");

        let entries = client
            .fetch_entries(&FetchOpts::new(start, end))
            .await
            .expect("fetch");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].client, ClientId::from("456"));
        assert_eq!(
            entries[0].client_name.as_deref(),
            Some("My Awesome Company")
        );
        assert_eq!(entries[0].project, ProjectId::from("123"));
        assert_eq!(entries[0].project_name.as_deref(), Some("MARVEL-101"));
        assert_eq!(entries[0].task, TaskId::from("789"));
        assert_eq!(entries[0].task_name.as_deref(), Some("Meet with Iron Man"));
        assert_eq!(entries[0].summary.as_deref(), Some("Meet with Iron Man"));
        assert_eq!(entries[0].notes.as_deref(), Some("Have a coffee with Tony"));
        assert!(entries[0].interval.is_billable());
        assert!(!entries[1].interval.is_billable());
        assert_eq!(
            entries[0].interval.elapsed(),
            Duration::from_secs(23 * 3600 + 59 * 60 + 59)
        );
    }

    #[tokio::test]
    async fn fetch_entries_tags_as_tasks_splits_matching_tags() {
        let server = MockServer::start().await;
        let (start, end) = sample_window();

        Mock::given(method("GET"))
            .and(path(
                "/workspaces/marvel-studios/user/steve-rogers/time-entries",
            ))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "description": "Have a coffee with Tony",
                    "billable": true,
                    "project": {
                        "id": "123",
                        "name": "MARVEL-101",
                        "clientId": "456",
                        "clientName": "My Awesome Company"
                    },
                    "timeInterval": {
                        "start": "2021-10-02T00:00:00Z",
                        "end": "2021-10-02T23:59:59Z"
                    },
                    "task": {"id": "", "name": ""},
                    "tags": [
                        {"id": "1234", "name": "Coffee"},
                        {"id": "5678", "name": "Meeting"},
                        {"id": "9876", "name": "TASK-1234"}
                    ]
                },
                {
                    "description": "Go back for my wallet",
                    "billable": false,
                    "project": {
                        "id": "123",
                        "name": "MARVEL-101",
                        "clientId": "456",
                        "clientName": "My Awesome Company"
                    },
                    "timeInterval": {
                        "start": "2021-10-02T00:00:00Z",
                        "end": "2021-10-02T23:59:59Z"
                    },
                    "task": {"id": "", "name": ""},
                    "tags": [
                        {"id": "1234", "name": "Coffee"},
                        {"id": "5678", "name": "Meeting"},
                        {"id": "9876", "name": "TASK-1234"},
                        {"id": "5432", "name": "TASK-5678"}
                    ]
                }
            ])))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&server)
            .await;

        let client = ClockifyClient::with_base_url(
            &format!("{}/", server.uri()),
            "marvel-studios",
            "steve-rogers",
            "t-o-k-e-n",
        )
        .expect("client")
        .with_tags_as_tasks(Regex::new(r"^TASK-\d+$").expect("regex"));

        let entries = client
            .fetch_entries(&FetchOpts::new(start, end))
            .await
            .expect("fetch");

        assert_eq!(entries.len(), 3);
        assert_eq!(
            entries[0].client_name.as_deref(),
            Some("My Awesome Company")
        );
        assert_eq!(entries[0].project_name.as_deref(), Some("MARVEL-101"));
        assert_eq!(entries[0].task, TaskId::from("9876"));
        assert_eq!(entries[0].task_name.as_deref(), Some("TASK-1234"));
        assert_eq!(
            entries[0].summary.as_deref(),
            Some("Have a coffee with Tony")
        );
        assert!(entries[0].interval.is_billable());

        assert_eq!(entries[1].task, TaskId::from("9876"));
        assert_eq!(entries[2].task, TaskId::from("5432"));
        assert!(!entries[1].interval.is_billable());
        let full = Duration::from_secs(23 * 3600 + 59 * 60 + 59);
        assert_eq!(entries[1].interval.elapsed(), full / 2);
        assert_eq!(entries[2].interval.elapsed(), full / 2);
    }

    #[tokio::test]
    async fn push_entries_posts_create_payload() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/workspaces/marvel-studios/time-entries"))
            .and(header("x-api-key", "t-o-k-e-n"))
            .and(header("content-type", "application/json"))
            .and(body_partial_json(json!({
                "billable": true,
                "description": "Have a coffee with Tony",
                "projectId": "123",
                "taskId": "789",
                "type": "REGULAR"
            })))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({})))
            .mount(&server)
            .await;

        let client = ClockifyClient::with_base_url(
            &format!("{}/", server.uri()),
            "marvel-studios",
            "steve-rogers",
            "t-o-k-e-n",
        )
        .expect("client");

        let (start, end) = sample_window();
        let entry = Entry::new(
            ClientId::from("456"),
            ProjectId::from("123"),
            TaskId::from("789"),
            WorkInterval::new(start, end.duration_since(start).expect("duration"), true),
        )
        .with_summary("Meet with Iron Man")
        .with_notes("Have a coffee with Tony");

        client.push_entries(&[entry]).await.expect("push");
    }

    #[tokio::test]
    async fn summary_falls_back_to_notes_when_task_name_empty() {
        let server = MockServer::start().await;
        let (start, end) = sample_window();

        Mock::given(method("GET"))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
                "description": "Have a coffee with Tony",
                "billable": true,
                "project": {
                    "id": "123",
                    "name": "MARVEL-101",
                    "clientId": "456",
                    "clientName": "My Awesome Company"
                },
                "timeInterval": {
                    "start": "2021-10-02T00:00:00Z",
                    "end": "2021-10-02T23:59:59Z"
                },
                "task": {"id": "789", "name": ""},
                "tags": []
            }])))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&server)
            .await;

        let client = ClockifyClient::with_base_url(
            &format!("{}/", server.uri()),
            "marvel-studios",
            "steve-rogers",
            "t-o-k-e-n",
        )
        .expect("client");

        let entries = client
            .fetch_entries(&FetchOpts::new(start, end))
            .await
            .expect("fetch");

        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].summary.as_deref(),
            Some("Have a coffee with Tony")
        );
    }
}
