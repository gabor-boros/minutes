//! Kimai backend for Minutes.
//!
//! Implements [`WorklogSource`] and [`WorklogSink`] against the Kimai JSON API:
//! - fetch: `GET /api/timesheets`
//! - push: `POST /api/timesheets`
//!
//! Auth uses a Bearer API token. Fetch requests ask for the expanded
//! representation so customer, project, and activity names are available.
//! Push payloads send HTML5 local datetimes (without a timezone offset), which
//! is the only format Kimai guarantees to accept.
//!
//! Kimai links a timesheet to a numeric project and activity. When an entry
//! carries a non-numeric project or activity identifier, the sink resolves it
//! by name through the Kimai API.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use minutes_client::{FetchOpts, HttpClient, HttpClientError, WorklogSink, WorklogSource};
use minutes_core::{ClientId, Entry, Id, ProjectId, TaskId, WorkInterval};
use regex::Regex;
use reqwest::StatusCode;
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Relative path for listing and creating timesheets.
const PATH_TIMESHEETS: &str = "timesheets";

/// Relative path for resolving projects by name.
const PATH_PROJECTS: &str = "projects";

/// Relative path for resolving activities by name.
const PATH_ACTIVITIES: &str = "activities";

/// Media type Kimai should return.
const ACCEPT_JSON: &str = "application/json";

/// Page size for paginated fetches (Kimai's maximum is 500).
const PAGE_SIZE: usize = 500;

/// Hard cap on pagination pages to avoid unbounded request loops if the API
/// keeps returning full pages.
const MAX_FETCH_PAGES: u32 = 1_000;

/// HTML5 local datetime format accepted by Kimai for writes and filters.
const HTML5_LOCAL: &str = "%Y-%m-%dT%H:%M:%S";

/// Errors produced by the Kimai backend adapter.
#[derive(Error, Debug)]
pub enum KimaiError {
    /// Underlying HTTP client failure (URL, transport, or non-success status).
    #[error(transparent)]
    Http(#[from] HttpClientError),
    /// Response body could not be decoded as JSON.
    #[error("failed to decode response: {0}")]
    Decode(#[from] reqwest::Error),
    /// API token was empty.
    #[error("API token is required")]
    ApiTokenRequired,
    /// A default request header value was invalid.
    #[error("invalid HTTP header value")]
    InvalidHeaderValue,
    /// [`FetchOpts`] has `start` after `end`.
    #[error("fetch window is inverted: start is after end")]
    InvertedFetchWindow,
    /// Pagination exceeded the configured page cap.
    #[error("pagination exceeded {MAX_FETCH_PAGES} pages")]
    TooManyPages,
    /// A datetime string could not be parsed or formed an invalid interval.
    #[error("invalid interval: {0}")]
    InvalidInterval(String),
    /// No project was given for the entry.
    #[error("project id is required to push to Kimai")]
    ProjectIdRequired,
    /// No activity was given for the entry.
    #[error("activity id is required to push to Kimai")]
    ActivityIdRequired,
    /// A project name could not be resolved to an id.
    #[error("project not found: {0}")]
    ProjectNotFound(String),
    /// An activity name could not be resolved to an id.
    #[error("activity not found: {0}")]
    ActivityNotFound(String),
}

/// Kimai client that fetches and pushes timesheet records.
#[derive(Debug)]
pub struct KimaiClient {
    /// Kimai user ID used for filtering and on create. Defaults to the token user.
    user_id: Option<i64>,
    client: HttpClient,
    /// When set, tags whose names match are treated as tasks when fetching.
    tags_as_tasks: Option<Regex>,
    /// Cache of resolved project names to ids.
    project_ids: Mutex<HashMap<String, i64>>,
    /// Cache of resolved `(project_id, activity name)` pairs to activity ids.
    activity_ids: Mutex<HashMap<(i64, String), i64>>,
}

impl KimaiClient {
    /// Creates a client for a Kimai installation.
    ///
    /// `base_url` may be either the installation root (for example
    /// `https://kimai.example.com`) or its API root (`https://kimai.example.com/api/`).
    ///
    /// # Errors
    ///
    /// Returns [`KimaiError::ApiTokenRequired`] when `api_token` is empty,
    /// [`KimaiError::InvalidHeaderValue`] when the authorization header cannot
    /// be built, or an HTTP client construction error when the base URL is invalid.
    pub fn new(base_url: &str, api_token: impl Into<String>) -> Result<Self, KimaiError> {
        Self::with_base_url(base_url, api_token)
    }

    /// Creates a client against an arbitrary base URL (useful for tests).
    ///
    /// # Errors
    ///
    /// Returns [`KimaiError::ApiTokenRequired`] when `api_token` is empty,
    /// [`KimaiError::InvalidHeaderValue`] when the authorization header cannot
    /// be built, or an HTTP client construction error when the base URL is invalid.
    pub fn with_base_url(base_url: &str, api_token: impl Into<String>) -> Result<Self, KimaiError> {
        let api_token = api_token.into();
        if api_token.is_empty() {
            return Err(KimaiError::ApiTokenRequired);
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_token}"))
                .map_err(|_| KimaiError::InvalidHeaderValue)?,
        );
        headers.insert(ACCEPT, HeaderValue::from_static(ACCEPT_JSON));

        Ok(Self {
            user_id: None,
            client: HttpClient::with_default_headers(&normalize_base_url(base_url), headers)?,
            tags_as_tasks: None,
            project_ids: Mutex::new(HashMap::new()),
            activity_ids: Mutex::new(HashMap::new()),
        })
    }

    /// Filters fetches by user and creates entries for that user.
    #[must_use]
    pub fn with_user(mut self, user_id: i64) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Treats Kimai tags matching `regex` as tasks when fetching.
    ///
    /// Matching tags replace the entry task and split the duration evenly
    /// across matches. When the regex is set and tags are present but none
    /// match, the interval is dropped (same semantics as other backends).
    #[must_use]
    pub fn with_tags_as_tasks(mut self, regex: Regex) -> Self {
        self.tags_as_tasks = Some(regex);
        self
    }

    fn parse_entries(&self, fetched: Vec<FetchEntry>) -> Result<Vec<Entry>, KimaiError> {
        let mut entries = Vec::new();
        for raw in fetched {
            entries.extend(self.parse_entry(raw)?);
        }
        Ok(entries)
    }

    fn parse_entry(&self, raw: FetchEntry) -> Result<Vec<Entry>, KimaiError> {
        let start = parse_api_datetime(&raw.begin)?;

        let elapsed = match raw.end.as_deref() {
            Some(end) => {
                let end = parse_api_datetime(end)?;
                end.duration_since(start)
                    .map_err(|_| KimaiError::InvalidInterval("end is before start".to_owned()))?
            }
            None => match raw.duration {
                Some(seconds) if seconds > 0 => {
                    Duration::from_secs(u64::try_from(seconds).unwrap_or(0))
                }
                _ => return Ok(Vec::new()),
            },
        };

        let interval = WorkInterval::new(start, elapsed, raw.billable);

        let mut client_id = None;
        let mut client_name = None;
        let mut project_id = None;
        let mut project_name = None;
        let mut activity_id = None;
        let mut activity_name = None;

        match &raw.project {
            Some(ProjectRef::Id(id)) => project_id = Some(*id),
            Some(ProjectRef::Object(project)) => {
                project_id = Some(project.id);
                project_name.clone_from(&project.name);

                if let Some(customer) = &project.customer {
                    client_id = Some(customer.id());
                    client_name = customer.name().map(str::to_owned);
                }
            }
            None => {}
        }

        match &raw.activity {
            Some(ActivityRef::Id(id)) => activity_id = Some(*id),
            Some(ActivityRef::Object(activity)) => {
                activity_id = Some(activity.id);
                activity_name.clone_from(&activity.name);
            }
            None => {}
        }

        let client = match (client_id, &client_name) {
            (Some(id), _) => ClientId::from(id),
            (None, Some(name)) => ClientId::from(name.as_str()),
            (None, None) => ClientId::from(""),
        };

        let project = match project_id {
            Some(id) if id > 0 => ProjectId::from(id),
            _ => ProjectId::from(project_name.as_deref().unwrap_or("")),
        };

        let task = match activity_id {
            Some(id) if id > 0 => TaskId::from(id),
            _ => TaskId::from(activity_name.as_deref().unwrap_or("")),
        };

        let mut entry = Entry::new(client, project, task, interval);

        if let Some(name) = client_name.filter(|name| !name.is_empty()) {
            entry = entry.with_client_name(name);
        }
        if let Some(name) = project_name.filter(|name| !name.is_empty()) {
            entry = entry.with_project_name(name);
        }
        if let Some(name) = activity_name.filter(|name| !name.is_empty()) {
            entry = entry.with_task_name(name);
        }

        let description = raw
            .description
            .filter(|description| !description.is_empty());
        if let Some(description) = description {
            entry = entry
                .with_summary(description.clone())
                .with_notes(description);
        }

        if let Some(regex) = &self.tags_as_tasks
            && !raw.tags.is_empty()
        {
            return Ok(split_by_tags_as_tasks(&entry, regex, &raw.tags));
        }

        Ok(vec![entry])
    }

    async fn resolve_project_id(&self, entry: &Entry) -> Result<i64, KimaiError> {
        if let Some(id) = positive_id(entry.project.as_inner()) {
            return Ok(id);
        }

        let name = label(entry.project_name.as_deref(), &entry.project);
        if name.is_empty() {
            return Err(KimaiError::ProjectIdRequired);
        }

        let cache_key = format!(
            "{}\u{1f}{name}",
            entry.client_name.as_deref().unwrap_or_default()
        );
        if let Some(id) = self
            .project_ids
            .lock()
            .expect("project cache lock")
            .get(&cache_key)
            .copied()
        {
            return Ok(id);
        }

        let url = self.client.join_url_with_query(
            PATH_PROJECTS,
            [
                ("visible", "1".to_owned()),
                ("ignoreDates", "1".to_owned()),
                ("term", name.clone()),
            ],
        )?;
        let response = self.client.get(url).await?;
        let projects: Vec<ProjectSummary> = response.json().await?;

        let project = select_project(&projects, &name, entry)
            .ok_or_else(|| KimaiError::ProjectNotFound(name.clone()))?;

        self.project_ids
            .lock()
            .expect("project cache lock")
            .insert(cache_key, project.id);

        Ok(project.id)
    }

    async fn resolve_activity_id(&self, project_id: i64, entry: &Entry) -> Result<i64, KimaiError> {
        if let Some(id) = positive_id(entry.task.as_inner()) {
            return Ok(id);
        }

        let name = label(entry.task_name.as_deref(), &entry.task);
        if name.is_empty() {
            return Err(KimaiError::ActivityIdRequired);
        }

        let cache_key = (project_id, name.clone());
        if let Some(id) = self
            .activity_ids
            .lock()
            .expect("activity cache lock")
            .get(&cache_key)
            .copied()
        {
            return Ok(id);
        }

        let url = self.client.join_url_with_query(
            PATH_ACTIVITIES,
            [
                ("project", project_id.to_string()),
                ("visible", "1".to_owned()),
                ("term", name.clone()),
            ],
        )?;
        let response = self.client.get(url).await?;
        let activities: Vec<ActivitySummary> = response.json().await?;

        let activity = activities
            .iter()
            .find(|activity| {
                activity
                    .name
                    .as_deref()
                    .is_some_and(|activity_name| activity_name.eq_ignore_ascii_case(&name))
            })
            .ok_or_else(|| KimaiError::ActivityNotFound(name.clone()))?;

        self.activity_ids
            .lock()
            .expect("activity cache lock")
            .insert(cache_key, activity.id);

        Ok(activity.id)
    }

    fn create_payload(
        entry: &Entry,
        project_id: i64,
        activity_id: i64,
    ) -> Result<CreateTimesheet, KimaiError> {
        let end = entry
            .interval
            .end()
            .ok_or_else(|| KimaiError::InvalidInterval("interval end overflow".to_owned()))?;

        Ok(CreateTimesheet {
            begin: format_html5_local(entry.interval.start()),
            end: format_html5_local(end),
            project: project_id,
            activity: activity_id,
            description: entry
                .summary
                .clone()
                .filter(|summary| !summary.is_empty())
                .or_else(|| entry.notes.clone().filter(|notes| !notes.is_empty())),
        })
    }

    /// Pushes one entry and returns the created Kimai timesheet id.
    ///
    /// # Errors
    ///
    /// Returns [`KimaiError`] when resolution or the HTTP request fails.
    pub async fn push_entry_with_id(&self, entry: &Entry) -> Result<i64, KimaiError> {
        let project_id = self.resolve_project_id(entry).await?;
        let activity_id = self.resolve_activity_id(project_id, entry).await?;
        let payload = Self::create_payload(entry, project_id, activity_id)?;
        let url = self.client.join_url(PATH_TIMESHEETS)?;
        let response = self.client.post_json(url, &payload).await?;
        let created: CreatedTimesheet = response.json().await?;
        Ok(created.id)
    }
}

#[async_trait]
impl WorklogSource for KimaiClient {
    type Error = KimaiError;

    async fn fetch_entries(&self, opts: &FetchOpts) -> Result<Vec<Entry>, Self::Error> {
        if opts.is_inverted() {
            return Err(KimaiError::InvertedFetchWindow);
        }

        let mut entries = Vec::new();
        let mut page = 1_u32;

        loop {
            if page > MAX_FETCH_PAGES {
                return Err(KimaiError::TooManyPages);
            }

            let mut query = vec![
                ("begin", format_html5_local(opts.start)),
                ("end", format_html5_local(opts.end)),
                ("full", "1".to_owned()),
                ("page", page.to_string()),
                ("size", PAGE_SIZE.to_string()),
                ("orderBy", "begin".to_owned()),
                ("order", "ASC".to_owned()),
            ];
            if let Some(user_id) = self.user_id {
                query.push(("user", user_id.to_string()));
            }

            let url = self.client.join_url_with_query(PATH_TIMESHEETS, query)?;

            let response = match self.client.get(url).await {
                Ok(response) => response,
                Err(HttpClientError::UnexpectedStatus { status, .. })
                    if status == StatusCode::NOT_FOUND && page > 1 =>
                {
                    break;
                }
                Err(err) => return Err(err.into()),
            };

            let fetched: Vec<FetchEntry> = response.json().await?;
            let page_len = fetched.len();
            entries.extend(self.parse_entries(fetched)?);

            if page_len < PAGE_SIZE {
                break;
            }

            page += 1;
        }

        Ok(entries)
    }
}

#[async_trait]
impl WorklogSink for KimaiClient {
    type Error = KimaiError;

    async fn push_entries(&self, entries: &[Entry]) -> Result<(), Self::Error> {
        for entry in entries {
            self.push_entry_with_id(entry).await?;
        }

        Ok(())
    }
}

/// Timesheet row returned by the Kimai API.
#[derive(Debug, Deserialize)]
struct FetchEntry {
    begin: String,
    #[serde(default)]
    end: Option<String>,
    #[serde(default)]
    duration: Option<i64>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default = "default_billable")]
    billable: bool,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    project: Option<ProjectRef>,
    #[serde(default)]
    activity: Option<ActivityRef>,
}

/// Project reference that may be an id or an expanded object.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ProjectRef {
    Id(i64),
    Object(ProjectObject),
}

/// Expanded project returned when fetching with `full=1`.
#[derive(Debug, Deserialize)]
struct ProjectObject {
    id: i64,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    customer: Option<CustomerRef>,
}

/// Activity reference that may be an id or an expanded object.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ActivityRef {
    Id(i64),
    Object(ActivityObject),
}

/// Expanded activity returned when fetching with `full=1`.
#[derive(Debug, Deserialize)]
struct ActivityObject {
    id: i64,
    #[serde(default)]
    name: Option<String>,
}

/// Customer reference that may be an id or an expanded object.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CustomerRef {
    Id(i64),
    Object(CustomerObject),
}

/// Expanded customer nested under a project.
#[derive(Debug, Deserialize)]
struct CustomerObject {
    id: i64,
    #[serde(default)]
    name: Option<String>,
}

impl CustomerRef {
    fn id(&self) -> i64 {
        match self {
            Self::Id(id) => *id,
            Self::Object(customer) => customer.id,
        }
    }

    fn name(&self) -> Option<&str> {
        match self {
            Self::Id(_) => None,
            Self::Object(customer) => customer.name.as_deref().filter(|name| !name.is_empty()),
        }
    }
}

/// Project row returned by `GET /api/projects`.
#[derive(Debug, Deserialize)]
struct ProjectSummary {
    id: i64,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    customer: Option<CustomerRef>,
    #[serde(rename = "parentTitle", default)]
    parent_title: Option<String>,
}

impl ProjectSummary {
    fn name(&self) -> &str {
        self.name.as_deref().unwrap_or_default()
    }

    fn customer_id(&self) -> Option<i64> {
        self.customer.as_ref().map(CustomerRef::id)
    }

    fn customer_name(&self) -> Option<&str> {
        self.customer
            .as_ref()
            .and_then(CustomerRef::name)
            .or(self.parent_title.as_deref())
    }
}

/// Activity row returned by `GET /api/activities`.
#[derive(Debug, Deserialize)]
struct ActivitySummary {
    id: i64,
    #[serde(default)]
    name: Option<String>,
}

/// Response body returned by `POST /api/timesheets`.
#[derive(Debug, Deserialize)]
struct CreatedTimesheet {
    id: i64,
}

/// Body for `POST /api/timesheets`.
#[derive(Debug, Serialize)]
struct CreateTimesheet {
    begin: String,
    end: String,
    project: i64,
    activity: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

fn default_billable() -> bool {
    true
}

/// Ensures the base URL ends with `/api/`.
fn normalize_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/api") {
        format!("{trimmed}/")
    } else {
        format!("{trimmed}/api/")
    }
}

fn format_html5_local(time: SystemTime) -> String {
    DateTime::<Local>::from(time)
        .format(HTML5_LOCAL)
        .to_string()
}

fn parse_api_datetime(value: &str) -> Result<SystemTime, KimaiError> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Ok(parsed.into());
    }

    for format in [
        "%Y-%m-%dT%H:%M:%S%z",
        "%Y-%m-%dT%H:%M:%S%.f%z",
        "%Y-%m-%dT%H:%M:%S",
    ] {
        if let Ok(parsed) = DateTime::parse_from_str(value, format) {
            return Ok(parsed.into());
        }
    }

    if let Ok(naive) = NaiveDateTime::parse_from_str(value, HTML5_LOCAL)
        && let Some(local) = Local.from_local_datetime(&naive).single()
    {
        return Ok(local.into());
    }

    Err(KimaiError::InvalidInterval(format!(
        "cannot parse datetime '{value}'"
    )))
}

fn positive_id(id: &Id) -> Option<i64> {
    match id {
        Id::Integer(value) if *value > 0 => Some(*value),
        Id::String(value) => value.parse::<i64>().ok().filter(|value| *value > 0),
        Id::Integer(_) => None,
    }
}

fn label(name: Option<&str>, id: &impl std::fmt::Display) -> String {
    match name {
        Some(name) if !name.is_empty() => name.to_owned(),
        _ => id.to_string(),
    }
}

fn select_project<'a>(
    projects: &'a [ProjectSummary],
    name: &str,
    entry: &Entry,
) -> Option<&'a ProjectSummary> {
    let candidates: Vec<&ProjectSummary> = projects
        .iter()
        .filter(|project| project.name().eq_ignore_ascii_case(name))
        .collect();

    if let Some(client_id) = positive_id(entry.client.as_inner())
        && let Some(project) = candidates
            .iter()
            .find(|project| project.customer_id() == Some(client_id))
    {
        return Some(project);
    }

    if let Some(client_name) = entry.client_name.as_deref().filter(|name| !name.is_empty())
        && let Some(project) = candidates.iter().find(|project| {
            project
                .customer_name()
                .is_some_and(|name| name.eq_ignore_ascii_case(client_name))
        })
    {
        return Some(project);
    }

    candidates.first().copied()
}

/// Splits `entry` into one entry per tag whose name matches `regex`.
///
/// Duration is divided evenly across matching tags. Tags that do not match are
/// ignored; if none match, an empty list is returned.
fn split_by_tags_as_tasks(entry: &Entry, regex: &Regex, tags: &[String]) -> Vec<Entry> {
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
            if let Some(summary) = &entry.summary {
                split = split.with_summary(summary.clone());
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
    use serde_json::json;
    use wiremock::matchers::{body_partial_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_window() -> (SystemTime, SystemTime) {
        let start = Local
            .with_ymd_and_hms(2021, 10, 2, 0, 0, 0)
            .single()
            .expect("start");
        let end = Local
            .with_ymd_and_hms(2021, 10, 2, 23, 59, 59)
            .single()
            .expect("end");
        (SystemTime::from(start), SystemTime::from(end))
    }

    fn sample_entry(project: ProjectId, task: TaskId) -> Entry {
        let start = Local
            .with_ymd_and_hms(2021, 10, 2, 11, 0, 0)
            .single()
            .expect("start");
        Entry::new(
            ClientId::from("Stark Industries"),
            project,
            task,
            WorkInterval::new(start.into(), Duration::from_secs(3600), true),
        )
        .with_client_name("Stark Industries")
        .with_project_name("Avengers")
        .with_task_name("Design")
        .with_summary("Meet Iron Man")
    }

    #[test]
    fn new_rejects_empty_api_token() {
        let err = KimaiClient::new("https://kimai.example.com", "").expect_err("empty token");
        assert!(matches!(err, KimaiError::ApiTokenRequired));
    }

    #[test]
    fn normalize_base_url_appends_api_path() {
        assert_eq!(
            normalize_base_url("https://kimai.example.com"),
            "https://kimai.example.com/api/"
        );
        assert_eq!(
            normalize_base_url("https://kimai.example.com/"),
            "https://kimai.example.com/api/"
        );
        assert_eq!(
            normalize_base_url("https://kimai.example.com/api"),
            "https://kimai.example.com/api/"
        );
        assert_eq!(
            normalize_base_url("https://kimai.example.com/api/"),
            "https://kimai.example.com/api/"
        );
    }

    #[tokio::test]
    async fn fetch_entries_rejects_inverted_window() {
        let client =
            KimaiClient::with_base_url("https://kimai.example.com", "token").expect("client");
        let (start, end) = sample_window();
        let err = client
            .fetch_entries(&FetchOpts::new(end, start))
            .await
            .expect_err("inverted window");
        assert!(matches!(err, KimaiError::InvertedFetchWindow));
    }

    #[tokio::test]
    async fn fetch_entries_maps_expanded_payload() {
        let server = MockServer::start().await;
        let (start, end) = sample_window();

        Mock::given(method("GET"))
            .and(path("/api/timesheets"))
            .and(header("authorization", "Bearer t-o-k-e-n"))
            .and(query_param("full", "1"))
            .and(query_param("orderBy", "begin"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "id": 1,
                    "begin": "2021-10-02T09:00:00+0200",
                    "end": "2021-10-02T10:00:00+0200",
                    "duration": 3600,
                    "description": "Meet Iron Man",
                    "billable": true,
                    "tags": ["meeting"],
                    "project": {
                        "id": 5,
                        "name": "Avengers",
                        "customer": {"id": 3, "name": "Stark Industries"}
                    },
                    "activity": {"id": 9, "name": "Design"}
                }
            ])))
            .mount(&server)
            .await;

        let client = KimaiClient::with_base_url(&server.uri(), "t-o-k-e-n").expect("client");
        let entries = client
            .fetch_entries(&FetchOpts::new(start, end))
            .await
            .expect("fetch");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].client, ClientId::from(3));
        assert_eq!(entries[0].client_name.as_deref(), Some("Stark Industries"));
        assert_eq!(entries[0].project, ProjectId::from(5));
        assert_eq!(entries[0].project_name.as_deref(), Some("Avengers"));
        assert_eq!(entries[0].task, TaskId::from(9));
        assert_eq!(entries[0].task_name.as_deref(), Some("Design"));
        assert_eq!(entries[0].summary.as_deref(), Some("Meet Iron Man"));
        assert_eq!(entries[0].notes.as_deref(), Some("Meet Iron Man"));
        assert!(entries[0].interval.is_billable());
        assert_eq!(entries[0].interval.elapsed(), Duration::from_secs(3600));
    }

    #[tokio::test]
    async fn fetch_entries_tags_as_tasks_splits_matching_tags() {
        let server = MockServer::start().await;
        let (start, end) = sample_window();

        Mock::given(method("GET"))
            .and(path("/api/timesheets"))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "id": 1,
                    "begin": "2021-10-02T09:00:00+0200",
                    "end": "2021-10-02T10:00:00+0200",
                    "duration": 3600,
                    "description": "Pair work",
                    "billable": true,
                    "tags": ["Coffee", "TASK-1", "TASK-2"],
                    "project": {
                        "id": 5,
                        "name": "Avengers",
                        "customer": {"id": 3, "name": "Stark Industries"}
                    },
                    "activity": {"id": 9, "name": "Design"}
                }
            ])))
            .mount(&server)
            .await;

        let client = KimaiClient::with_base_url(&server.uri(), "t-o-k-e-n")
            .expect("client")
            .with_tags_as_tasks(Regex::new(r"^TASK-\d+$").expect("regex"));

        let entries = client
            .fetch_entries(&FetchOpts::new(start, end))
            .await
            .expect("fetch");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].task, TaskId::from("TASK-1"));
        assert_eq!(entries[1].task, TaskId::from("TASK-2"));
        assert_eq!(entries[0].interval.elapsed(), Duration::from_mins(30));
        assert_eq!(entries[1].interval.elapsed(), Duration::from_mins(30));
    }

    #[tokio::test]
    async fn push_entries_posts_numeric_ids() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/timesheets"))
            .and(header("authorization", "Bearer t-o-k-e-n"))
            .and(body_partial_json(json!({
                "begin": "2021-10-02T11:00:00",
                "end": "2021-10-02T12:00:00",
                "project": 5,
                "activity": 9,
                "description": "Meet Iron Man"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": 1})))
            .mount(&server)
            .await;

        let client = KimaiClient::with_base_url(&server.uri(), "t-o-k-e-n").expect("client");
        let entry = sample_entry(ProjectId::from(5), TaskId::from(9));

        client.push_entries(&[entry]).await.expect("push");
    }

    #[tokio::test]
    async fn push_entries_resolves_project_and_activity_by_name() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/projects"))
            .and(query_param("term", "Avengers"))
            .and(query_param("ignoreDates", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "id": 5,
                    "name": "Avengers",
                    "customer": 3,
                    "parentTitle": "Stark Industries"
                }
            ])))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/api/activities"))
            .and(query_param("project", "5"))
            .and(query_param("term", "Design"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"id": 9, "name": "Design"}
            ])))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/timesheets"))
            .and(body_partial_json(json!({
                "project": 5,
                "activity": 9,
                "description": "Meet Iron Man"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": 1})))
            .mount(&server)
            .await;

        let client = KimaiClient::with_base_url(&server.uri(), "t-o-k-e-n").expect("client");
        let entry = sample_entry(ProjectId::from("Avengers"), TaskId::from("Design"));

        client.push_entries(&[entry]).await.expect("push");
    }

    #[tokio::test]
    async fn push_entries_reports_missing_project() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/projects"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&server)
            .await;

        let client = KimaiClient::with_base_url(&server.uri(), "t-o-k-e-n").expect("client");
        let entry = sample_entry(ProjectId::from("Unknown"), TaskId::from("Design"));

        let err = client
            .push_entries(&[entry])
            .await
            .expect_err("missing project");
        assert!(matches!(err, KimaiError::ProjectNotFound(_)));
    }
}
