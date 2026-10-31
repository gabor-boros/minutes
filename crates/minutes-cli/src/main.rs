//! Minutes command-line interface.
//!
//! Syncs worklog entries from a source time tracker to a target. Configuration
//! is layered as **flags > `MINUTES_*` env > `.minutes.toml` > defaults**.
//!
//! # Example
//!
//! ```text
//! minutes --source toggl --target tempo --dry-run
//! minutes --yes --start "2026-02-03 00:00:00" --end "2026-02-04 00:00:00"
//! ```
//!
//! See the crate `README.md` for configuration keys and safety notes.

mod backends;
mod cli;
mod config;
mod progress;
mod sync;
mod table;
mod terminal;
mod tracker;
mod transform;

use std::process::ExitCode;

use anyhow::Context;
use clap::Parser;

use crate::cli::Cli;
use crate::config::Config;

#[tokio::main]
async fn main() -> ExitCode {
    // Best-effort: a failure here must not block the CLI from running.
    let _ = terminal::install_interrupt_handler();

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            terminal::restore_cursor();
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = Config::load(&cli).context("configuration error")?;
    sync::run(config).await
}
