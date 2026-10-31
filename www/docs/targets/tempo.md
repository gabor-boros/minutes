Target documentation for [Tempo](https://tempo.io/).

!!! warning

    Tempo can go crazy when not a whole minute is uploaded. It is highly recommended using the `round-to-closest-minute` option.

## Field mappings

The target makes the following special mappings.

| From | To | Description |
| ---- | -- | ----------- |
| Summary | Comment | The entry summary will be used as the comment |
| Task | OriginTaskID | Since OriginTaskID must be an Issue Key, the Issue Key defined by Task must exist in Jira |
| tempo-worker | Worker | Worker key sent on create/search requests |

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

- It is not possible to filter for projects when fetching, though it is a [planned](https://github.com/gabor-boros/minutes/issues/1) feature.
- Tempo entries cannot have Summary and Notes at the same time, therefore we use Summary for the comment field during upload.
- At the moment, it is not possible to upload an entry in the name of someone else beyond setting `tempo-worker`.
