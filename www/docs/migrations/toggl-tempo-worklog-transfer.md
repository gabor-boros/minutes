Migrating from [Toggl to Jira worklog transfer](https://github.com/giovannicimolin/toggl-tempo-worklog-transfer).

!!! warning
    
    To get your Toggl user ID, please check the [source documentation](https://gabor-boros.github.io/minutes/sources/toggl/)
    of Toggl.

## Recommended config

```toml
# Source config
source = "toggl"
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
tags-as-tasks-regex = '[A-Z]{2,7}-\d{1,6}'
round-to-closest-minute = true
force-billed-duration = true
```