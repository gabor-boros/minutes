//! Construct tracker clients from resolved [`Config`](crate::config::Config).

use anyhow::{Context, Result, bail};
use minutes_client::{FetchOpts, WorklogSink, WorklogSource};
use minutes_clockify::ClockifyClient;
use minutes_core::Entry;
use minutes_harvest::HarvestClient;
use minutes_tempo::TempoClient;
use minutes_timewarrior::TimewarriorClient;
use minutes_toggl::TogglClient;
use regex::Regex;

use crate::config::{Config, ensure_trailing_slash};
use crate::tracker::Tracker;

/// Default Clockify API base (includes `/api/v1/`).
const CLOCKIFY_DEFAULT_URL: &str = "https://api.clockify.me/api/v1/";

/// Default Toggl Track API base.
const TOGGL_DEFAULT_URL: &str = "https://api.track.toggl.com/";

/// A concrete backend that can fetch and/or push worklogs.
pub enum Backend {
    /// Clockify HTTP client.
    Clockify(ClockifyClient),
    /// Harvest HTTP client.
    Harvest(HarvestClient),
    /// Tempo HTTP client.
    Tempo(TempoClient),
    /// Local Timewarrior CLI client.
    Timewarrior(TimewarriorClient),
    /// Toggl Track HTTP client.
    Toggl(TogglClient),
}

impl Backend {
    /// Builds a client for `tracker` using credentials from `config`.
    ///
    /// # Errors
    ///
    /// Returns an error when required settings are missing or client
    /// construction fails.
    pub fn build(tracker: Tracker, config: &Config) -> Result<Self> {
        let tags_as_tasks = compile_tags_regex(config)?;

        match tracker {
            Tracker::Clockify => Ok(Self::Clockify(build_clockify(config, tags_as_tasks)?)),
            Tracker::Harvest => Ok(Self::Harvest(build_harvest(config)?)),
            Tracker::Tempo => Ok(Self::Tempo(build_tempo(config)?)),
            Tracker::Timewarrior => {
                Ok(Self::Timewarrior(build_timewarrior(config, tags_as_tasks)?))
            }
            Tracker::Toggl => Ok(Self::Toggl(build_toggl(config, tags_as_tasks)?)),
        }
    }

    /// Fetches worklog entries for `opts`.
    ///
    /// # Errors
    ///
    /// Returns the underlying backend error with context.
    pub async fn fetch_entries(&self, opts: &FetchOpts) -> Result<Vec<Entry>> {
        match self {
            Self::Clockify(client) => client
                .fetch_entries(opts)
                .await
                .context("clockify: failed to fetch entries"),
            Self::Harvest(client) => client
                .fetch_entries(opts)
                .await
                .context("harvest: failed to fetch entries"),
            Self::Tempo(client) => client
                .fetch_entries(opts)
                .await
                .context("tempo: failed to fetch entries"),
            Self::Timewarrior(client) => client
                .fetch_entries(opts)
                .await
                .context("timewarrior: failed to fetch entries"),
            Self::Toggl(client) => client
                .fetch_entries(opts)
                .await
                .context("toggl: failed to fetch entries"),
        }
    }

    /// Pushes a single worklog entry.
    ///
    /// # Errors
    ///
    /// Returns the underlying backend error with context.
    pub async fn push_entry(&self, entry: &Entry) -> Result<()> {
        let entries = std::slice::from_ref(entry);
        match self {
            Self::Clockify(client) => client
                .push_entries(entries)
                .await
                .context("clockify: failed to push entry"),
            Self::Harvest(client) => client
                .push_entries(entries)
                .await
                .context("harvest: failed to push entry"),
            Self::Tempo(client) => client
                .push_entries(entries)
                .await
                .context("tempo: failed to push entry"),
            Self::Timewarrior(client) => client
                .push_entries(entries)
                .await
                .context("timewarrior: failed to push entry"),
            Self::Toggl(client) => client
                .push_entries(entries)
                .await
                .context("toggl: failed to push entry"),
        }
    }
}

fn compile_tags_regex(config: &Config) -> Result<Option<Regex>> {
    config
        .tags_as_tasks_regex
        .as_deref()
        .filter(|pattern| !pattern.is_empty())
        .map(Regex::new)
        .transpose()
        .context("invalid tags-as-tasks-regex")
}

fn build_clockify(config: &Config, tags_as_tasks: Option<Regex>) -> Result<ClockifyClient> {
    let api_key = require(config.clockify_api_key.as_deref(), "clockify-api-key")?;
    let workspace = require(config.clockify_workspace.as_deref(), "clockify-workspace")?;
    let user = require(config.clockify_user.as_deref(), "clockify-user")?;
    let base = config
        .clockify_url
        .as_deref()
        .map_or_else(|| CLOCKIFY_DEFAULT_URL.to_owned(), ensure_trailing_slash);

    let mut client = ClockifyClient::with_base_url(&base, workspace, user, api_key)
        .context("failed to create Clockify client")?;
    if let Some(regex) = tags_as_tasks {
        client = client.with_tags_as_tasks(regex);
    }
    Ok(client)
}

fn build_harvest(config: &Config) -> Result<HarvestClient> {
    let token = require(
        config.harvest_access_token.as_deref(),
        "harvest-access-token",
    )?;
    let account = config
        .harvest_account
        .context("harvest-account is required when using Harvest")?;
    let user = require(config.harvest_user.as_deref(), "harvest-user")?;

    HarvestClient::new(account, user, token).context("failed to create Harvest client")
}

fn build_tempo(config: &Config) -> Result<TempoClient> {
    let url = require(config.tempo_url.as_deref(), "tempo-url")?;
    let username = require(config.tempo_username.as_deref(), "tempo-username")?;
    let password = require(config.tempo_password.as_deref(), "tempo-password")?;
    let worker = require(config.tempo_worker.as_deref(), "tempo-worker")?;

    TempoClient::new(&ensure_trailing_slash(url), username, password, worker)
        .context("failed to create Tempo client")
}

fn build_timewarrior(config: &Config, tags_as_tasks: Option<Regex>) -> Result<TimewarriorClient> {
    let program = config
        .timewarrior_command
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or("timew");

    let mut client =
        TimewarriorClient::with_program(program).context("failed to create Timewarrior client")?;

    if !config.timewarrior_arguments.is_empty() {
        client = client.with_default_args(config.timewarrior_arguments.clone());
    }

    if let Some(tag) = &config.timewarrior_unbillable_tag {
        client = client.with_unbillable_tag(tag.clone());
    }

    if let Some(pattern) = &config.timewarrior_client_tag_regex {
        client = client
            .with_client_tag_regex(pattern)
            .context("invalid timewarrior-client-tag-regex")?;
    } else {
        bail!(
            "timewarrior-client-tag-regex is required when using Timewarrior as source or target"
        );
    }

    if let Some(pattern) = &config.timewarrior_project_tag_regex {
        client = client
            .with_project_tag_regex(pattern)
            .context("invalid timewarrior-project-tag-regex")?;
    } else {
        bail!(
            "timewarrior-project-tag-regex is required when using Timewarrior as source or target"
        );
    }

    if let Some(regex) = tags_as_tasks {
        client = client.with_tags_as_tasks(regex);
    }

    Ok(client)
}

fn build_toggl(config: &Config, tags_as_tasks: Option<Regex>) -> Result<TogglClient> {
    let token = require(config.toggl_api_token.as_deref(), "toggl-api-token")?;
    let workspace = config
        .toggl_workspace
        .context("toggl-workspace is required when using Toggl")?;
    let user = require(config.toggl_user.as_deref(), "toggl-user")?;
    let base = config
        .toggl_url
        .as_deref()
        .map_or_else(|| TOGGL_DEFAULT_URL.to_owned(), ensure_trailing_slash);

    let mut client = TogglClient::with_base_url(&base, workspace, user, token)
        .context("failed to create Toggl client")?;
    if let Some(regex) = tags_as_tasks {
        client = client.with_tags_as_tasks(regex);
    }
    Ok(client)
}

fn require<'a>(value: Option<&'a str>, name: &str) -> Result<&'a str> {
    value
        .filter(|value| !value.is_empty())
        .with_context(|| format!("`{name}` is required for the selected tracker"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> Config {
        let mut config = Config::defaults();
        config.source = Some(Tracker::Toggl);
        config.target = Some(Tracker::Tempo);
        config
    }

    #[test]
    fn toggl_requires_token_workspace_and_user() {
        let Err(err) = Backend::build(Tracker::Toggl, &base_config()) else {
            panic!("expected missing credentials");
        };
        assert!(err.to_string().contains("toggl-api-token"));
    }

    #[test]
    fn tempo_requires_url_and_credentials() {
        let Err(err) = Backend::build(Tracker::Tempo, &base_config()) else {
            panic!("expected missing credentials");
        };
        assert!(err.to_string().contains("tempo-url"));
    }

    #[test]
    fn timewarrior_requires_tag_regexes() {
        let mut config = base_config();
        config.timewarrior_command = Some("timew".to_owned());
        let Err(err) = Backend::build(Tracker::Timewarrior, &config) else {
            panic!("expected missing regex");
        };
        assert!(err.to_string().contains("client-tag-regex"));
    }
}
