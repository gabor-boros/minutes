Source documentation for [Toggl Track](https://track.toggl.com/).

!!! warning

    To get the available User IDs, please follow [this instruction](https://github.com/toggl/toggl_api_docs/blob/master/chapters/workspaces.md#get-workspace-users).
    Only **workspace admins** can get the User IDs.

!!! info

    Toggl Track's detailed report API does support filtering, and `minutes` supports filtering by setting
    `toggl-user` to the desired user ID.

## Field mappings

The source makes the following special mappings.

| From | To | Description |
| ---- | -- | ----------- |
| Description | Summary | Toggl Track has no separate notes field for entries in this integration |

## CLI flags

```plaintext
Flags:
    --toggl-api-token <TOKEN>    Toggl API token
    --toggl-url <URL>            Toggl Track API base URL (default https://api.track.toggl.com/)
    --toggl-workspace <ID>       Toggl workspace ID
    --toggl-user <ID>            Toggl user ID
```

## Configuration options

| Config option | Kind | Description | Example |
| ------------- | ---- | ----------- | ------- |
| toggl-api-token | string | API token gathered from Toggl Track[^1] | toggl-api-token = "<API TOKEN>" |
| toggl-url | string | Toggl Track API base URL | toggl-url = "https://api.track.toggl.com/" |
| toggl-workspace | int | Workspace ID | toggl-workspace = 123456789 |
| toggl-user | string | User ID | toggl-user = "<USER ID>" |

## Limitations

- No precise start and end date filtering is accepted by Toggl Track **report API** that is used for this source, therefore only ISO 8601 (`YYYY-MM-DD`) date format can be used. Set `date-format = "%Y-%m-%d"` when using day-only ranges.

## Example configuration

```toml
source = "toggl"
target = "tempo"

# To retrieve your user ID, please follow the instructions listed here:
# https://github.com/toggl/toggl_api_docs/blob/master/chapters/workspaces.md#get-workspace-users
toggl-user = "<YOUR TOGGL USER ID>"
toggl-api-token = "<YOUR API TOKEN>"
toggl-url = "https://api.track.toggl.com/"
toggl-workspace = 123456789

tempo-url = "https://<org>.atlassian.net/"
tempo-username = "<jira username>"
tempo-password = "<jira password or API token>"
tempo-worker = "<jira username>"

tags-as-tasks-regex = '[A-Z]{2,7}-\d{1,6}'
round-to-closest-minute = true
force-billed-duration = true
```

[^1]: Visit your Toggl Track profile page to create or copy an API token.
