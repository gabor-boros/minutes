Migrating from [Tempoit](https://sr.ht/%7Eswalladge/tempoit/).

## Recommended config

```toml
# Source config
source = "timewarrior"
source-user = "-"

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
tags-as-tasks-regex = '[A-Z]{2,7}-\d{1,6}'
round-to-closest-minute = true
force-billed-duration = true
```