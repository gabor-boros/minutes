Source documentation for [Timewarrior](https://timewarrior.net/).

Timewarrior is one of the most flexible tools. Thanks to its flexibility there is no built-in/dedicated way to mark an entry billable/unbillable, set client, project, or task.

Therefore, several assumptions were made to integrate with Timewarrior, though the goal was to keep the maximum flexibility.

!!! warning

    Timewarrior has no built-in support for marking an entry billable/unbillable. Therefore, every entry will be treated
    as billable unless it is forced by `force-billed-duration` or a matching tag for `timewarrior-unbillable-tag`.

!!! warning

    When `timewarrior-client-tag-regex` or `timewarrior-project-tag-regex` is matching multiple tags, the last tag will be used.

!!! warning

    To extract tasks from tags, set the `tags-as-tasks-regex`.

## Field mappings

The source makes the following special mappings.

| From | To | Description |
| ---- | -- | ----------- |
| Annotation | Notes, Summary, Task (optionally) | Annotations are used to set Notes and Summary; if no task regex is set, it will be used for Task as well |
| Tags | Client, Project, Task | Depending on the client, project, and task regex, tags will be used accordingly |

## CLI flags

```plaintext
Flags:
    --timewarrior-arguments <ARG>...           Extra arguments prepended to every Timewarrior invocation
    --timewarrior-client-tag-regex <REGEX>     Regex matching tags treated as the client
    --timewarrior-command <PROGRAM>            Executable name or path (default timew)
    --timewarrior-project-tag-regex <REGEX>    Regex matching tags treated as the project
    --timewarrior-unbillable-tag <TAG>         Tag that marks an interval as non-billable (default unbillable)
```

## Configuration options

| Config option | Kind | Description | Example |
| ------------- | ---- | ----------- | ------- |
| timewarrior-arguments | []string | Extra arguments for Timewarrior | timewarrior-arguments = ["log"] |
| timewarrior-client-tag-regex | string | Regex for extracting Client names from tags | timewarrior-client-tag-regex = '^(CLIENT-\w+)$' |
| timewarrior-command | string | Timewarrior command | timewarrior-command = "timew" |
| timewarrior-project-tag-regex | string | Regex for extracting Project names from tags | timewarrior-project-tag-regex = '^PROJ-DEV-\w+$' |
| timewarrior-unbillable-tag | string | Tag that marks entries as unbillable | timewarrior-unbillable-tag = "unbillable" |

## Limitations

No known limitations.

## Example configuration

```toml
source = "timewarrior"
target = "tempo"

timewarrior-arguments = ["log"]
timewarrior-client-tag-regex = '^(oc)$'
timewarrior-project-tag-regex = '^(log)$'

tempo-url = "https://<org>.atlassian.net/"
tempo-username = "<jira username>"
tempo-password = "<jira password or API token>"
tempo-worker = "<jira username>"

tags-as-tasks-regex = '[A-Z]{2,7}-\d{1,6}'
round-to-closest-minute = true
force-billed-duration = true
```
