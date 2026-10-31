//! Overview table for fetched worklog entries.

use std::fmt::Write as _;
use std::time::Duration;

use chrono::{DateTime, Local};
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use minutes_core::Entry;

/// Column identifiers used by sort / hide configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Column {
    Task,
    Summary,
    Project,
    Client,
    Start,
    End,
    Billable,
    Unbillable,
}

impl Column {
    fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "task" => Some(Self::Task),
            "summary" => Some(Self::Summary),
            "project" => Some(Self::Project),
            "client" => Some(Self::Client),
            "start" => Some(Self::Start),
            "end" => Some(Self::End),
            // `duration` / `billable-duration` remain aliases for hide/sort compat.
            "billable" | "duration" | "billable-duration" => Some(Self::Billable),
            "unbillable" => Some(Self::Unbillable),
            _ => None,
        }
    }

    const fn header(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Summary => "summary",
            Self::Project => "project",
            Self::Client => "client",
            Self::Start => "start",
            Self::End => "end",
            Self::Billable => "billable",
            Self::Unbillable => "unbillable",
        }
    }
}

const DEFAULT_COLUMNS: [Column; 8] = [
    Column::Task,
    Column::Summary,
    Column::Project,
    Column::Client,
    Column::Start,
    Column::End,
    Column::Billable,
    Column::Unbillable,
];

/// Floor for dynamic layout so narrow TTYs (and `cargo test` under a small
/// terminal) do not truncate summary/task cells into unreadable stubs.
const MIN_OVERVIEW_WIDTH: u16 = 120;

/// Renders an overview table for `complete` and `incomplete` entries.
///
/// Incomplete rows are shown after complete ones and tinted when color is supported.
/// A final footer row sums billable and unbillable durations across all rows.
#[must_use]
pub fn render_overview(
    complete: &[Entry],
    incomplete: &[Entry],
    start_label: &str,
    end_label: &str,
    sort_by: &[String],
    hide: &[String],
) -> String {
    let hidden: Vec<Column> = hide.iter().filter_map(|name| Column::parse(name)).collect();
    let columns: Vec<Column> = DEFAULT_COLUMNS
        .into_iter()
        .filter(|column| !hidden.contains(column))
        .collect();

    let mut rows: Vec<Row> = complete
        .iter()
        .map(|entry| Row {
            entry: entry.clone(),
            complete: true,
        })
        .chain(incomplete.iter().map(|entry| Row {
            entry: entry.clone(),
            complete: false,
        }))
        .collect();

    sort_rows(&mut rows, sort_by);

    let mut table = Table::new();
    table
        .load_preset(comfy_table::presets::ASCII_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic);
    let width = table
        .width()
        .unwrap_or(MIN_OVERVIEW_WIDTH)
        .max(MIN_OVERVIEW_WIDTH);
    table
        .set_width(width)
        .set_header(columns.iter().map(|column| Cell::new(column.header())));
    for row in &rows {
        let cells = columns.iter().map(|column| {
            let text = cell_value(&row.entry, *column);
            let mut cell = Cell::new(text);
            if !row.complete {
                cell = cell.fg(Color::Yellow);
            }
            cell
        });
        table.add_row(cells);
    }

    let (total_billable, total_unbillable) = duration_totals(&rows);
    table.add_row(columns.iter().map(|column| {
        let mut cell = Cell::new(footer_value(*column, total_billable, total_unbillable))
            .add_attribute(Attribute::Bold);
        match column {
            Column::Billable => cell = cell.fg(Color::Green),
            Column::Unbillable => cell = cell.fg(Color::Yellow),
            _ => {}
        }
        cell
    }));

    let mut output = format!("Worklog entries ({start_label} - {end_label})\n\n{table}\n");
    let _ = writeln!(
        output,
        "Complete: {}  |  Incomplete (not uploadable): {}\n",
        complete.len(),
        incomplete.len()
    );
    if !incomplete.is_empty() {
        output.push_str(
            "Incomplete entries are missing client/project/task IDs or have zero duration.\n",
        );
    }
    output.push_str(
        "NO WARRANTY: review entries carefully before uploading; re-runs may create duplicates.\n\n",
    );
    output
}

#[derive(Clone)]
struct Row {
    entry: Entry,
    complete: bool,
}

fn duration_totals(rows: &[Row]) -> (Duration, Duration) {
    let mut billable = Duration::ZERO;
    let mut unbillable = Duration::ZERO;
    for row in rows {
        let elapsed = row.entry.interval.elapsed();
        if row.entry.interval.is_billable() {
            billable += elapsed;
        } else {
            unbillable += elapsed;
        }
    }
    (billable, unbillable)
}

fn footer_value(column: Column, billable: Duration, unbillable: Duration) -> String {
    match column {
        Column::End => "total time spent".to_owned(),
        Column::Billable => format_duration(billable),
        Column::Unbillable => format_duration(unbillable),
        _ => String::new(),
    }
}

fn sort_rows(rows: &mut [Row], sort_by: &[String]) {
    if sort_by.is_empty() {
        return;
    }

    rows.sort_by(|left, right| {
        for spec in sort_by {
            let (descending, name) = if let Some(rest) = spec.strip_prefix('-') {
                (true, rest)
            } else {
                (false, spec.as_str())
            };
            let Some(column) = Column::parse(name) else {
                continue;
            };
            let ordering = cell_value(&left.entry, column).cmp(&cell_value(&right.entry, column));
            let ordering = if descending {
                ordering.reverse()
            } else {
                ordering
            };
            if ordering != std::cmp::Ordering::Equal {
                return ordering;
            }
        }
        std::cmp::Ordering::Equal
    });
}

fn cell_value(entry: &Entry, column: Column) -> String {
    match column {
        Column::Task => display_or_id(entry.task_name.as_deref(), &entry.task),
        Column::Summary => entry.summary.clone().unwrap_or_default(),
        Column::Project => display_or_id(entry.project_name.as_deref(), &entry.project),
        Column::Client => display_or_id(entry.client_name.as_deref(), &entry.client),
        Column::Start => format_local(entry.interval.start()),
        Column::End => entry
            .interval
            .end()
            .map_or_else(|| "-".to_owned(), format_local),
        Column::Billable => {
            if entry.interval.is_billable() {
                format_duration(entry.interval.elapsed())
            } else {
                format_duration(Duration::ZERO)
            }
        }
        Column::Unbillable => {
            if entry.interval.is_billable() {
                format_duration(Duration::ZERO)
            } else {
                format_duration(entry.interval.elapsed())
            }
        }
    }
}

fn display_or_id(name: Option<&str>, id: &impl std::fmt::Display) -> String {
    match name {
        Some(name) if !name.is_empty() => name.to_owned(),
        _ => id.to_string(),
    }
}

fn format_local(time: std::time::SystemTime) -> String {
    DateTime::<Local>::from(time)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    format!(
        "{:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use minutes_core::{ClientId, ProjectId, TaskId, WorkInterval};
    use std::time::UNIX_EPOCH;

    fn sample(summary: &str) -> Entry {
        Entry::new(
            ClientId::from("c"),
            ProjectId::from("p"),
            TaskId::from("t"),
            WorkInterval::new(UNIX_EPOCH, Duration::from_secs(90), true),
        )
        .with_summary(summary)
        .with_client_name("Client")
        .with_project_name("Project")
        .with_task_name("Task")
    }

    fn sample_unbillable(summary: &str, secs: u64) -> Entry {
        Entry::new(
            ClientId::from("c"),
            ProjectId::from("p"),
            TaskId::from("t"),
            WorkInterval::new(UNIX_EPOCH, Duration::from_secs(secs), false),
        )
        .with_summary(summary)
        .with_client_name("Client")
        .with_project_name("Project")
        .with_task_name("Task")
    }

    #[test]
    fn render_includes_title_and_counts() {
        let text = render_overview(
            &[sample("done")],
            &[sample("partial")],
            "start",
            "end",
            &[],
            &[],
        );
        assert!(
            text.contains("Worklog entries (start - end)"),
            "missing title:\n{text}"
        );
        assert!(
            text.contains("Complete: 1"),
            "missing complete count:\n{text}"
        );
        assert!(
            text.contains("Incomplete (not uploadable): 1"),
            "missing incomplete count:\n{text}"
        );
        assert!(text.contains("done"), "missing complete summary:\n{text}");
        assert!(
            text.contains("partial"),
            "missing incomplete summary:\n{text}"
        );
    }

    #[test]
    fn hide_column_removes_header() {
        let text = render_overview(&[sample("x")], &[], "a", "b", &[], &["end".to_owned()]);
        assert!(!text.lines().nth(1).unwrap_or("").contains("end"));
    }

    #[test]
    fn render_uses_billable_and_unbillable_headers() {
        let text = render_overview(&[sample("x")], &[], "a", "b", &[], &[]);
        assert!(
            text.contains("billable"),
            "missing billable header:\n{text}"
        );
        assert!(
            text.contains("unbillable"),
            "missing unbillable header:\n{text}"
        );
        assert!(
            !text.contains("| duration |"),
            "unexpected duration header:\n{text}"
        );
    }

    #[test]
    fn footer_sums_billable_and_unbillable_durations() {
        let text = render_overview(
            &[sample("billable-work"), sample_unbillable("internal", 45)],
            &[sample_unbillable("partial", 30)],
            "a",
            "b",
            &[],
            &[],
        );
        assert!(
            text.contains("total time spent"),
            "missing footer label:\n{text}"
        );
        // 90s billable + 45s + 30s unbillable
        assert!(text.contains("00:01:30"), "missing billable total:\n{text}");
        assert!(
            text.contains("00:01:15"),
            "missing unbillable total:\n{text}"
        );
    }
}
