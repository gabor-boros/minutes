This page documents the available settings for `minutes`. Please note that not all configuration options are covered by a CLI flag.

## Configuration file

Minutes will look for the following places for the configuration file, based on your operating system.

The configuration file name in **every** case is `.minutes.toml`.

### Linux/Unix

On Linux/Unix systems, the following locations are checked for the configuration file:

- `$HOME/.minutes.toml`
- `$XDG_CONFIG_HOME/.minutes.toml` as specified by https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html

### Darwin

- `$HOME/.minutes.toml`
- `$HOME/Library/Application Support/.minutes.toml`

### Windows

- `%USERPROFILE%/.minutes.toml`
- `%AppData%/.minutes.toml`

### On Plan 9

- `$home/.minutes.toml`
- `$home/lib/.minutes.toml`

## Common configuration

| Config option           | Kind                                                | Description                                                                                                                                   | Example                                               | Available options                                                                |
| ----------------------- | --------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- | -------------------------------------------------------------------------------- |
| date-format             | string                                              | Set the date format in [Go specific](https://www.geeksforgeeks.org/time-formatting-in-golang/) date format                                    | date-format = "2006-01-02"                            |                                                                                  |
| dry-run                 | bool                                                | Fetch entries from source, print the fetched entries, but do not upload them                                                                  | dry-run = true                                        |                                                                                  |
| end                     | string                                              | Set the end date for fetching entries (must match the `date-format`)                                                                          | end = "2021-10-01"                                    |                                                                                  |
| filter-client           | string                                              | Regex of the client name to filter for                                                                                                        | filter-client = '^ACME Inc\.?(orporation)$'           |                                                                                  |
| filter-project          | string                                              | Regex of the project name to filter for                                                                                                       | filter-project = '._(website)._'                      |                                                                                  |
| force-billed-duration   | bool                                                | Treat the total spent time as billable time                                                                                                   | force-billed-duration = true                          |                                                                                  |
| round-to-closest-minute | bool                                                | Round time to closest minute, even if the closest minute is 0 (zero)                                                                          | round-to-closest-minute = true                        |                                                                                  |
| source                  | string                                              | Set the fetch source name                                                                                                                     | source = "tempo"                                      | Check the list of available sources                                              |
| source-user             | string                                              | Set the fetch source user ID                                                                                                                  | source-user = "gabor-boros"                           |                                                                                  |
| start                   | string                                              | Set the start date for fetching entries (must match the `date-format`)                                                                        | start = "2021-10-01"                                  |                                                                                  |
| table-column-config     | [[]table.ColumnConfig][column config documentation] | Customize columns based on the underlying column config struct[^1]                                                                            | table-column-config = { summary = { widthmax = 40 } } |                                                                                  |
| table-hide-column       | []string                                            | Hide the specified columns of the printed overview table                                                                                      | table-hide-column = ["start", "end"]                  | `summary`, `project`, `client`, `start`, `end`                                   |
| table-sort-by           | []string                                            | Sort the specified rows of the printed table by the given column; each sort option can have a `-` (hyphen) prefix to indicate descending sort | table-sort-by = ["start", "task"]                     | `task`, `summary`, `project`, `client`, `start`, `end`, `billable`, `unbillable` |
| table-truncate-column   | map[string]int                                      | Truncate text in the given column to contain no more than `x` characters, where `x` is set by `int`                                           | table-truncate-column = { summary = 30 }              |                                                                                  |
| target                  | string                                              | Set the upload target name                                                                                                                    | target = "tempo"                                      | Check the list of available targets                                              |
| target-user             | string                                              | Set the upload target user ID                                                                                                                 | target = "gabor-boros"                                |                                                                                  |
| tags-as-tasks-regex     | string                                              | Regex of the task pattern                                                                                                                     | tags-as-tasks-regex = '[A-Z]{2,7}-\d{1,6}'            |                                                                                  |

## Source and target specific configuration

Source and target specific configuration is **not** covered by this guide. For more information, please refer to the source or target documentation.

## Example configuration

```toml
# Source config
source = "clockify"
source-user = "<clockify user ID>"

clockify-url = "https://api.clockify.me"
clockify-api-key = "<clockify API token>"
clockify-workspace = "<clockify workspace ID>"

# Target config
target = "tempo"
target-user = "<jira username>"

tempo-url = "https://<org>.atlassian.net"
tempo-username = "<jira username>"
tempo-password = "<jira password>"

# General config
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

[table-column-truncates]
summary = 40
project = 10
client = 10

# Column Config
[table-column-config.summary]
widthmax = 40
widthmin = 20
```

[^1]: The column configuration cannot be mapped directly as-is. Therefore, the configuration option names are lower-cased. Also, some settings cannot be used that would require Go code, like transformers.

[column config documentation]: https://github.com/jedib0t/go-pretty/blob/b2f15441a4e4addd806df446c65f0ce5e327003c/table/config.go#L7-L71
