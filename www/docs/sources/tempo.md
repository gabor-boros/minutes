Source documentation for [Tempo](https://tempo.io/).

## Field mappings

The source makes the following special mappings.

| From | To | Description |
| ---- | -- | ----------- |
| AccountKey | Client | |
| ProjectKey | Project | |
| IssueKey | Task | |
| Comment | Notes | |

## CLI flags

```plaintext
Flags:
    --tempo-password <PASSWORD>    Tempo password or API token for Basic Auth
    --tempo-url <URL>              Tempo base URL
    --tempo-username <USER>        Tempo username for Basic Auth
    --tempo-worker <WORKER>        Tempo worker key used on search and create requests
```

## Configuration options

| Config option | Kind | Description | Example |
| ------------- | ---- | ----------- | ------- |
| tempo-password | string | Jira password or API token | tempo-password = "<SECRET>" |
| tempo-url | string | URL for the Jira installation | tempo-url = "https://example.atlassian.net/" |
| tempo-username | string | Jira username | tempo-username = "gabor-boros" |
| tempo-worker | string | Worker key for search/create | tempo-worker = "gabor-boros" |

## Limitations

No known limitations.

## Example configuration

```toml
source = "tempo"
target = "tempo"

tempo-url = "https://<org>.atlassian.net/"
tempo-username = "<jira username>"
tempo-password = "<jira password or API token>"
tempo-worker = "<jira username>"

round-to-closest-minute = true
force-billed-duration = true
```
