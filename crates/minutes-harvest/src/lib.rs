//! Harvest backend for Minutes.
//!
//! Implements [`WorklogSource`] and [`WorklogSink`] against the Harvest
//! Time Entries API v2:
//! - fetch: `GET /v2/time_entries`
//! - push: `POST /v2/time_entries` (create via duration)
//!
//! Auth uses a Bearer personal access token plus the `Harvest-Account-Id`
//! header. Harvest requires a `User-Agent` on every request.
//!
//! Harvest does not accept a billable flag when creating entries; billable
//! status is derived from the project/task assignment on the server.

use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Timelike, Utc};
use minutes_client::{FetchOpts, HttpClient, HttpClientError, WorklogSink, WorklogSource};
use minutes_core::{ClientId, Entry, Id, ProjectId, TaskId, WorkInterval};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Production Harvest API base URL (trailing slash required for correct joins).
const BASE_URL: &str = "https://api.harvestapp.com/";

/// Relative path for listing and creating time entries.
const PATH_TIME_ENTRIES: &str = "v2/time_entries";

/// Header used by Harvest to select the account for a personal access token.
const ACCOUNT_ID_HEADER: &str = "harvest-account-id";

/// User-Agent identifying this client (required by Harvest).
const USER_AGENT_VALUE: &str = "minutes";

/// Page size for paginated fetches (under Harvest's 2000 maximum).
const DEFAULT_PAGE_SIZE: u32 = 100;

/// Hard cap on pagination pages to avoid unbounded request loops if the API
/// keeps returning full pages.
const MAX_FETCH_PAGES: u32 = 1_000;

/// Errors produced by the Harvest backend adapter.
#[derive(Error, Debug)]
pub enum HarvestError {
    /// Underlying HTTP client failure (URL, transport, or non-success status).
    #[error(transparent)]
    Http(#[from] HttpClientError),
    /// Response body could not be decoded as JSON.
    #[error("failed to decode response: {0}")]
    Decode(#[from] reqwest::Error),
    /// Access token was empty.
    #[error("access token is required")]
    AccessTokenRequired,
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
    /// `spent_date` could not be converted to a wall-clock instant.
    #[error("invalid spent date: {0}")]
    InvalidSpentDate(String),
    /// `hours` was negative or not finite.
    #[error("invalid hours value: {0}")]
    InvalidHours(f64),
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

/// Harvest client that fetches and pushes time entries for an account user.
#[derive(Debug)]
pub struct HarvestClient {
    /// Harvest user ID sent on list and create requests.
    user_id: String,
    client: HttpClient,
}

impl HarvestClient {
    /// Creates a client against the production Harvest API.
    ///
    /// # Errors
    ///
    /// Returns [`HarvestError::AccessTokenRequired`] when `access_token` is
    /// empty, [`HarvestError::UserIdRequired`] when `user_id` is empty, or an
    /// HTTP client construction error when headers/base URL are invalid.
    pub fn new(
        account_id: i64,
        user_id: impl Into<String>,
        access_token: impl Into<String>,
    ) -> Result<Self, HarvestError> {
        Self::with_base_url(BASE_URL, account_id, user_id, access_token)
    }

    /// Creates a client against an arbitrary base URL (useful for tests).
    ///
    /// `base_url` should end with `/` so relative path joins keep the API prefix.
    ///
    /// # Errors
    ///
    /// Returns [`HarvestError::AccessTokenRequired`] when `access_token` is
    /// empty, [`HarvestError::UserIdRequired`] when `user_id` is empty, or
    /// [`HarvestError::InvalidHeaderValue`] when a default header cannot be built.
    pub fn with_base_url(
        base_url: &str,
        account_id: i64,
        user_id: impl Into<String>,
        access_token: impl Into<String>,
    ) -> Result<Self, HarvestError> {
        let access_token = access_token.into();
        if access_token.is_empty() {
            return Err(HarvestError::AccessTokenRequired);
        }

        let user_id = user_id.into();
        if user_id.is_empty() {
            return Err(HarvestError::UserIdRequired);
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {access_token}"))
                .map_err(|_| HarvestError::InvalidHeaderValue)?,
        );
        headers.insert(
            HeaderName::from_static(ACCOUNT_ID_HEADER),
            HeaderValue::from_str(&account_id.to_string())
                .map_err(|_| HarvestError::InvalidHeaderValue)?,
        );
        headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));

        Ok(Self {
            user_id,
            client: HttpClient::with_default_headers(base_url, headers)?,
        })
    }

    async fn fetch_page(&self, opts: &FetchOpts, page: u32) -> Result<FetchResponse, HarvestError> {
        let url = self.client.join_url_with_query(
            PATH_TIME_ENTRIES,
            [
                ("from", format_utc_date(opts.start)),
                ("to", format_utc_date(opts.end)),
                ("user_id", self.user_id.clone()),
                ("is_running", "false".to_owned()),
                ("page", page.to_string()),
                ("per_page", DEFAULT_PAGE_SIZE.to_string()),
            ],
        )?;

        let response = self.client.get(url).await?;
        Ok(response.json().await?)
    }

    /// Maps a Harvest time entry onto one [`Entry`].
    ///
    /// Harvest `notes` are split on the first newline: the first line becomes
    /// `summary`, and any remainder becomes `notes`.
    ///
    /// Start time combines `spent_date` (calendar day) with the time-of-day
    /// from `created_at`, so an edited spent date still yields a usable
    /// wall-clock instant.
    fn parse_entry(raw: FetchEntry) -> Result<Entry, HarvestError> {
        if !raw.hours.is_finite() || raw.hours < 0.0 {
            return Err(HarvestError::InvalidHours(raw.hours));
        }

        let elapsed_secs = (raw.hours * 3600.0).round();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let elapsed = Duration::from_secs(elapsed_secs as u64);

        let start = combine_spent_date_and_created_at(raw.spent_date, raw.created_at)?;
        let interval = WorkInterval::new(start, elapsed, raw.billable);

        let mut entry = Entry::new(
            ClientId::from(raw.client.id),
            ProjectId::from(raw.project.id),
            TaskId::from(raw.task.id),
            interval,
        );

        if !raw.client.name.is_empty() {
            entry = entry.with_client_name(raw.client.name);
        }
        if !raw.project.name.is_empty() {
            entry = entry.with_project_name(raw.project.name);
        }
        if !raw.task.name.is_empty() {
            entry = entry.with_task_name(raw.task.name);
        }

        let (summary, notes) = split_harvest_notes(raw.notes.as_deref());
        if let Some(summary) = summary {
            entry = entry.with_summary(summary);
        }
        if let Some(notes) = notes {
            entry = entry.with_notes(notes);
        }

        Ok(entry)
    }

    /// Builds a duration-based create payload from an [`Entry`].
    ///
    /// Harvest `notes` are the inverse of the fetch split: summary and notes
    /// are joined with a newline when both are present.
    fn create_payload(&self, entry: &Entry) -> Result<CreateTimeEntry, HarvestError> {
        let user_id = parse_i64(&self.user_id, HarvestError::InvalidUserId)?;
        let project_id = id_as_i64(entry.project.as_inner(), HarvestError::InvalidProjectId)?;
        let task_id = id_as_i64(entry.task.as_inner(), HarvestError::InvalidTaskId)?;

        Ok(CreateTimeEntry {
            user_id,
            project_id,
            task_id,
            spent_date: format_utc_date(entry.interval.start()),
            hours: entry.interval.elapsed().as_secs_f64() / 3600.0,
            notes: join_harvest_notes(entry.summary.as_deref(), entry.notes.as_deref()),
        })
    }
}

#[async_trait]
impl WorklogSource for HarvestClient {
    type Error = HarvestError;

    async fn fetch_entries(&self, opts: &FetchOpts) -> Result<Vec<Entry>, Self::Error> {
        if opts.is_inverted() {
            return Err(HarvestError::InvertedFetchWindow);
        }

        let mut entries = Vec::new();
        let mut page = 1_u32;

        loop {
            if page > MAX_FETCH_PAGES {
                return Err(HarvestError::TooManyPages);
            }

            let response = self.fetch_page(opts, page).await?;
            if response.time_entries.is_empty() {
                break;
            }

            let page_len = response.time_entries.len();
            for raw in response.time_entries {
                entries.push(Self::parse_entry(raw)?);
            }

            let per_page = if response.per_page > 0 {
                response.per_page
            } else {
                DEFAULT_PAGE_SIZE as usize
            };

            if page_len < per_page {
                break;
            }

            if response.total_entries > 0 && entries.len() >= response.total_entries {
                break;
            }

            page += 1;
        }

        Ok(entries)
    }
}

#[async_trait]
impl WorklogSink for HarvestClient {
    type Error = HarvestError;

    async fn push_entries(&self, entries: &[Entry]) -> Result<(), Self::Error> {
        let url = self.client.join_url(PATH_TIME_ENTRIES)?;

        for entry in entries {
            let payload = self.create_payload(entry)?;
            self.client.post_json(url.clone(), &payload).await?;
        }

        Ok(())
    }
}

/// Nested Harvest `{ id, name }` object.
#[derive(Debug, Deserialize)]
struct IdName {
    id: i64,
    #[serde(default)]
    name: String,
}

/// Time entry returned by Harvest list.
#[derive(Debug, Deserialize)]
struct FetchEntry {
    client: IdName,
    project: IdName,
    task: IdName,
    #[serde(default)]
    notes: Option<String>,
    #[serde(rename = "spent_date", with = "naive_date_serde")]
    spent_date: NaiveDate,
    hours: f64,
    #[serde(rename = "created_at")]
    created_at: DateTime<Utc>,
    billable: bool,
}

/// Body for paginated `GET .../time_entries`.
#[derive(Debug, Deserialize)]
struct FetchResponse {
    #[serde(rename = "time_entries", default)]
    time_entries: Vec<FetchEntry>,
    #[serde(rename = "per_page", default)]
    per_page: usize,
    #[serde(rename = "total_entries", default)]
    total_entries: usize,
}

/// Body for `POST .../time_entries` (create via duration).
#[derive(Debug, Serialize)]
struct CreateTimeEntry {
    user_id: i64,
    project_id: i64,
    task_id: i64,
    spent_date: String,
    hours: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
}

/// Formats `time` as a UTC calendar `YYYY-MM-DD` date.
fn format_utc_date(time: SystemTime) -> String {
    DateTime::<Utc>::from(time).format("%Y-%m-%d").to_string()
}

/// Combines Harvest's date-only `spent_date` with the clock from `created_at`.
fn combine_spent_date_and_created_at(
    spent_date: NaiveDate,
    created_at: DateTime<Utc>,
) -> Result<SystemTime, HarvestError> {
    let combined = spent_date
        .and_hms_nano_opt(
            created_at.hour(),
            created_at.minute(),
            created_at.second(),
            created_at.nanosecond(),
        )
        .ok_or_else(|| HarvestError::InvalidSpentDate(spent_date.to_string()))?;
    Ok(SystemTime::from(
        DateTime::<Utc>::from_naive_utc_and_offset(combined, Utc),
    ))
}

/// Splits Harvest notes: first line → summary, remainder → notes.
fn split_harvest_notes(raw: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(text) = raw.map(str::trim_end).filter(|text| !text.is_empty()) else {
        return (None, None);
    };

    match text.split_once('\n') {
        Some((first, rest)) => {
            let summary = first.trim_end().to_owned();
            let notes = rest.trim_start_matches('\r').to_owned();
            let summary = (!summary.is_empty()).then_some(summary);
            let notes = (!notes.is_empty()).then_some(notes);
            (summary, notes)
        }
        None => (Some(text.to_owned()), None),
    }
}

/// Joins summary/notes back into a single Harvest notes field.
fn join_harvest_notes(summary: Option<&str>, notes: Option<&str>) -> Option<String> {
    match (
        summary.filter(|s| !s.is_empty()),
        notes.filter(|s| !s.is_empty()),
    ) {
        (Some(summary), Some(notes)) => Some(format!("{summary}\n{notes}")),
        (Some(summary), None) => Some(summary.to_owned()),
        (None, Some(notes)) => Some(notes.to_owned()),
        (None, None) => None,
    }
}

fn id_as_i64(id: &Id, err: impl FnOnce(String) -> HarvestError) -> Result<i64, HarvestError> {
    match id {
        Id::Integer(value) => Ok(*value),
        Id::String(value) => parse_i64(value, err),
    }
}

fn parse_i64(value: &str, err: impl FnOnce(String) -> HarvestError) -> Result<i64, HarvestError> {
    value.parse().map_err(|_| err(value.to_owned()))
}

/// Serde helpers for Harvest's date-only `YYYY-MM-DD` JSON fields.
mod naive_date_serde {
    use chrono::NaiveDate;
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(serde::de::Error::custom)
    }
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

    fn fetch_entry_json(notes: &str, hours: f64, billable: bool) -> serde_json::Value {
        json!({
            "id": 1,
            "spent_date": "2021-10-02",
            "client": { "id": 456, "name": "Stark Industries" },
            "project": { "id": 123, "name": "MARVEL" },
            "task": { "id": 789, "name": "Graphic Design" },
            "notes": notes,
            "hours": hours,
            "created_at": "2021-10-03T15:30:45Z",
            "billable": billable,
            "is_running": false
        })
    }

    #[test]
    fn new_rejects_empty_access_token() {
        let err = HarvestClient::new(1, "42", "").expect_err("empty token");
        assert!(matches!(err, HarvestError::AccessTokenRequired));
    }

    #[test]
    fn new_rejects_empty_user_id() {
        let err = HarvestClient::new(1, "", "token").expect_err("empty user");
        assert!(matches!(err, HarvestError::UserIdRequired));
    }

    #[test]
    fn split_harvest_notes_splits_on_first_newline() {
        let (summary, notes) = split_harvest_notes(Some("Meet Iron Man\nDiscuss armor upgrades"));
        assert_eq!(summary.as_deref(), Some("Meet Iron Man"));
        assert_eq!(notes.as_deref(), Some("Discuss armor upgrades"));
    }

    #[test]
    fn join_harvest_notes_prefers_summary_then_notes() {
        assert_eq!(
            join_harvest_notes(Some("Title"), Some("Details")).as_deref(),
            Some("Title\nDetails")
        );
        assert_eq!(
            join_harvest_notes(Some("Title"), None).as_deref(),
            Some("Title")
        );
        assert_eq!(
            join_harvest_notes(None, Some("Details")).as_deref(),
            Some("Details")
        );
        assert_eq!(join_harvest_notes(None, None), None);
    }

    #[tokio::test]
    async fn fetch_entries_rejects_inverted_window() {
        let client = HarvestClient::with_base_url("https://api.example.com/", 1, "42", "token")
            .expect("client");
        let (start, end) = sample_window();
        let err = client
            .fetch_entries(&FetchOpts::new(end, start))
            .await
            .expect_err("inverted window");
        assert!(matches!(err, HarvestError::InvertedFetchWindow));
    }

    #[tokio::test]
    async fn fetch_entries_maps_payload() {
        let server = MockServer::start().await;
        let (start, end) = sample_window();

        Mock::given(method("GET"))
            .and(path("/v2/time_entries"))
            .and(header("authorization", "Bearer t-o-k-e-n"))
            .and(header("harvest-account-id", "10254"))
            .and(header("user-agent", "minutes"))
            .and(query_param("from", "2021-10-02"))
            .and(query_param("to", "2021-10-02"))
            .and(query_param("user_id", "1782959"))
            .and(query_param("is_running", "false"))
            .and(query_param("page", "1"))
            .and(query_param("per_page", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "time_entries": [
                    fetch_entry_json("Meet Iron Man\nDiscuss armor upgrades", 2.0, true),
                    fetch_entry_json("Single line only", 0.5, false),
                ],
                "per_page": 100,
                "total_entries": 2
            })))
            .mount(&server)
            .await;

        let client = HarvestClient::with_base_url(
            &format!("{}/", server.uri()),
            10_254,
            "1782959",
            "t-o-k-e-n",
        )
        .expect("client");

        let entries = client
            .fetch_entries(&FetchOpts::new(start, end))
            .await
            .expect("fetch");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].client, ClientId::from(456));
        assert_eq!(entries[0].client_name.as_deref(), Some("Stark Industries"));
        assert_eq!(entries[0].project, ProjectId::from(123));
        assert_eq!(entries[0].project_name.as_deref(), Some("MARVEL"));
        assert_eq!(entries[0].task, TaskId::from(789));
        assert_eq!(entries[0].task_name.as_deref(), Some("Graphic Design"));
        assert_eq!(entries[0].summary.as_deref(), Some("Meet Iron Man"));
        assert_eq!(entries[0].notes.as_deref(), Some("Discuss armor upgrades"));
        assert!(entries[0].interval.is_billable());
        assert_eq!(entries[0].interval.elapsed(), Duration::from_secs(7200));

        let created_time = DateTime::parse_from_rfc3339("2021-10-03T15:30:45Z")
            .expect("created")
            .with_timezone(&Utc);
        let expected_start = DateTime::parse_from_rfc3339("2021-10-02T15:30:45Z")
            .expect("start")
            .with_timezone(&Utc);
        assert_eq!(
            DateTime::<Utc>::from(entries[0].interval.start()),
            expected_start
        );
        assert_eq!(created_time.time(), expected_start.time());

        assert_eq!(entries[1].summary.as_deref(), Some("Single line only"));
        assert_eq!(entries[1].notes, None);
        assert!(!entries[1].interval.is_billable());
        assert_eq!(entries[1].interval.elapsed(), Duration::from_mins(30));
    }

    #[tokio::test]
    async fn fetch_entries_paginates_until_short_page() {
        let server = MockServer::start().await;
        let (start, end) = sample_window();

        let full_page: Vec<_> = (0..100)
            .map(|i| {
                json!({
                    "id": i,
                    "spent_date": "2021-10-02",
                    "client": { "id": 1, "name": "Acme" },
                    "project": { "id": 2, "name": "Web" },
                    "task": { "id": 3, "name": "Dev" },
                    "notes": format!("Entry {i}"),
                    "hours": 1.0,
                    "created_at": "2021-10-02T12:00:00Z",
                    "billable": true,
                    "is_running": false
                })
            })
            .collect();

        Mock::given(method("GET"))
            .and(path("/v2/time_entries"))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "time_entries": full_page,
                "per_page": 100,
                "total_entries": 101
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v2/time_entries"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "time_entries": [fetch_entry_json("Last", 1.0, true)],
                "per_page": 100,
                "total_entries": 101
            })))
            .mount(&server)
            .await;

        let client = HarvestClient::with_base_url(
            &format!("{}/", server.uri()),
            10_254,
            "1782959",
            "t-o-k-e-n",
        )
        .expect("client");

        let entries = client
            .fetch_entries(&FetchOpts::new(start, end))
            .await
            .expect("fetch");
        assert_eq!(entries.len(), 101);
    }

    #[tokio::test]
    async fn push_entries_posts_create_payload() {
        let server = MockServer::start().await;
        let (start, _) = sample_window();

        Mock::given(method("POST"))
            .and(path("/v2/time_entries"))
            .and(header("authorization", "Bearer t-o-k-e-n"))
            .and(header("harvest-account-id", "10254"))
            .and(header("user-agent", "minutes"))
            .and(header("content-type", "application/json"))
            .and(body_partial_json(json!({
                "user_id": 1_782_959,
                "project_id": 123,
                "task_id": 789,
                "spent_date": "2021-10-02",
                "hours": 2.0,
                "notes": "Meet Iron Man\nDiscuss armor upgrades"
            })))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({})))
            .mount(&server)
            .await;

        let client = HarvestClient::with_base_url(
            &format!("{}/", server.uri()),
            10_254,
            "1782959",
            "t-o-k-e-n",
        )
        .expect("client");

        let entry = Entry::new(
            ClientId::from(456),
            ProjectId::from(123),
            TaskId::from(789),
            WorkInterval::new(start, Duration::from_secs(7200), true),
        )
        .with_summary("Meet Iron Man")
        .with_notes("Discuss armor upgrades");

        client.push_entries(&[entry]).await.expect("push");
    }

    #[tokio::test]
    async fn push_entries_rejects_non_integer_project_id() {
        let client = HarvestClient::with_base_url("https://api.example.com/", 1, "42", "token")
            .expect("client");
        let (start, _) = sample_window();
        let entry = Entry::new(
            ClientId::from(1),
            ProjectId::from("not-an-int"),
            TaskId::from(2),
            WorkInterval::new(start, Duration::from_secs(60), true),
        );

        let err = client
            .push_entries(&[entry])
            .await
            .expect_err("invalid project");
        assert!(matches!(err, HarvestError::InvalidProjectId(_)));
    }
}
