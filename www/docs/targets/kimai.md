Target documentation for [Kimai](https://www.kimai.org/).

!!! warning

    Kimai expects project and activity IDs, not free-form names. When the source
    provides non-numeric project or activity identifiers, `minutes` looks them up
    by name through the Kimai API. The names must match existing Kimai projects
    and activities exactly (case-insensitive).

!!! warning

    Kimai treats incoming datetimes as local time in the user's configured
    timezone. `minutes` sends the entry start/end in the machine's local
    timezone, so keep both timezones aligned.

## Field mappings

The target makes the following special mappings.

| From | To | Description |
| ---- | -- | ----------- |
| Summary, Notes | Description | Summary is preferred; notes are used when the summary is empty |
| Project (id or name) | Project | Numeric IDs are used as-is, otherwise resolved by name |
| Task (id or name) | Activity | Numeric IDs are used as-is, otherwise resolved by name |
| Interval billable flag | — | Not sent; Kimai applies the activity's default |

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
| kimai-user | int | Fetch timesheets for this user (default: token user) | kimai-user = 1 |

## Limitations

- `kimai-user` filters fetched timesheets only; create requests always target the
  token user because the API rejects the `user` field on some Kimai versions.
- The billable flag and tags are not uploaded; Kimai applies the activity's
  default billable setting.
- When the source is Timewarrior, each successfully uploaded entry tags its
  source interval with `kimai_id:<id>`, so later runs skip already uploaded
  intervals.
- Name-based resolution picks the first project matching the name and, when a
  client is set, prefers the project belonging to that customer. Use numeric IDs
  if project names are ambiguous.

## Example configuration (Timewarrior → Kimai)

```toml
source = "timewarrior"
target = "kimai"

kimai-url = "https://kimai.example.com"
kimai-api-token = "<YOUR API TOKEN>"

# Tags become client / project / task names, then Kimai resolves them.
timewarrior-client-tag-regex = '^client:(.+)$'
timewarrior-project-tag-regex = '^project:(.+)$'
tags-as-tasks-regex = '^[A-Z]{2,7}-\d{1,6}$'

round-to-closest-minute = true
```
