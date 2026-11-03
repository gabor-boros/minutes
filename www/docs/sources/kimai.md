Source documentation for [Kimai](https://www.kimai.org/).

!!! info

    Kimai's API returns timesheet datetimes in the user's timezone. `minutes`
    converts them to absolute instants, so day grouping follows the machine's
    local timezone.

## Field mappings

The source makes the following special mappings.

| From | To | Description |
| ---- | -- | ----------- |
| Customer | Client | Taken from the project's customer |
| Project | Project | Kimai project |
| Activity | Task | Kimai activities map to tasks |
| Description | Summary, Notes | Kimai has a single description field |
| Tags | Tasks | When `tags-as-tasks-regex` is set, matching tags are split into tasks |

Running timesheets without an end time are skipped unless Kimai reports a
positive duration.

## CLI flags

```plaintext
Flags:
    --kimai-api-token <TOKEN>    Kimai API token
    --kimai-url <URL>            Kimai base URL (for example https://kimai.example.com)
    --kimai-user <ID>            Kimai user ID used for fetching timesheets
```

## Configuration options

| Config option | Kind | Description | Example |
| ------------- | ---- | ----------- | ------- |
| kimai-api-token | string | API token created in the Kimai user profile | kimai-api-token = "<API TOKEN>" |
| kimai-url | string | Base URL of the Kimai installation (with or without `/api`) | kimai-url = "https://kimai.example.com" |
| kimai-user | int | User ID to fetch timesheets for (default: token user) | kimai-user = 1 |

## Limitations

- Fetching for another user requires the `view_other_timesheet` permission.

## Example configuration

```toml
source = "kimai"
target = "timewarrior"

kimai-url = "https://kimai.example.com"
kimai-api-token = "<YOUR API TOKEN>"

timewarrior-client-tag-regex = '^client:(.+)$'
timewarrior-project-tag-regex = '^project:(.+)$'
```
