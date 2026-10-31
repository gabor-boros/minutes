Minutes is a CLI tool, primarily made for entrepreneurs and finance people, to help their daily work by synchronizing worklogs from a `source` to a `target` software. The `source` can be your own time tracking tool you use, while the `target` is a bookkeeping software.

This guide shows you the basics of `minutes`, walks through the available flags, and gives some examples for basic configuration. For the full list of available configuration options, visit the related [documentation](https://gabor-boros.github.io/minutes/configuration).

Upgrading from 0.3.x? Read the [0.3 → 0.4 migration guide](migrations/from-0.3-to-0.4.md).

## Installation

### Using `brew`

```shell
$ brew tap gabor-boros/brew
$ brew install minutes
```

### Manual install

To install `minutes`, use one of the [release artifacts](https://github.com/gabor-boros/minutes/releases). If you have a Rust toolchain installed, you can build from source as well:

```shell
$ cargo install --path crates/minutes-cli
```

### Configuration

`minutes` has numerous flags and there will be more when other sources or targets are added. Therefore, `minutes` comes with a config file, that can be placed in the user's home directory or the config directory.

## Usage

```plaintext
Usage: minutes [OPTIONS]

Options:
      --config <PATH>                          Path to a `.minutes.toml` file
  -s, --source <SOURCE>                        Source time tracker [clockify|harvest|tempo|timewarrior|toggl]
  -t, --target <TARGET>                        Target time tracker
      --start <START>                          Inclusive range start (must match `--date-format`)
      --end <END>                              Inclusive range end (must match `--date-format`)
      --date-format <FORMAT>                   Chrono strftime pattern (default %Y-%m-%d %H:%M:%S)
      --dry-run                                Fetch and print entries without uploading
      --yes                                    Skip the interactive "Continue?" confirmation
      --force-billed-duration                  Treat total time as billable before upload
      --round-to-closest-minute                Round each entry's duration to the closest minute
      --tags-as-tasks-regex <REGEX>            Regex of tags that should become tasks
      --filter-client <REGEX>                  Regex filter applied to client names after fetch
      --filter-project <REGEX>                 Regex filter applied to project names after fetch
      --table-sort-by <COLUMN>...              Sort overview table by these columns
      --table-hide-column <COLUMN>...          Hide these columns from the overview table
      --clockify-url <URL>                     Clockify API base URL
      --clockify-api-key <KEY>                 Clockify API key
      --clockify-workspace <ID>                Clockify workspace ID
      --clockify-user <ID>                     Clockify user ID
      --harvest-access-token <TOKEN>           Harvest personal access token
      --harvest-account <ID>                   Harvest account ID
      --harvest-user <ID>                      Harvest user ID
      --tempo-url <URL>                        Tempo base URL
      --tempo-username <USER>                  Tempo username for Basic Auth
      --tempo-password <PASSWORD>              Tempo password or API token
      --tempo-worker <WORKER>                  Tempo worker key
      --timewarrior-command <PROGRAM>          Timewarrior executable
      --timewarrior-arguments <ARG>...         Extra Timewarrior arguments
      --timewarrior-unbillable-tag <TAG>       Unbillable tag (default unbillable)
      --timewarrior-client-tag-regex <REGEX>   Client tag regex
      --timewarrior-project-tag-regex <REGEX>  Project tag regex
      --toggl-url <URL>                        Toggl Track API base URL
      --toggl-api-token <TOKEN>                Toggl API token
      --toggl-workspace <ID>                   Toggl workspace ID
      --toggl-user <ID>                        Toggl user ID
  -h, --help                                   Print help
```

## Usage examples

Depending on the config file, the number of flags can change.

### Simplest command

```shell
# No arguments, no flags, just running the command
$ minutes
```

### Preview without uploading

```shell
$ minutes --dry-run
```

### Non-interactive sync

```shell
$ minutes --yes
```

### Set specific date and time

```shell
$ minutes --start "2021-10-07 00:00:00" --end "2021-10-07 23:59:59"
```

```shell
$ minutes --date-format "%Y-%m-%d" --start "2021-10-07" --end "2021-10-08"
```

### Use tags for tasks

```shell
$ minutes --tags-as-tasks-regex '[A-Z]{2,7}-\d{1,6}'
```

### Minute based rounding

```shell
$ minutes --round-to-closest-minute
```

### Format the table output

```shell
$ minutes --table-sort-by "-start" --table-hide-column "client" --table-hide-column "project"
```

## Config file vs flags

Be aware that not all configuration options are covered by flags, especially advanced options.

When using the configuration file and flags in conjunction, flags take precedence over environment variables and the configuration file.
