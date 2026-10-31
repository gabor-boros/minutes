//! Core worklog types for the Minutes time-tracking toolkit.
//!
//! This crate models time-tracking records (client, project, task, interval)
//! and provides utilities for aggregating them.
//!
//! Role-specific identifiers ([`ClientId`], [`ProjectId`], [`TaskId`]) brand a
//! shared [`Id`] representation so a client ID cannot be passed where a task
//! ID is required.
//!
//! # Examples
//!
//! ```
//! use minutes_core::{
//!     group_by_task, ClientId, Entry, ProjectId, TaskId, WorkInterval,
//! };
//! use std::time::{Duration, UNIX_EPOCH};
//!
//! let entries = [
//!     Entry::new(
//!         ClientId::from("acme"),
//!         ProjectId::from(1),
//!         TaskId::from("api"),
//!         WorkInterval::new(UNIX_EPOCH, Duration::from_secs(60), true),
//!     ),
//!     Entry::new(
//!         ClientId::from("acme"),
//!         ProjectId::from(1),
//!         TaskId::from("api"),
//!         WorkInterval::new(UNIX_EPOCH, Duration::from_secs(30), true),
//!     ),
//! ];
//!
//! let groups = group_by_task(&entries);
//! assert_eq!(groups[&TaskId::from("api")].len(), 2);
//! ```

mod entry;
mod filter;
mod id;
mod interval;

pub use entry::{Entry, Error, group_by_task};
pub use filter::{EntryFilter, TextMatcher};
pub use id::{Client, ClientId, Id, Project, ProjectId, Task, TaskId, TypedId};
pub use interval::WorkInterval;
