Migrating from [Toggl to Jira worklog transfer](https://github.com/giovannicimolin/toggl-tempo-worklog-transfer).

!!! warning

    To get your Toggl user ID, please check the [source documentation](https://gabor-boros.github.io/minutes/sources/toggl/)
    of Toggl.

## Recommended config

```toml
source = "toggl"
target = "tempo"

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
