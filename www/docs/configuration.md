This page documents the available settings for `minutes`. Please note that not all configuration options are covered by a CLI flag.

## Configuration file

Minutes looks for the configuration file in the following places, based on your operating system.

The configuration file name in **every** case is `.minutes.toml`.

### Linux/Unix

- `$HOME/.minutes.toml`
- `$XDG_CONFIG_HOME/.minutes.toml` (defaults to `$HOME/.config/.minutes.toml` when `XDG_CONFIG_HOME` is unset), as specified by https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html

### Darwin

- `$HOME/.minutes.toml`
- `$HOME/Library/Application Support/.minutes.toml`

### Windows

- `%USERPROFILE%/.minutes.toml`
- `%AppData%/.minutes.toml`

The paths are checked in the order listed above and **only the first existing file is loaded**. A file passed explicitly with `--config PATH` takes precedence over the search order and must exist.

## Environment variables

After the config file is loaded, `MINUTES_*` environment variables override matching keys. Kebab-case config keys map to uppercase names with underscores:

- `toggl-api-token` → `MINUTES_TOGGL_API_TOKEN`
- `source` → `MINUTES_SOURCE`
- `round-to-closest-minute` → `MINUTES_ROUND_TO_CLOSEST_MINUTE`

Precedence: **CLI flags > environment > `.minutes.toml` > defaults**.

## Common configuration

| Config option | Kind | Description | Example | Available options |
| ------------- | ---- | ----------- | ------- | ----------------- |
| date-format | string | Chrono [strftime](https://docs.rs/chrono/latest/chrono/format/strftime/index.html) pattern for `start` / `end` | date-format = "%Y-%m-%d" | |
| dry-run | bool | Fetch entries from source, print them, but do not upload | dry-run = true | |
| yes | bool | Skip the interactive confirmation prompt | yes = true | |
| end | string | End date for fetching entries (must match `date-format`) | end = "2021-10-01" | |
| filter-client | string | Regex of the client name to filter for | filter-client = '^ACME Inc\.?(orporation)$' | |
| filter-project | string | Regex of the project name to filter for | filter-project = '.*(website).*' | |
| force-billed-duration | bool | Treat the total spent time as billable time | force-billed-duration = true | |
| merge-by-day | bool | Merge entries with the same client/project/task/summary on the same day; set to `false` to keep each source entry separate | merge-by-day = false | `true`, `false` |
| round-to-closest-minute | bool | Round time to closest minute, even if the closest minute is 0 (zero) | round-to-closest-minute = true | |
| source | string | Fetch source name | source = "tempo" | `clockify`, `harvest`, `kimai`, `tempo`, `timewarrior`, `toggl` |
| start | string | Start date for fetching entries (must match `date-format`) | start = "2021-10-01" | |
| table-hide-column | []string | Hide the specified columns of the printed overview table | table-hide-column = ["start", "end"] | `summary`, `project`, `client`, `start`, `end` |
| table-sort-by | []string | Sort the printed table by the given column; each option can have a `-` prefix for descending sort | table-sort-by = ["start", "task"] | `task`, `summary`, `project`, `client`, `start`, `end`, `billable`, `unbillable` |
| target | string | Upload target name | target = "tempo" | Check the list of available targets |
| tags-as-tasks-regex | string | Regex of the task pattern | tags-as-tasks-regex = '[A-Z]{2,7}-\d{1,6}' | |

Per-backend user IDs use dedicated keys such as `clockify-user`, `harvest-user`, `kimai-user`, `toggl-user`, and `tempo-worker` (see source/target docs). There is no generic `source-user` / `target-user`.

## Source and target specific configuration

Source and target specific configuration is **not** covered by this guide. For more information, please refer to the source or target documentation.

## Example configuration

```toml
source = "clockify"
target = "tempo"

clockify-url = "https://api.clockify.me/api/v1/"
clockify-api-key = "<clockify API token>"
clockify-workspace = "<clockify workspace ID>"
clockify-user = "<clockify user ID>"

tempo-url = "https://<org>.atlassian.net/"
tempo-username = "<jira username>"
tempo-password = "<jira password or API token>"
tempo-worker = "<jira username>"

tags-as-tasks-regex = '[A-Z]{2,7}-\d{1,6}'
round-to-closest-minute = true
force-billed-duration = true

filter-client = '^ACME Inc\.?(orporation)$'
filter-project = '.*(website).*'

table-sort-by = [
    "start",
    "project",
    "task",
    "summary",
]

table-hide-column = [
    "end"
]
```

## Kimai example

Place the following in the first existing config file from the search order
above (for example `$HOME/.minutes.toml` on Linux). Replace the token with your
own and remove `kimai-user` unless you want to fetch another user's timesheets.

```toml
source = "timewarrior"
target = "kimai"

timewarrior-client-tag-regex = '^client:(.+)$'
timewarrior-project-tag-regex = '^project:(.+)$'
tags-as-tasks-regex = '^([A-Z]+-[0-9]+)$'

kimai-url = "https://kimai.example.com"
kimai-api-token = "<YOUR API TOKEN>"
# kimai-user = 1

round-to-closest-minute = true
```

You can also keep the token out of the file and set it per run:
`MINUTES_KIMAI_API_TOKEN="<token>" minutes`.

