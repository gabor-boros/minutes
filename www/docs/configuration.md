This page documents the available settings for `minutes`. Please note that not all configuration options are covered by a CLI flag.

## Configuration file

Minutes looks for the configuration file in the following places, based on your operating system.

The configuration file name in **every** case is `.minutes.toml`.

### Linux/Unix

- `$HOME/.minutes.toml`
- `$XDG_CONFIG_HOME/.minutes.toml` as specified by https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html

### Darwin

- `$HOME/.minutes.toml`
- `$HOME/Library/Application Support/.minutes.toml`

### Windows

- `%USERPROFILE%/.minutes.toml`
- `%AppData%/.minutes.toml`

You can also pass an explicit path with `--config PATH`.

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
| round-to-closest-minute | bool | Round time to closest minute, even if the closest minute is 0 (zero) | round-to-closest-minute = true | |
| source | string | Fetch source name | source = "tempo" | `clockify`, `harvest`, `tempo`, `timewarrior`, `toggl` |
| start | string | Start date for fetching entries (must match `date-format`) | start = "2021-10-01" | |
| table-hide-column | []string | Hide the specified columns of the printed overview table | table-hide-column = ["start", "end"] | `summary`, `project`, `client`, `start`, `end` |
| table-sort-by | []string | Sort the printed table by the given column; each option can have a `-` prefix for descending sort | table-sort-by = ["start", "task"] | `task`, `summary`, `project`, `client`, `start`, `end`, `billable`, `unbillable` |
| target | string | Upload target name | target = "tempo" | Check the list of available targets |
| tags-as-tasks-regex | string | Regex of the task pattern | tags-as-tasks-regex = '[A-Z]{2,7}-\d{1,6}' | |

Per-backend user IDs use dedicated keys such as `clockify-user`, `harvest-user`, `toggl-user`, and `tempo-worker` (see source/target docs). There is no generic `source-user` / `target-user`.

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
