Source documentation for [Timewarrior](https://timewarrior.net/).

Timewarrior is one of the most flexible tools. Thanks to its flexibility there is no built-in/dedicated way to mark an entry billable/unbillable, set client, project, or task.

Therefore, several assumptions were made to integrate with Timewarrior, though the goal was to keep the maximum flexibility.

!!! warning

    Timewarrior has no built-in support for marking an entry billable/unbillable. Therefore, every entry will be treated
    as billable unless it is not forced by `force-billed-duration` or a matching tag for `timewarrior-unbillable-tag`.

!!! warning

    When `timewarrior-client-tag-regex` or `timewarrior-project-tag-regex` is matching multiple tags, the last tag will be used.

!!! warning

    To extract tasks from tags, set the `tags-as-tasks-regex` regardless the value of `tags-as-tasks`.

## Field mappings

The source makes the following special mappings.

| From       | To                                | Description                                                                                              |
| ---------- | --------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Annotation | Notes, Summary, Task (optionally) | Annotations are used to set Notes and Summary; if no task regex is set, it will be used for Task as well |
| Tags       | Client, Project, Task             | Depending on the client, project, and task regex, tags will be used accordingly                          |

## CLI flags

The source provides to following extra CLI flags.

```plaintext
Flags:
    --timewarrior-arguments strings          set additional arguments
    --timewarrior-client-tag-regex string    regex of client tag pattern
    --timewarrior-command string             set the executable name (default "timew")
    --timewarrior-project-tag-regex string   regex of project tag pattern
    --timewarrior-unbillable-tag string      set the unbillable tag (default "unbillable")
```

## Configuration options

The source provides the following extra configuration options.

| Config option                 | Kind    | Description                                                         | Example                                          |
| ----------------------------- | ------- | ------------------------------------------------------------------- | ------------------------------------------------ |
| timewarrior-arguments         | []string | Set additional arguments for the export command                    | timewarrior-arguments = "reviewed"               |
| timewarrior-client-tag-regex  | string  | Set the regular expression for extracting Client names from tags    | timewarrior-client-tag-regex = '^(CLIENT-\w+)$'  |
| timewarrior-command           | string  | Set the timewarrior command                                         | timewarrior-command = "timew"                    |
| timewarrior-project-tag-regex | string  | Set the regular expression for extracting Project names from tags   | timewarrior-project-tag-regex = '^PROJ-DEV-\w+$' |
| timewarrior-unbillable-tag    | string  | Set the regular expression to identify which entries are unbillable | timewarrior-unbillable-tag = "unbillable"        |

## Limitations

No known limitations.

## Example configuration

```toml
# Source config
source = "timewarrior"
source-user = "-"  # Timewarrior does not support multiple users

# Timewarrior config
timewarrior-arguments = ["log"]
timewarrior-client-tag-regex = '^(oc)$'
timewarrior-project-tag-regex = '^(log)$'

# Target config
target = "tempo"
target-user = "<jira username>"

# Tempo config
tempo-url = "https://<org>.atlassian.net"
tempo-username = "<jira username>"
tempo-password = "<jira password>"

# General config
tags-as-tasks = true
tags-as-tasks-regex = '[A-Z]{2,7}-\d{1,6}'
round-to-closest-minute = true
force-billed-duration = true
```
