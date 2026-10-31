//! Toggl Track backend for Minutes.
//!
//! Implements [`WorklogSource`] and [`WorklogSink`] against Toggl Track:
//! - fetch: `GET /reports/api/v2/details`
//! - push: `POST /api/v9/workspaces/{workspace_id}/time_entries`
//!
//! Auth uses HTTP Basic Auth with a personal API token as the username and the
//! literal password `api_token`.

use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, SecondsFormat, Utc};
use minutes_client::{FetchOpts, HttpClient, HttpClientError, WorklogSink, WorklogSource};
use minutes_core::{ClientId, Entry, Id, ProjectId, TaskId, WorkInterval};
use regex::Regex;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Production Toggl Track API base URL (trailing slash required for correct joins).
const BASE_URL: &str = "https://api.track.toggl.com/";

/// Relative path for the detailed report used by fetch.
const PATH_REPORT_DETAILS: &str = "reports/api/v2/details";

/// User-Agent query value required by the reports API.
const USER_AGENT_VALUE: &str = "minutes";

/// Application name sent as `created_with` on create.
const CREATED_WITH: &str = "minutes";

/// Hard cap on pagination pages to avoid unbounded request loops if the API
/// keeps returning full pages.
const MAX_FETCH_PAGES: u32 = 1_000;

/// Errors produced by the Toggl Track backend adapter.
#[derive(Error, Debug)]
pub enum TogglError {
    /// Underlying HTTP client failure (URL, transport, or non-success status).
    #[error(transparent)]
    Http(#[from] HttpClientError),
    /// Response body could not be decoded as JSON.
    #[error("failed to decode response: {0}")]
    Decode(#[from] reqwest::Error),
    /// API token was empty.
    #[error("API token is required")]
    ApiTokenRequired,
    /// User ID was empty.
    #[error("user id is required")]
    UserIdRequired,
    /// A default request header value was invalid.
    #[error("invalid HTTP header value")]
    InvalidHeaderValue,
    /// [`FetchOpts`] has `start` after `end`.
    #[error("fetch window is inverted: start is after end")]
    InvertedFetchWindow,
    /// Pagination exceeded the configured page cap.
    #[error("pagination exceeded {MAX_FETCH_PAGES} pages")]
    TooManyPages,
    /// Report duration was negative.
    #[error("invalid duration milliseconds: {0}")]
    InvalidDuration(i64),
    /// Project ID could not be parsed as an integer.
    #[error("invalid project id: {0}")]
    InvalidProjectId(String),
    /// Task ID could not be parsed as an integer.
    #[error("invalid task id: {0}")]
    InvalidTaskId(String),
    /// User ID could not be parsed as an integer.
    #[error("invalid user id: {0}")]
    InvalidUserId(String),
}

/// Toggl Track client that fetches and pushes time entries for a workspace user.
#[derive(Debug)]
pub struct TogglClient {
    workspace_id: i64,
    /// Toggl user ID sent on report and create requests.
    user_id: String,
    client: HttpClient,
    /// When set, tags whose names match are treated as tasks when fetching.
    tags_as_tasks: Option<Regex>,
}

impl TogglClient {
    /// Creates a client against the production Toggl Track API.
    ///
    /// # Errors
    ///
    /// Returns [`TogglError::ApiTokenRequired`] when `api_token` is empty,
    /// [`TogglError::UserIdRequired`] when `user_id` is empty, or an HTTP
    /// client construction error when headers/base URL are invalid.
    pub fn new(
        workspace_id: i64,
        user_id: impl Into<String>,
        api_token: impl Into<String>,
    ) -> Result<Self, TogglError> {
        Self::with_base_url(BASE_URL, workspace_id, user_id, api_token)
    }

    /// Creates a client against an arbitrary base URL (useful for tests).
    ///
    /// `base_url` should end with `/` so relative path joins keep the API prefix.
    ///
    /// # Errors
    ///
    /// Returns [`TogglError::ApiTokenRequired`] when `api_token` is empty,
    /// [`TogglError::UserIdRequired`] when `user_id` is empty, or
    /// [`TogglError::InvalidHeaderValue`] when the Basic Auth header cannot be built.
    pub fn with_base_url(
        base_url: &str,
        workspace_id: i64,
        user_id: impl Into<String>,
        api_token: impl Into<String>,
    ) -> Result<Self, TogglError> {
        let api_token = api_token.into();
        if api_token.is_empty() {
            return Err(TogglError::ApiTokenRequired);
        }

        let user_id = user_id.into();
        if user_id.is_empty() {
            return Err(TogglError::UserIdRequired);
        }

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, basic_auth_header(&api_token)?);

        Ok(Self {
            workspace_id,
            user_id,
            client: HttpClient::with_default_headers(base_url, headers)?,
            tags_as_tasks: None,
        })
    }

    /// Treats Toggl tags matching `regex` as tasks when fetching.
    ///
    /// Matching tags replace the entry task and split the duration evenly
    /// across matches. Tags that do not match are ignored; if none match, an
    /// empty list is returned for that report row.
    #[must_use]
    pub fn with_tags_as_tasks(mut self, regex: Regex) -> Self {
        self.tags_as_tasks = Some(regex);
        self
    }

    /// Workspace-scoped create path used by push.
    fn create_time_entries_path(&self) -> String {
        format!("api/v9/workspaces/{}/time_entries", self.workspace_id)
    }

    async fn fetch_page(&self, opts: &FetchOpts, page: u32) -> Result<FetchResponse, TogglError> {
        let url = self.client.join_url_with_query(
            PATH_REPORT_DETAILS,
            [
                ("since", format_utc_date(opts.start)),
                ("until", format_utc_date(opts.end)),
                ("user_id", self.user_id.clone()),
                ("workspace_id", self.workspace_id.to_string()),
                ("user_agent", USER_AGENT_VALUE.to_owned()),
                ("page", page.to_string()),
            ],
        )?;

        let response = self.client.get(url).await?;
        Ok(response.json().await?)
    }

    fn parse_entries(&self, fetched: Vec<FetchEntry>) -> Result<Vec<Entry>, TogglError> {
        let mut entries = Vec::with_capacity(fetched.len());
        for raw in fetched {
            entries.extend(self.parse_entry(raw)?);
        }
        Ok(entries)
    }

    /// Maps a detailed-report row onto one or more [`Entry`] values.
    fn parse_entry(&self, raw: FetchEntry) -> Result<Vec<Entry>, TogglError> {
        if raw.dur < 0 {
            return Err(TogglError::InvalidDuration(raw.dur));
        }

        #[allow(clippy::cast_sign_loss)]
        let elapsed = Duration::from_millis(raw.dur as u64);
        let start = SystemTime::from(raw.start);
        let interval = WorkInterval::new(start, elapsed, raw.is_billable);

        let client_name = raw.client.unwrap_or_default();
        let project_name = raw.project.unwrap_or_default();
        let task_name = raw.task.unwrap_or_default();
        let task_id = raw.tid.unwrap_or(0);
        let description = raw.description.unwrap_or_default();
        let tags = raw.tags;

        let mut entry = Entry::new(
            ClientId::from(client_name.as_str()),
            ProjectId::from(raw.pid.unwrap_or(0)),
            TaskId::from(task_id),
            interval,
        );

        if !client_name.is_empty() {
            entry = entry.with_client_name(client_name);
        }
        if !project_name.is_empty() {
            entry = entry.with_project_name(project_name);
        }
        if !task_name.is_empty() {
            entry = entry.with_task_name(task_name);
        }
        if !description.is_empty() {
            entry = entry
                .with_summary(description.clone())
                .with_notes(description.clone());
        }

        if let Some(regex) = &self.tags_as_tasks
            && !tags.is_empty()
        {
            return Ok(split_by_tags_as_tasks(&entry, &description, regex, &tags));
        }

        Ok(vec![entry])
    }

    /// Builds an API v9 create payload from an [`Entry`].
    fn create_payload(&self, entry: &Entry) -> Result<CreateTimeEntry, TogglError> {
        let user_id = parse_i64(&self.user_id, TogglError::InvalidUserId)?;
        let project_id =
            optional_positive_id(entry.project.as_inner(), TogglError::InvalidProjectId)?;
        let task_id = optional_positive_id(entry.task.as_inner(), TogglError::InvalidTaskId)?;

        let duration_secs = i64::try_from(entry.interval.elapsed().as_secs()).unwrap_or(i64::MAX);

        Ok(CreateTimeEntry {
            workspace_id: self.workspace_id,
            created_with: CREATED_WITH.to_owned(),
            description: entry
                .summary
                .clone()
                .filter(|s| !s.is_empty())
                .or_else(|| entry.notes.clone().filter(|s| !s.is_empty())),
            start: format_rfc3339(entry.interval.start()),
            duration: duration_secs,
            billable: entry.interval.is_billable(),
            project_id,
            task_id,
            user_id,
        })
    }
}

#[async_trait]
impl WorklogSource for TogglClient {
    type Error = TogglError;

    async fn fetch_entries(&self, opts: &FetchOpts) -> Result<Vec<Entry>, Self::Error> {
        if opts.is_inverted() {
            return Err(TogglError::InvertedFetchWindow);
        }

        let mut entries = Vec::new();
        let mut page = 1_u32;

        loop {
            if page > MAX_FETCH_PAGES {
                return Err(TogglError::TooManyPages);
            }

            let response = self.fetch_page(opts, page).await?;
            if response.data.is_empty() {
                break;
            }

            let page_len = response.data.len();
            entries.extend(self.parse_entries(response.data)?);

            let per_page = if response.per_page > 0 {
                response.per_page
            } else {
                50
            };

            if page_len < per_page {
                break;
            }

            if response.total_count > 0 && entries.len() >= response.total_count {
                break;
            }

            page += 1;
        }

        Ok(entries)
    }
}

#[async_trait]
impl WorklogSink for TogglClient {
    type Error = TogglError;

    async fn push_entries(&self, entries: &[Entry]) -> Result<(), Self::Error> {
        let url = self.client.join_url(&self.create_time_entries_path())?;

        for entry in entries {
            let payload = self.create_payload(entry)?;
            self.client.post_json(url.clone(), &payload).await?;
        }

        Ok(())
    }
}

/// Detailed-report time entry row.
#[derive(Debug, Deserialize)]
struct FetchEntry {
    #[serde(default)]
    client: Option<String>,
    #[serde(default)]
    description: Option<String>,
    dur: i64,
    #[serde(rename = "is_billable", default)]
    is_billable: bool,
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    pid: Option<i64>,
    start: DateTime<Utc>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    task: Option<String>,
    #[serde(default)]
    tid: Option<i64>,
}

/// Body for paginated detailed report responses.
#[derive(Debug, Deserialize)]
struct FetchResponse {
    #[serde(default)]
    data: Vec<FetchEntry>,
    #[serde(rename = "per_page", default)]
    per_page: usize,
    #[serde(rename = "total_count", default)]
    total_count: usize,
}

/// Body for `POST .../time_entries`.
#[derive(Debug, Serialize)]
struct CreateTimeEntry {
    workspace_id: i64,
    created_with: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    start: String,
    duration: i64,
    billable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_id: Option<i64>,
    user_id: i64,
}

/// Builds an `Authorization: Basic …` header for `{api_token}:api_token`.
fn basic_auth_header(api_token: &str) -> Result<HeaderValue, TogglError> {
    let token = BASE64.encode(format!("{api_token}:api_token"));
    HeaderValue::from_str(&format!("Basic {token}")).map_err(|_| TogglError::InvalidHeaderValue)
}

fn format_utc_date(time: SystemTime) -> String {
    DateTime::<Utc>::from(time).format("%Y-%m-%d").to_string()
}

fn format_rfc3339(time: SystemTime) -> String {
    DateTime::<Utc>::from(time).to_rfc3339_opts(SecondsFormat::Secs, true)
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
    let tasks: Vec<&String> = tags
        .iter()
        .filter(|tag| regex.find(tag).is_some())
        .collect();

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

fn optional_positive_id(
    id: &Id,
    err: impl FnOnce(String) -> TogglError,
) -> Result<Option<i64>, TogglError> {
    let value = match id {
        Id::Integer(value) => *value,
        Id::String(value) if value.is_empty() || value == "0" => return Ok(None),
        Id::String(value) => parse_i64(value, err)?,
    };

    if value == 0 {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

fn parse_i64(value: &str, err: impl FnOnce(String) -> TogglError) -> Result<i64, TogglError> {
    value.parse().map_err(|_| err(value.to_owned()))
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

    fn expected_basic_auth() -> String {
        format!("Basic {}", BASE64.encode("t-o-k-e-n:api_token"))
    }

    fn fetch_entry_json(
        description: &str,
        dur_ms: i64,
        billable: bool,
        tags: &[&str],
    ) -> serde_json::Value {
        json!({
            "id": 1,
            "client": "Stark Industries",
            "description": description,
            "dur": dur_ms,
            "is_billable": billable,
            "project": "MARVEL",
            "pid": 123,
            "start": "2021-10-02T09:00:00Z",
            "end": "2021-10-02T10:00:00Z",
            "tags": tags,
            "task": "Graphic Design",
            "tid": 789
        })
    }

    #[test]
    fn new_rejects_empty_api_token() {
        let err = TogglClient::new(1, "42", "").expect_err("empty token");
        assert!(matches!(err, TogglError::ApiTokenRequired));
    }

    #[test]
    fn new_rejects_empty_user_id() {
        let err = TogglClient::new(1, "", "token").expect_err("empty user");
        assert!(matches!(err, TogglError::UserIdRequired));
    }

    #[tokio::test]
    async fn fetch_entries_rejects_inverted_window() {
        let client = TogglClient::with_base_url("https://api.example.com/", 1, "42", "token")
            .expect("client");
        let (start, end) = sample_window();
        let err = client
            .fetch_entries(&FetchOpts::new(end, start))
            .await
            .expect_err("inverted window");
        assert!(matches!(err, TogglError::InvertedFetchWindow));
    }

    #[tokio::test]
    async fn fetch_entries_maps_payload() {
        let server = MockServer::start().await;
        let (start, end) = sample_window();

        Mock::given(method("GET"))
            .and(path("/reports/api/v2/details"))
            .and(header("authorization", expected_basic_auth().as_str()))
            .and(query_param("since", "2021-10-02"))
            .and(query_param("until", "2021-10-02"))
            .and(query_param("user_id", "777"))
            .and(query_param("workspace_id", "1234"))
            .and(query_param("user_agent", "minutes"))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total_count": 2,
                "per_page": 50,
                "data": [
                    fetch_entry_json("Meet Iron Man", 3_600_000, true, &[]),
                    fetch_entry_json("Coffee break", 1_800_000, false, &[]),
                ]
            })))
            .mount(&server)
            .await;

        let client =
            TogglClient::with_base_url(&format!("{}/", server.uri()), 1234, "777", "t-o-k-e-n")
                .expect("client");

        let entries = client
            .fetch_entries(&FetchOpts::new(start, end))
            .await
            .expect("fetch");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].client, ClientId::from("Stark Industries"));
        assert_eq!(entries[0].client_name.as_deref(), Some("Stark Industries"));
        assert_eq!(entries[0].project, ProjectId::from(123));
        assert_eq!(entries[0].project_name.as_deref(), Some("MARVEL"));
        assert_eq!(entries[0].task, TaskId::from(789));
        assert_eq!(entries[0].task_name.as_deref(), Some("Graphic Design"));
        assert_eq!(entries[0].summary.as_deref(), Some("Meet Iron Man"));
        assert_eq!(entries[0].notes.as_deref(), Some("Meet Iron Man"));
        assert!(entries[0].interval.is_billable());
        assert_eq!(entries[0].interval.elapsed(), Duration::from_secs(3600));
        assert!(!entries[1].interval.is_billable());
        assert_eq!(entries[1].interval.elapsed(), Duration::from_mins(30));
    }

    #[tokio::test]
    async fn fetch_entries_tags_as_tasks_splits_matching_tags() {
        let server = MockServer::start().await;
        let (start, end) = sample_window();

        Mock::given(method("GET"))
            .and(path("/reports/api/v2/details"))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total_count": 1,
                "per_page": 50,
                "data": [fetch_entry_json(
                    "Have a coffee with Tony",
                    3_600_000,
                    true,
                    &["Coffee", "Meeting", "TASK-1234", "TASK-5678"],
                )]
            })))
            .mount(&server)
            .await;

        let client =
            TogglClient::with_base_url(&format!("{}/", server.uri()), 1234, "777", "t-o-k-e-n")
                .expect("client")
                .with_tags_as_tasks(Regex::new(r"^TASK-\d+$").expect("regex"));

        let entries = client
            .fetch_entries(&FetchOpts::new(start, end))
            .await
            .expect("fetch");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].task, TaskId::from("TASK-1234"));
        assert_eq!(entries[0].task_name.as_deref(), Some("TASK-1234"));
        assert_eq!(
            entries[0].summary.as_deref(),
            Some("Have a coffee with Tony")
        );
        assert_eq!(entries[1].task, TaskId::from("TASK-5678"));
        assert_eq!(entries[0].interval.elapsed(), Duration::from_mins(30));
        assert_eq!(entries[1].interval.elapsed(), Duration::from_mins(30));
    }

    #[tokio::test]
    async fn push_entries_posts_create_payload() {
        let server = MockServer::start().await;
        let (start, _) = sample_window();

        Mock::given(method("POST"))
            .and(path("/api/v9/workspaces/1234/time_entries"))
            .and(header("authorization", expected_basic_auth().as_str()))
            .and(header("content-type", "application/json"))
            .and(body_partial_json(json!({
                "workspace_id": 1234,
                "created_with": "minutes",
                "description": "Meet Iron Man",
                "start": "2021-10-02T00:00:00Z",
                "duration": 3600,
                "billable": true,
                "project_id": 123,
                "task_id": 789,
                "user_id": 777
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&server)
            .await;

        let client =
            TogglClient::with_base_url(&format!("{}/", server.uri()), 1234, "777", "t-o-k-e-n")
                .expect("client");

        let entry = Entry::new(
            ClientId::from("Stark Industries"),
            ProjectId::from(123),
            TaskId::from(789),
            WorkInterval::new(start, Duration::from_secs(3600), true),
        )
        .with_summary("Meet Iron Man")
        .with_notes("Discuss armor");

        client.push_entries(&[entry]).await.expect("push");
    }

    #[tokio::test]
    async fn push_entries_rejects_non_integer_project_id() {
        let client = TogglClient::with_base_url("https://api.example.com/", 1, "42", "token")
            .expect("client");
        let (start, _) = sample_window();
        let entry = Entry::new(
            ClientId::from("acme"),
            ProjectId::from("not-an-int"),
            TaskId::from(2),
            WorkInterval::new(start, Duration::from_secs(60), true),
        );

        let err = client
            .push_entries(&[entry])
            .await
            .expect_err("invalid project");
        assert!(matches!(err, TogglError::InvalidProjectId(_)));
    }
}
