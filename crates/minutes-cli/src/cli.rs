//! Command-line argument definitions.
//!
//! Flags that are unset serialize as omitted fields so figment can keep values
//! from the config file and environment layers.

use std::path::PathBuf;

use clap::Parser;
use serde::Serialize;

use crate::tracker::Tracker;

/// Sync worklogs from a source time tracker to a target.
///
/// Configuration precedence: CLI flags > `MINUTES_*` environment variables >
/// `.minutes.toml` > built-in defaults.
#[derive(Debug, Parser, Serialize)]
#[command(
    name = "minutes",
    about = "Sync worklog entries between time trackers",
    long_about = "Fetches worklog entries from a source tracker, shows an overview \
table, then uploads complete entries to a target tracker.\n\n\
Config file: .minutes.toml (home directory or OS config dir). Override with \
--config, MINUTES_* environment variables, or flags. Date formats use chrono \
strftime (default %Y-%m-%d %H:%M:%S)."
)]
#[serde(rename_all = "kebab-case")]
#[allow(clippy::struct_excessive_bools)] // CLI mirrors independent config toggles
pub struct Cli {
    /// Path to a `.minutes.toml` file (skips the default search path).
    #[arg(long, value_name = "PATH")]
    #[serde(skip)]
    pub config: Option<PathBuf>,

    /// Source time tracker.
    #[arg(short, long, value_enum)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Tracker>,

    /// Target time tracker.
    #[arg(short, long, value_enum)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<Tracker>,

    /// Inclusive range start (must match `--date-format`).
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,

    /// Inclusive range end (must match `--date-format`).
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,

    /// Chrono strftime pattern for `--start` / `--end`.
    #[arg(long, value_name = "FORMAT")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_format: Option<String>,

    /// Fetch and print entries without uploading.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub dry_run: bool,

    /// Skip the interactive “Continue?” confirmation.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub yes: bool,

    /// Treat total time as billable before upload.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub force_billed_duration: bool,

    /// Round each entry's duration to the closest minute before upload.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub round_to_closest_minute: bool,

    /// Regex of tags that should become tasks (split time evenly across matches).
    #[arg(long, value_name = "REGEX")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_as_tasks_regex: Option<String>,

    /// Regex filter applied to client names after fetch.
    #[arg(long, value_name = "REGEX")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_client: Option<String>,

    /// Regex filter applied to project names after fetch.
    #[arg(long, value_name = "REGEX")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_project: Option<String>,

    /// Sort overview table by these columns (`-name` for descending).
    #[arg(long, value_name = "COLUMN", num_args = 1..)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub table_sort_by: Vec<String>,

    /// Hide these columns from the overview table.
    #[arg(long, value_name = "COLUMN", num_args = 1..)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub table_hide_column: Vec<String>,

    /// Clockify API base URL (default `https://api.clockify.me/api/v1/`).
    #[arg(long, value_name = "URL")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clockify_url: Option<String>,

    /// Clockify API key.
    #[arg(long, value_name = "KEY")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clockify_api_key: Option<String>,

    /// Clockify workspace ID.
    #[arg(long, value_name = "ID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clockify_workspace: Option<String>,

    /// Clockify user ID.
    #[arg(long, value_name = "ID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clockify_user: Option<String>,

    /// Harvest personal access token.
    #[arg(long, value_name = "TOKEN")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harvest_access_token: Option<String>,

    /// Harvest account ID.
    #[arg(long, value_name = "ID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harvest_account: Option<i64>,

    /// Harvest user ID.
    #[arg(long, value_name = "ID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harvest_user: Option<String>,

    /// Tempo base URL (for example `https://org.atlassian.net/`).
    #[arg(long, value_name = "URL")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tempo_url: Option<String>,

    /// Tempo username for Basic Auth.
    #[arg(long, value_name = "USER")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tempo_username: Option<String>,

    /// Tempo password or API token for Basic Auth.
    #[arg(long, value_name = "PASSWORD")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tempo_password: Option<String>,

    /// Tempo worker key used on search and create requests.
    #[arg(long, value_name = "WORKER")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tempo_worker: Option<String>,

    /// Timewarrior executable name or path (default `timew`).
    #[arg(long, value_name = "PROGRAM")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timewarrior_command: Option<String>,

    /// Extra arguments prepended to every Timewarrior invocation.
    #[arg(long, value_name = "ARG", num_args = 1..)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub timewarrior_arguments: Vec<String>,

    /// Tag that marks an interval as non-billable (default `unbillable`).
    #[arg(long, value_name = "TAG")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timewarrior_unbillable_tag: Option<String>,

    /// Regex matching Timewarrior tags treated as the client.
    #[arg(long, value_name = "REGEX")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timewarrior_client_tag_regex: Option<String>,

    /// Regex matching Timewarrior tags treated as the project.
    #[arg(long, value_name = "REGEX")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timewarrior_project_tag_regex: Option<String>,

    /// Toggl Track API base URL (default `https://api.track.toggl.com/`).
    #[arg(long, value_name = "URL")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toggl_url: Option<String>,

    /// Toggl API token.
    #[arg(long, value_name = "TOKEN")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toggl_api_token: Option<String>,

    /// Toggl workspace ID.
    #[arg(long, value_name = "ID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toggl_workspace: Option<i64>,

    /// Toggl user ID.
    #[arg(long, value_name = "ID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toggl_user: Option<String>,
}

impl Cli {
    /// Empty CLI overrides for unit tests (no flags set).
    #[cfg(test)]
    #[must_use]
    pub fn empty_for_test() -> Self {
        Self {
            config: None,
            source: None,
            target: None,
            start: None,
            end: None,
            date_format: None,
            dry_run: false,
            yes: false,
            force_billed_duration: false,
            round_to_closest_minute: false,
            tags_as_tasks_regex: None,
            filter_client: None,
            filter_project: None,
            table_sort_by: Vec::new(),
            table_hide_column: Vec::new(),
            clockify_url: None,
            clockify_api_key: None,
            clockify_workspace: None,
            clockify_user: None,
            harvest_access_token: None,
            harvest_account: None,
            harvest_user: None,
            tempo_url: None,
            tempo_username: None,
            tempo_password: None,
            tempo_worker: None,
            timewarrior_command: None,
            timewarrior_arguments: Vec::new(),
            timewarrior_unbillable_tag: None,
            timewarrior_client_tag_regex: None,
            timewarrior_project_tag_regex: None,
            toggl_url: None,
            toggl_api_token: None,
            toggl_workspace: None,
            toggl_user: None,
        }
    }
}
