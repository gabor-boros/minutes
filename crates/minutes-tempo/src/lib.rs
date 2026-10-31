//! Tempo backend for Minutes.
//!
//! Implements [`WorklogSource`] and [`WorklogSink`] against the Tempo
//! Timesheets REST API (v4):
//! - fetch: `POST /rest/tempo-timesheets/4/worklogs/search`
//! - push: `POST /rest/tempo-timesheets/4/worklogs`
//!
//! Auth uses HTTP Basic Auth. Search and create payloads use local-calendar
//! `YYYY-MM-DD` dates.

use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Local, NaiveDate, TimeZone};
use minutes_client::{FetchOpts, HttpClient, HttpClientError, WorklogSink, WorklogSource};
use minutes_core::{ClientId, Entry, ProjectId, TaskId, WorkInterval};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Relative path used to search existing Tempo worklogs.
const PATH_WORKLOG_SEARCH: &str = "rest/tempo-timesheets/4/worklogs/search";

/// Relative path used to create new Tempo worklogs.
const PATH_WORKLOG_CREATE: &str = "rest/tempo-timesheets/4/worklogs";

/// Errors produced by the Tempo backend adapter.
#[derive(Error, Debug)]
pub enum TempoError {
    /// Underlying HTTP client failure (URL, transport, or non-success status).
    #[error(transparent)]
    Http(#[from] HttpClientError),
    /// Response body could not be decoded as JSON.
    #[error("failed to decode response: {0}")]
    Decode(#[from] reqwest::Error),
    /// Username, password, or worker was empty.
    #[error("username, password, and worker are required")]
    CredentialsRequired,
    /// A default request header value was invalid.
    #[error("invalid HTTP header value")]
    InvalidHeaderValue,
    /// [`FetchOpts`] has `start` after `end`.
    #[error("fetch window is inverted: start is after end")]
    InvertedFetchWindow,
    /// `timeSpentSeconds` was less than `billableSeconds`.
    #[error("time spent ({time_spent}) is less than billable ({billable})")]
    NegativeUnbillable {
        /// Total time spent in seconds.
        time_spent: u64,
        /// Billable time in seconds.
        billable: u64,
    },
    /// A date-only field could not be converted to a wall-clock instant.
    #[error("invalid start date: {0}")]
    InvalidStartDate(String),
}

/// Tempo client that fetches and pushes worklogs for a Jira/Tempo worker.
#[derive(Debug)]
pub struct TempoClient {
    /// Tempo worker key sent on search and create requests.
    worker: String,
    client: HttpClient,
}

impl TempoClient {
    /// Creates a client against a Tempo/Jira instance base URL.
    ///
    /// `base_url` should end with `/` so relative path joins keep the API prefix
    /// (for example `https://jira.example.com/`). Authentication uses HTTP
    /// Basic Auth with `username` and `password`. `worker` is the Tempo worker
    /// key sent on search and create requests.
    ///
    /// # Errors
    ///
    /// Returns [`TempoError::CredentialsRequired`] when `username`, `password`,
    /// or `worker` is empty, [`TempoError::InvalidHeaderValue`] when the Basic
    /// Auth header cannot be built, or an HTTP client construction error when
    /// the base URL/headers are invalid.
    pub fn new(
        base_url: &str,
        username: impl Into<String>,
        password: impl Into<String>,
        worker: impl Into<String>,
    ) -> Result<Self, TempoError> {
        let username = username.into();
        let password = password.into();
        let worker = worker.into();

        if username.is_empty() || password.is_empty() || worker.is_empty() {
            return Err(TempoError::CredentialsRequired);
        }

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, basic_auth_header(&username, &password)?);

        Ok(Self {
            worker,
            client: HttpClient::with_default_headers(base_url, headers)?,
        })
    }

    /// Maps a Tempo search hit onto one [`Entry`].
    ///
    /// Billable/unbillable seconds collapse into a single [`WorkInterval`]:
    /// elapsed is `timeSpentSeconds`, and the interval is billable when
    /// `billableSeconds > 0`.
    fn parse_entry(raw: FetchEntry) -> Result<Entry, TempoError> {
        let time_spent = u64::try_from(raw.time_spent_seconds.max(0)).unwrap_or(0);
        let billable = u64::try_from(raw.billable_seconds.max(0)).unwrap_or(0);
        if time_spent < billable {
            return Err(TempoError::NegativeUnbillable {
                time_spent,
                billable,
            });
        }

        let start = naive_date_to_system_time(raw.start_date)?;
        let interval = WorkInterval::new(start, Duration::from_secs(time_spent), billable > 0);

        let mut entry = Entry::new(
            ClientId::from(raw.issue.account_key.clone()),
            ProjectId::from(raw.issue.project_id),
            TaskId::from(raw.issue.id),
            interval,
        );

        if !raw.issue.account_key.is_empty() {
            entry = entry.with_client_name(raw.issue.account_key);
        }

        if !raw.issue.project_key.is_empty() {
            entry = entry.with_project_name(raw.issue.project_key);
        }

        if !raw.issue.key.is_empty() {
            entry = entry.with_task_name(raw.issue.key);
        }

        if !raw.issue.summary.is_empty() {
            entry = entry.with_summary(raw.issue.summary);
        }

        if !raw.comment.is_empty() {
            entry = entry.with_notes(raw.comment);
        }

        Ok(entry)
    }

    /// Builds a Tempo create payload from an [`Entry`].
    ///
    /// - `comment` comes from `summary`
    /// - `originTaskId` prefers `task_name` (Jira issue key), else the task id
    /// - `billableSeconds` is the full elapsed time when billable, otherwise `0`
    /// - `started` is the local `YYYY-MM-DD` calendar date of the interval start
    fn create_payload(&self, entry: &Entry) -> UploadEntry {
        let elapsed_secs = u32::try_from(entry.interval.elapsed().as_secs()).unwrap_or(u32::MAX);
        let billable_seconds = if entry.interval.is_billable() {
            elapsed_secs
        } else {
            0
        };

        // Prefer the issue key (task name); fall back to the numeric task id.
        let origin_task_id = entry
            .task_name
            .clone()
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| entry.task.to_string());

        UploadEntry {
            comment: entry.summary.clone().filter(|s| !s.is_empty()),
            include_non_working_days: true,
            origin_task_id: Some(origin_task_id).filter(|s| !s.is_empty()),
            started: Some(format_local_date(entry.interval.start())),
            billable_seconds: Some(billable_seconds),
            time_spent_seconds: Some(elapsed_secs),
            worker: Some(self.worker.clone()),
        }
    }
}

#[async_trait]
impl WorklogSource for TempoClient {
    type Error = TempoError;

    async fn fetch_entries(&self, opts: &FetchOpts) -> Result<Vec<Entry>, Self::Error> {
        if opts.is_inverted() {
            return Err(TempoError::InvertedFetchWindow);
        }

        let url = self.client.join_url(PATH_WORKLOG_SEARCH)?;
        let body = SearchParams {
            from: format_local_date(opts.start),
            to: format_local_date(opts.end),
            worker: self.worker.clone(),
        };

        let response = self.client.post_json(url, &body).await?;
        let fetched: Vec<FetchEntry> = response.json().await?;

        let mut entries = Vec::with_capacity(fetched.len());
        for raw in fetched {
            entries.push(Self::parse_entry(raw)?);
        }
        Ok(entries)
    }
}

#[async_trait]
impl WorklogSink for TempoClient {
    type Error = TempoError;

    async fn push_entries(&self, entries: &[Entry]) -> Result<(), Self::Error> {
        let url = self.client.join_url(PATH_WORKLOG_CREATE)?;

        for entry in entries {
            let payload = self.create_payload(entry);
            self.client.post_json(url.clone(), &payload).await?;
        }

        Ok(())
    }
}

/// Jira issue the time was logged against.
#[derive(Debug, Deserialize)]
struct Issue {
    #[serde(default)]
    id: i64,
    #[serde(default)]
    key: String,
    #[serde(rename = "accountKey", default)]
    account_key: String,
    #[serde(rename = "projectId", default)]
    project_id: i64,
    #[serde(rename = "projectKey", default)]
    project_key: String,
    #[serde(default)]
    summary: String,
}

/// Worklog entry returned by Tempo search.
///
/// `startDate` must be `YYYY-MM-DD` (required by Tempo).
#[derive(Debug, Deserialize)]
struct FetchEntry {
    #[serde(rename = "startDate", with = "naive_date_serde")]
    start_date: NaiveDate,
    #[serde(rename = "billableSeconds", default)]
    billable_seconds: i64,
    #[serde(rename = "timeSpentSeconds", default)]
    time_spent_seconds: i64,
    #[serde(default)]
    comment: String,
    issue: Issue,
}

/// Body for `POST .../worklogs/search`.
///
/// `from` and `to` must be `YYYY-MM-DD` (required by Tempo).
#[derive(Debug, Serialize)]
struct SearchParams {
    from: String,
    to: String,
    worker: String,
}

/// Body for `POST .../worklogs`.
///
/// `started` must be `YYYY-MM-DD` (required by Tempo).
#[derive(Debug, Serialize)]
struct UploadEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
    #[serde(rename = "includeNonWorkingDays")]
    include_non_working_days: bool,
    #[serde(rename = "originTaskId", skip_serializing_if = "Option::is_none")]
    origin_task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    started: Option<String>,
    #[serde(rename = "billableSeconds", skip_serializing_if = "Option::is_none")]
    billable_seconds: Option<u32>,
    #[serde(rename = "timeSpentSeconds", skip_serializing_if = "Option::is_none")]
    time_spent_seconds: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worker: Option<String>,
}

/// Builds an `Authorization: Basic …` header value.
fn basic_auth_header(username: &str, password: &str) -> Result<HeaderValue, TempoError> {
    let token = BASE64.encode(format!("{username}:{password}"));
    HeaderValue::from_str(&format!("Basic {token}")).map_err(|_| TempoError::InvalidHeaderValue)
}

/// Formats `time` as a local-calendar `YYYY-MM-DD` date.
fn format_local_date(time: SystemTime) -> String {
    DateTime::<Local>::from(time).format("%Y-%m-%d").to_string()
}

/// Converts a Tempo date-only value to local midnight as [`SystemTime`].
fn naive_date_to_system_time(date: NaiveDate) -> Result<SystemTime, TempoError> {
    let local = Local
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).expect("midnight"))
        .single()
        .ok_or_else(|| TempoError::InvalidStartDate(date.to_string()))?;
    Ok(SystemTime::from(local))
}

/// Serde helpers for Tempo's date-only `YYYY-MM-DD` JSON fields.
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
    use wiremock::matchers::{body_partial_json, header, method, path};
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

    fn expected_basic_auth() -> String {
        format!("Basic {}", BASE64.encode("steve:rogers"))
    }

    fn fetch_entry_json(billable_seconds: i64, time_spent_seconds: i64) -> serde_json::Value {
        json!({
            "id": 1,
            "startDate": "2021-10-02",
            "billableSeconds": billable_seconds,
            "timeSpentSeconds": time_spent_seconds,
            "comment": "Have a coffee with Tony",
            "workerKey": "steve-rogers",
            "issue": {
                "id": 789,
                "key": "MARVEL-101",
                "accountKey": "stark-industries",
                "projectId": 123,
                "projectKey": "MARVEL",
                "summary": "Meet with Iron Man"
            }
        })
    }

    #[test]
    fn new_rejects_empty_credentials() {
        let err = TempoClient::new("https://jira.example.com/", "", "pass", "worker")
            .expect_err("empty username");
        assert!(matches!(err, TempoError::CredentialsRequired));

        let err = TempoClient::new("https://jira.example.com/", "user", "", "worker")
            .expect_err("empty password");
        assert!(matches!(err, TempoError::CredentialsRequired));

        let err = TempoClient::new("https://jira.example.com/", "user", "pass", "")
            .expect_err("empty worker");
        assert!(matches!(err, TempoError::CredentialsRequired));
    }

    #[tokio::test]
    async fn fetch_entries_rejects_inverted_window() {
        let client = TempoClient::new(
            "https://jira.example.com/",
            "steve",
            "rogers",
            "steve-rogers",
        )
        .expect("client");
        let (start, end) = sample_window();
        let err = client
            .fetch_entries(&FetchOpts::new(end, start))
            .await
            .expect_err("inverted window");
        assert!(matches!(err, TempoError::InvertedFetchWindow));
    }

    #[tokio::test]
    async fn fetch_entries_maps_payload() {
        let server = MockServer::start().await;
        let (start, end) = sample_window();

        Mock::given(method("POST"))
            .and(path("/rest/tempo-timesheets/4/worklogs/search"))
            .and(header("authorization", expected_basic_auth().as_str()))
            .and(body_partial_json(json!({
                "from": "2021-10-02",
                "to": "2021-10-02",
                "worker": "steve-rogers"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                fetch_entry_json(3600, 3600),
                fetch_entry_json(0, 1800),
            ])))
            .mount(&server)
            .await;

        let client = TempoClient::new(
            &format!("{}/", server.uri()),
            "steve",
            "rogers",
            "steve-rogers",
        )
        .expect("client");

        let entries = client
            .fetch_entries(&FetchOpts::new(start, end))
            .await
            .expect("fetch");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].client, ClientId::from("stark-industries"));
        assert_eq!(entries[0].client_name.as_deref(), Some("stark-industries"));
        assert_eq!(entries[0].project, ProjectId::from(123));
        assert_eq!(entries[0].project_name.as_deref(), Some("MARVEL"));
        assert_eq!(entries[0].task, TaskId::from(789));
        assert_eq!(entries[0].task_name.as_deref(), Some("MARVEL-101"));
        assert_eq!(entries[0].summary.as_deref(), Some("Meet with Iron Man"));
        assert_eq!(entries[0].notes.as_deref(), Some("Have a coffee with Tony"));
        assert!(entries[0].interval.is_billable());
        assert_eq!(entries[0].interval.elapsed(), Duration::from_secs(3600));
        assert!(!entries[1].interval.is_billable());
        assert_eq!(entries[1].interval.elapsed(), Duration::from_mins(30));
    }

    #[tokio::test]
    async fn push_entries_posts_create_payload() {
        let server = MockServer::start().await;
        let (start, _) = sample_window();

        Mock::given(method("POST"))
            .and(path("/rest/tempo-timesheets/4/worklogs"))
            .and(header("authorization", expected_basic_auth().as_str()))
            .and(header("content-type", "application/json"))
            .and(body_partial_json(json!({
                "comment": "Meet with Iron Man",
                "includeNonWorkingDays": true,
                "originTaskId": "MARVEL-101",
                "started": "2021-10-02",
                "billableSeconds": 3600,
                "timeSpentSeconds": 3600,
                "worker": "steve-rogers"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&server)
            .await;

        let client = TempoClient::new(
            &format!("{}/", server.uri()),
            "steve",
            "rogers",
            "steve-rogers",
        )
        .expect("client");

        let entry = Entry::new(
            ClientId::from("stark-industries"),
            ProjectId::from(123),
            TaskId::from(789),
            WorkInterval::new(start, Duration::from_secs(3600), true),
        )
        .with_task_name("MARVEL-101")
        .with_summary("Meet with Iron Man");

        client.push_entries(&[entry]).await.expect("push");
    }

    #[tokio::test]
    async fn parse_entry_rejects_negative_unbillable() {
        let server = MockServer::start().await;
        let (start, end) = sample_window();

        Mock::given(method("POST"))
            .and(path("/rest/tempo-timesheets/4/worklogs/search"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!([fetch_entry_json(7200, 3600)])),
            )
            .mount(&server)
            .await;

        let client = TempoClient::new(
            &format!("{}/", server.uri()),
            "steve",
            "rogers",
            "steve-rogers",
        )
        .expect("client");

        let err = client
            .fetch_entries(&FetchOpts::new(start, end))
            .await
            .expect_err("negative unbillable");
        assert!(matches!(
            err,
            TempoError::NegativeUnbillable {
                time_spent: 3600,
                billable: 7200
            }
        ));
    }
}
