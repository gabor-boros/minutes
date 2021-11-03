Minutes is a CLI tool, primarily made for entrepreneurs and finance people, to help their daily work by synchronizing worklogs from a `source` to a `target` software. The `source` can be your own time tracking tool you use, while the `target` is a bookkeeping software.

This guide show you the basics `minutes`, walks through the available flags, and gives some examples for basic configuration. For the full list of available configuration options, visit the related [documentation](https://gabor-boros.github.io/minutes/configuration).

## Installation

### Using `brew`

``` shell
$ brew tap gabor-boros/brew
$ brew install minutes
```

### Manual install

To install `minutes`, use one of the [release artifacts](https://github.com/gabor-boros/minutes/releases). If you have `go` installed, you can build from source as well

### Configuration

`minutes` has numerous flags and there will be more when other sources or targets are added. Therefore, `minutes` comes with a config file, that can be placed to the user's home directory or the config directory.

## Usage

```plaintext
Usage:
  minutes [flags]

Flags:
      --clockify-api-key string      set the API key
      --clockify-url string          set the base URL
      --clockify-workspace string    set the workspace ID
      --config string                config file (default is $HOME/.minutes.yaml)
      --date-format string           set start and end date format (in Go style) (default "2006-01-02 15:04:05")
      --dry-run                      fetch entries, but do not sync them
      --end string                   set the end date (defaults to now)
      --force-billed-duration        treat every second spent as billed
  -h, --help                         help for minutes
      --round-to-closest-minute      round time to closest minute
  -s, --source string                set the source of the sync [clockify tempo]
      --source-user string           set the source user ID
      --start string                 set the start date (defaults to 00:00:00)
      --table-hide-column strings    hide table column [summary project client start end]
      --table-sort-by strings        sort table by column [task summary project client start end billable unbillable] (default [start,project,task,summary])
  -t, --target string                set the target of the sync [tempo]
      --target-user string           set the source user ID
      --tags-as-tasks-regex string   regex of the task pattern
      --tempo-password string        set the login password
      --tempo-url string             set the base URL
      --tempo-username string        set the login user ID
      --verbose                      print verbose messages
      --version                      show command version
```

## Usage examples

Depending on the config file, the number of flags can change.

### Simplest command

```shell
# No arguments, no flags, just running the command
$ minutes
```

### Set specific date and time

```shell
# Set the date and time to fetch entries in the given time frame
$ minutes --start "2021-10-07 00:00:00" --end "2021-10-07 23:59:59"
```

```shell
# Specify the start and end date format
$ minutes --date-format "2006-01-02" --start "2021-10-07" --end "2021-10-08"
```

### Use tags for tasks

```shell
# Specify how a tag should look like to be considered as a task
$ minutes --tags-as-tasks-regex '[A-Z]{2,7}-\d{1,6}'
```

### Minute based rounding

```shell
# Set the billed and unbilled time separately
# to round to the closest minute (even if it is zero)
$ minutes --round-to-closest-minute
```

### Format the table output

```shell
# Skip some columns and sort table by -start date
$ minutes --table-sort-by "-start" --table-hide-column "client" --table-hide-column "project"
```

## Config file vs flags

Be aware that not all configuration option is covered by flags, especially not more advanced options, like table column width or truncate settings.

When using the configuration file and flags in conjunction, please note that flags take precedence, hence it can override settings from the configuration file.
