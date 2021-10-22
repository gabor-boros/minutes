Source documentation for [Toggl Track](https://track.toggl.com/).

!!! warning

    To get the available User IDs, please follow [this instruction](https://github.com/toggl/toggl_api_docs/blob/master/chapters/workspaces.md#get-workspace-users).
    Only **workspace admins** can get the User IDs.

!!! info

    Toggl Track's detailed report API does support filtering, and `minutes` unexplicitly supports filtering by setting,
    the source-user to the desired user ID, however it is not officially supported yet.

## Field mappings

The source makes the following special mappings.

| From        | To      | Description                                              |
| ----------- | ------- | -------------------------------------------------------- |
| Description | Summary | Toggl Track has no option to set description for entries |

## CLI flags

The source provides to following extra CLI flags.

```plaintext
Flags:
    --toggl-api-key string      set the API key
    --toggl-url string          set the base URL (default "https://api.track.toggl.com")
    --toggl-workspace int       set the workspace ID
```

## Configuration options

The source provides the following extra configuration options.

| Config option   | Kind   | Description                                                   | Example                                   |
| --------------- | ------ | ------------------------------------------------------------- | ----------------------------------------- |
| toggl-api-key   | string | API key gathered from Toggl Track[^1]                         | toggl-api-key = "<API KEY>"               |
| toggl-workspace | int    | Set the workspace ID                                          | toggl-workspace = 123456789               |

## Limitations

- No precise start and end date filtering is accepted by Toggl Track **report API** that is used for this source, therefore only ISO 8601 (`YYYY-MM-DD`) date format can be used. In Go it is translated to `2006-01-02` when setting `date-format` in config or flags.

## Example configuration

```toml
# Source config
source = "toggl"

# To retrieve your user ID, please follow the instructions listed here:
# https://github.com/toggl/toggl_api_docs/blob/master/chapters/workspaces.md#get-workspace-users
source-user = "<YOUR TOGGL USER ID>"

# Toggl config
toggl-api-key = "<YOUR API KEY>"
toggl-url = "https://api.track.toggl.com"
toggl-workspace = "<YOUR WORKSPACE ID>"

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

[^1]: The API key can be generated as described in their [documentation](https://support.toggl.com/en/articles/3116844-where-is-my-api-key-located).
