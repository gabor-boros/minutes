Migrating from [Tempoit](https://sr.ht/%7Eswalladge/tempoit/).

## Recommended config

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
