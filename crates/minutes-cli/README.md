# minutes

Sync worklog entries between time trackers (Clockify, Harvest, Tempo, Timewarrior, Toggl).

## Quick start

```bash
# Preview today's entries (no upload)
minutes --dry-run

# Sync with confirmation prompt
minutes

# Non-interactive sync
minutes --yes
```

Configuration is loaded from `.minutes.toml` (see below), then `MINUTES_*` environment
variables, then CLI flags. Later layers win.

## Configuration

Search order:

1. `--config PATH`
2. `$HOME/.minutes.toml`
3. OS config directory (XDG / `~/Library/Application Support` / `%AppData%`) + `/.minutes.toml`

Example:

```toml
source = "toggl"
target = "tempo"
date-format = "%Y-%m-%d %H:%M:%S"
tags-as-tasks-regex = '[A-Z]{2,7}-\d{1,6}'
round-to-closest-minute = true
force-billed-duration = true

toggl-api-token = "..."
toggl-workspace = 123456
toggl-user = "789"

tempo-url = "https://jira.example.com/"
tempo-username = "..."
tempo-password = "..."
tempo-worker = "jdoe"
```

Environment overrides use a `MINUTES_` prefix and underscores
(`MINUTES_TOGGL_API_TOKEN`, `MINUTES_SOURCE`, …).

Date strings use [`chrono` strftime](https://docs.rs/chrono/latest/chrono/format/strftime/index.html)
patterns (default `%Y-%m-%d %H:%M:%S`), not Go reference layouts.

## Safety

Minutes does not guarantee idempotent uploads. Always review the overview table
before confirming. Prefer `--dry-run` when trying a new configuration.
