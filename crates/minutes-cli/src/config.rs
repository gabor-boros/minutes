//! Layered configuration: defaults → TOML → `MINUTES_*` env → CLI flags.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use chrono::{Local, NaiveDateTime, TimeZone};
use figment::providers::{Env, Format, Serialized, Toml};
use figment::{Figment, Provider};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::cli::Cli;
use crate::tracker::Tracker;

/// Default strftime pattern for `--start` / `--end` parsing.
pub const DEFAULT_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// Filename searched in home and OS config directories.
pub const CONFIG_FILE_NAME: &str = ".minutes.toml";

/// Fully resolved application settings after merging all layers.
///
/// TOML keys use kebab-case (for example `toggl-api-token`). Environment
/// variables use a `MINUTES_` prefix with underscores
/// (`MINUTES_TOGGL_API_TOKEN`).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "kebab-case")]
#[allow(clippy::struct_excessive_bools)] // flat config mirrors independent toggles
pub struct Config {
    /// Worklog source backend.
    pub source: Option<Tracker>,
    /// Worklog target backend.
    pub target: Option<Tracker>,

    /// Inclusive range start (parsed with [`Self::date_format`]).
    pub start: Option<String>,
    /// Inclusive range end (parsed with [`Self::date_format`]).
    pub end: Option<String>,
    /// [`chrono` strftime](https://docs.rs/chrono/latest/chrono/format/strftime/index.html) pattern.
    pub date_format: String,

    /// Fetch and print entries without uploading.
    pub dry_run: bool,
    /// Skip the interactive confirmation prompt.
    pub yes: bool,
    /// Treat every second as billable before upload.
    pub force_billed_duration: bool,
    /// Round each entry's elapsed time to the nearest minute before upload.
    pub round_to_closest_minute: bool,

    /// Regex applied to tags that should become tasks (where the source supports tags).
    pub tags_as_tasks_regex: Option<String>,
    /// Regex filter on client display name (or id).
    pub filter_client: Option<String>,
    /// Regex filter on project display name (or id).
    pub filter_project: Option<String>,

    /// Column names used to sort the overview table (`-col` = descending).
    pub table_sort_by: Vec<String>,
    /// Column names hidden from the overview table.
    pub table_hide_column: Vec<String>,

    // --- Clockify ---
    pub clockify_url: Option<String>,
    pub clockify_api_key: Option<String>,
    pub clockify_workspace: Option<String>,
    pub clockify_user: Option<String>,

    // --- Harvest ---
    pub harvest_access_token: Option<String>,
    pub harvest_account: Option<i64>,
    pub harvest_user: Option<String>,

    // --- Tempo ---
    pub tempo_url: Option<String>,
    pub tempo_username: Option<String>,
    pub tempo_password: Option<String>,
    pub tempo_worker: Option<String>,

    // --- Timewarrior ---
    pub timewarrior_command: Option<String>,
    pub timewarrior_arguments: Vec<String>,
    pub timewarrior_unbillable_tag: Option<String>,
    pub timewarrior_client_tag_regex: Option<String>,
    pub timewarrior_project_tag_regex: Option<String>,

    // --- Toggl ---
    pub toggl_url: Option<String>,
    pub toggl_api_token: Option<String>,
    pub toggl_workspace: Option<i64>,
    pub toggl_user: Option<String>,
}

impl Config {
    /// Built-in defaults before any file, env, or flag layer.
    #[must_use]
    pub fn defaults() -> Self {
        Self {
            date_format: DEFAULT_DATE_FORMAT.to_owned(),
            table_sort_by: vec![
                "start".to_owned(),
                "project".to_owned(),
                "task".to_owned(),
                "summary".to_owned(),
            ],
            timewarrior_command: Some("timew".to_owned()),
            timewarrior_unbillable_tag: Some("unbillable".to_owned()),
            ..Self::default()
        }
    }

    /// Loads configuration with precedence: defaults < file < env < CLI.
    ///
    /// # Errors
    ///
    /// Returns an error when a layer cannot be parsed or extraction fails.
    pub fn load(cli: &Cli) -> Result<Self> {
        let mut figment = Figment::new().merge(Serialized::defaults(Self::defaults()));

        if let Some(path) = cli.config.as_ref() {
            if !path.is_file() {
                bail!("config file not found: {}", path.display());
            }
            figment = figment.merge(Toml::file(path));
        } else if let Some(path) = find_config_file() {
            figment = figment.merge(Toml::file(path));
        }

        figment = figment
            .merge(minutes_env())
            .merge(Serialized::defaults(cli));

        figment
            .extract()
            .context("failed to load configuration (check TOML, env, and flags)")
    }

    /// Validates required fields and compiles filter regexes.
    ///
    /// # Errors
    ///
    /// Returns an error when source/target are missing or equal, when the
    /// fetch window is inverted, or when a regex is invalid.
    pub fn validate(&self) -> Result<()> {
        let source = self
            .source
            .context("source is required (set `source` in config or pass `--source`)")?;
        let target = self
            .target
            .context("target is required (set `target` in config or pass `--target`)")?;

        if source == target {
            bail!("source and target must differ (both are `{source}`)");
        }

        let (start, end) = self.parse_window()?;
        if start > end {
            bail!("start is after end; check `--start`, `--end`, and `date-format`");
        }

        if let Some(pattern) = &self.tags_as_tasks_regex {
            Regex::new(pattern)
                .with_context(|| format!("invalid tags-as-tasks-regex `{pattern}`"))?;
        }
        if let Some(pattern) = &self.filter_client {
            Regex::new(pattern).with_context(|| format!("invalid filter-client `{pattern}`"))?;
        }
        if let Some(pattern) = &self.filter_project {
            Regex::new(pattern).with_context(|| format!("invalid filter-project `{pattern}`"))?;
        }

        Ok(())
    }

    /// Parses the fetch window as local [`std::time::SystemTime`] values.
    ///
    /// Empty `start` defaults to today at local midnight. Empty `end` defaults
    /// to tomorrow at local midnight.
    ///
    /// # Errors
    ///
    /// Returns an error when a provided timestamp does not match `date_format`.
    pub fn parse_window(&self) -> Result<(std::time::SystemTime, std::time::SystemTime)> {
        let start = match &self.start {
            Some(raw) if !raw.is_empty() => parse_local_datetime(raw, &self.date_format)
                .with_context(|| format!("failed to parse start `{raw}`"))?,
            _ => today_midnight(),
        };
        let end = match &self.end {
            Some(raw) if !raw.is_empty() => parse_local_datetime(raw, &self.date_format)
                .with_context(|| format!("failed to parse end `{raw}`"))?,
            _ => tomorrow_midnight(),
        };
        Ok((start, end))
    }
}

/// Environment provider that maps `MINUTES_TOGGL_API_TOKEN` → `toggl-api-token`.
fn minutes_env() -> impl Provider {
    Env::prefixed("MINUTES_").map(|key| key.as_str().replace('_', "-").into())
}

/// Returns the first existing config path in the documented search order.
#[must_use]
pub fn find_config_file() -> Option<PathBuf> {
    config_search_paths()
        .into_iter()
        .find(|path| path.is_file())
}

/// Candidate paths for `.minutes.toml` (home, then OS config dir).
#[must_use]
pub fn config_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(base) = directories::BaseDirs::new() {
        paths.push(base.home_dir().join(CONFIG_FILE_NAME));
        paths.push(base.config_dir().join(CONFIG_FILE_NAME));
    }

    paths
}

fn parse_local_datetime(raw: &str, format: &str) -> Result<std::time::SystemTime> {
    let naive = NaiveDateTime::parse_from_str(raw, format)
        .or_else(|_| {
            // Allow date-only values when the format itself is date-only, and
            // also when users pass a bare date with a datetime format.
            chrono::NaiveDate::parse_from_str(raw, format)
                .or_else(|_| chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d"))
                .map(|date| date.and_hms_opt(0, 0, 0).expect("midnight"))
        })
        .with_context(|| format!("expected format `{format}`"))?;

    let local = Local
        .from_local_datetime(&naive)
        .single()
        .with_context(|| format!("ambiguous or invalid local time `{raw}`"))?;

    Ok(std::time::SystemTime::from(local))
}

fn today_midnight() -> std::time::SystemTime {
    let today = Local::now().date_naive();
    let midnight = today.and_hms_opt(0, 0, 0).expect("midnight");
    let local = Local
        .from_local_datetime(&midnight)
        .single()
        .expect("local midnight");
    std::time::SystemTime::from(local)
}

fn tomorrow_midnight() -> std::time::SystemTime {
    let tomorrow = Local::now().date_naive() + chrono::Duration::days(1);
    let midnight = tomorrow.and_hms_opt(0, 0, 0).expect("midnight");
    let local = Local
        .from_local_datetime(&midnight)
        .single()
        .expect("local midnight");
    std::time::SystemTime::from(local)
}

/// Ensures a URL ends with `/` so relative API joins keep the path prefix.
#[must_use]
pub fn ensure_trailing_slash(url: &str) -> String {
    if url.ends_with('/') {
        url.to_owned()
    } else {
        format!("{url}/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use figment::Jail;

    #[test]
    fn defaults_include_date_format_and_sort() {
        let config = Config::defaults();
        assert_eq!(config.date_format, DEFAULT_DATE_FORMAT);
        assert_eq!(config.table_sort_by[0], "start");
        assert!(!config.dry_run);
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn file_then_env_then_cli_precedence() {
        Jail::expect_with(|jail| {
            jail.create_file(
                CONFIG_FILE_NAME,
                r#"
source = "toggl"
target = "tempo"
dry-run = false
toggl-api-token = "from-file"
"#,
            )?;

            jail.set_env("MINUTES_DRY_RUN", "true");
            jail.set_env("MINUTES_TOGGL_API_TOKEN", "from-env");

            let cli = Cli {
                source: Some(Tracker::Clockify),
                ..Cli::empty_for_test()
            };

            let config: Config = Figment::new()
                .merge(Serialized::defaults(Config::defaults()))
                .merge(Toml::file(CONFIG_FILE_NAME))
                .merge(minutes_env())
                .merge(Serialized::defaults(&cli))
                .extract()?;

            assert_eq!(config.source, Some(Tracker::Clockify)); // CLI wins
            assert_eq!(config.target, Some(Tracker::Tempo)); // file
            assert!(config.dry_run); // env wins over file
            assert_eq!(config.toggl_api_token.as_deref(), Some("from-env"));
            Ok(())
        });
    }

    #[test]
    fn validate_rejects_same_source_and_target() {
        let mut config = Config::defaults();
        config.source = Some(Tracker::Tempo);
        config.target = Some(Tracker::Tempo);
        let err = config.validate().expect_err("same source/target");
        assert!(err.to_string().contains("must differ"));
    }

    #[test]
    fn ensure_trailing_slash_appends_when_missing() {
        assert_eq!(
            ensure_trailing_slash("https://jira.example.com"),
            "https://jira.example.com/"
        );
        assert_eq!(
            ensure_trailing_slash("https://jira.example.com/"),
            "https://jira.example.com/"
        );
    }

    #[test]
    fn parse_window_accepts_strftime_datetime() {
        let mut config = Config::defaults();
        config.start = Some("2026-02-03 00:00:00".to_owned());
        config.end = Some("2026-02-04 00:00:00".to_owned());
        let (start, end) = config.parse_window().expect("parse");
        assert!(start < end);
    }
}
