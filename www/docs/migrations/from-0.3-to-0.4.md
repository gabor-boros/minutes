# Migrating from 0.3.x to 0.4.x

Minutes 0.4.x is a Rust rewrite of the 0.3.x Go CLI. The binary name, brew tap, and overall workflow are the same, but **`.minutes.toml` keys and date formats changed**. Update your config before syncing real data, and run with `--dry-run` first.

## Install

Install from the same places as before:

```shell
brew tap gabor-boros/brew
brew install minutes
```

Or download a release artifact from [GitHub Releases](https://github.com/gabor-boros/minutes/releases). Building from source now requires a Rust toolchain (`cargo install --path crates/minutes-cli`) instead of Go.

## Date format

`date-format` no longer uses [Go reference layouts](https://pkg.go.dev/time#pkg-constants). It uses [chrono strftime](https://docs.rs/chrono/latest/chrono/format/strftime/index.html) patterns.

| 0.3.x (Go)              | 0.4.x (chrono)           |
| ----------------------- | ------------------------ |
| `2006-01-02 15:04:05`   | `%Y-%m-%d %H:%M:%S`      |
| `2006-01-02`            | `%Y-%m-%d`               |
| `15:04:05`              | `%H:%M:%S`               |

Default in 0.4.x is `%Y-%m-%d %H:%M:%S`.

## User IDs: `source-user` / `target-user` removed

Generic `source-user` and `target-user` are gone. Set the user (or worker) on the backend that needs it:

| 0.3.x | 0.4.x | Notes |
| ----- | ----- | ----- |
| `source-user` (Clockify) | `clockify-user` | |
| `source-user` (Harvest) | `harvest-user` | |
| `source-user` (Toggl) | `toggl-user` | |
| `source-user` (Tempo) | `tempo-worker` | Worker key for search/create; still use `tempo-username` for auth |
| `target-user` (Tempo) | `tempo-worker` | Same worker key when Tempo is the target |
| `source-user = "-"` (Timewarrior) | _(remove)_ | Timewarrior has no user id |

## Renamed keys

| 0.3.x | 0.4.x |
| ----- | ----- |
| `toggl-api-key` | `toggl-api-token` |
| `harvest-api-key` | `harvest-access-token` |

## Removed keys

These go-pretty table options have no equivalent in 0.4.x; delete them from your config:

- `table-column-config` (and nested `[table-column-config.*]` tables)
- `table-column-truncates` / `table-truncate-column`

`table-sort-by` and `table-hide-column` still work.

## Unchanged keys

These keep the same names: `source`, `target`, `start`, `end`, `dry-run`, `force-billed-duration`, `round-to-closest-minute`, `tags-as-tasks-regex`, `filter-client`, `filter-project`, `table-sort-by`, `table-hide-column`, plus existing Clockify / Tempo / Timewarrior URL and credential fields that already matched (`clockify-api-key`, `tempo-url`, `tempo-username`, `tempo-password`, `timewarrior-*`, and so on).

## New options

- `yes` / `--yes` — skip the interactive "Continue?" confirmation
- Environment overrides with a `MINUTES_` prefix: kebab-case config keys map to `MINUTES_` + underscores (for example `toggl-api-token` → `MINUTES_TOGGL_API_TOKEN`, `source` → `MINUTES_SOURCE`)

Configuration precedence: **CLI flags > `MINUTES_*` env > `.minutes.toml` > defaults**.

## Before / after examples

### Clockify → Tempo

**0.3.x**

```toml
source = "clockify"
source-user = "<clockify user ID>"

clockify-url = "https://api.clockify.me"
clockify-api-key = "<clockify API token>"
clockify-workspace = "<clockify workspace ID>"

target = "tempo"
target-user = "<jira username>"

tempo-url = "https://<org>.atlassian.net"
tempo-username = "<jira username>"
tempo-password = "<jira password>"

date-format = "2006-01-02 15:04:05"
tags-as-tasks-regex = '[A-Z]{2,7}-\d{1,6}'
round-to-closest-minute = true
force-billed-duration = true

[table-column-truncates]
summary = 40
```

**0.4.x**

```toml
source = "clockify"
target = "tempo"
date-format = "%Y-%m-%d %H:%M:%S"
tags-as-tasks-regex = '[A-Z]{2,7}-\d{1,6}'
round-to-closest-minute = true
force-billed-duration = true

clockify-url = "https://api.clockify.me/api/v1/"
clockify-api-key = "<clockify API token>"
clockify-workspace = "<clockify workspace ID>"
clockify-user = "<clockify user ID>"

tempo-url = "https://<org>.atlassian.net/"
tempo-username = "<jira username>"
tempo-password = "<jira password or API token>"
tempo-worker = "<jira username>"
```

### Toggl → Tempo

**0.3.x**

```toml
source = "toggl"
source-user = "<toggl user ID>"
toggl-api-key = "<API KEY>"
toggl-workspace = 123456789

target = "tempo"
target-user = "<jira username>"
tempo-url = "https://<org>.atlassian.net"
tempo-username = "<jira username>"
tempo-password = "<jira password>"
```

**0.4.x**

```toml
source = "toggl"
target = "tempo"

toggl-api-token = "<API KEY>"
toggl-workspace = 123456789
toggl-user = "<toggl user ID>"

tempo-url = "https://<org>.atlassian.net/"
tempo-username = "<jira username>"
tempo-password = "<jira password or API token>"
tempo-worker = "<jira username>"
```

## Checklist

1. Rewrite `date-format` to chrono strftime (or remove it to use the default).
2. Replace `source-user` / `target-user` with the per-backend keys above.
3. Rename `toggl-api-key` and `harvest-api-key` if you use those sources.
4. Delete `table-column-config` / truncate settings.
5. Run `minutes --dry-run` and confirm the overview table before uploading.
