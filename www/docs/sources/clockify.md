Source documentation for [Clockify](https://clockify.me/).

## Field mappings

The source makes the following special mappings.

| From | To | Description |
| ---- | -- | ----------- |
| Tags | Task | Turns tags into tasks and splits the entry into as many pieces as the item has matching tags when `tags-as-tasks-regex` is set |
| Task | Summary or Description | Tasks are used for the summary of an entry; when `tags-as-tasks-regex` is set, Summary is set to the Description of the item |

## CLI flags

```plaintext
Flags:
    --clockify-api-key <KEY>       Clockify API key
    --clockify-url <URL>           Clockify API base URL (default https://api.clockify.me/api/v1/)
    --clockify-workspace <ID>      Clockify workspace ID
    --clockify-user <ID>           Clockify user ID
```

## Configuration options

| Config option | Kind | Description | Example |
| ------------- | ---- | ----------- | ------- |
| clockify-url | string | Clockify API base URL | clockify-url = "https://api.clockify.me/api/v1/" |
| clockify-api-key | string | API key gathered from Clockify[^1] | clockify-api-key = "<API KEY>" |
| clockify-workspace | string | Clockify workspace ID[^2] | clockify-workspace = "<WORKSPACE ID>" |
| clockify-user | string | Clockify user ID | clockify-user = "<USER ID>" |

## Limitations

No known limitations.

## Example configuration

```toml
source = "clockify"
target = "tempo"

clockify-url = "https://api.clockify.me/api/v1/"
clockify-api-key = "<YOUR API KEY>"
clockify-workspace = "<YOUR WORKSPACE ID>"
clockify-user = "<YOUR USER ID>"

tempo-url = "https://<org>.atlassian.net/"
tempo-username = "<jira username>"
tempo-password = "<jira password or API token>"
tempo-worker = "<jira username>"

tags-as-tasks-regex = '[A-Z]{2,7}-\d{1,6}'
round-to-closest-minute = true
force-billed-duration = true
```

[^1]: As described in the [API documentation](https://clockify.me/developers-api), visit the [settings](https://clockify.me/user/settings) page to get your API token.
[^2]: To get your workspace ID, navigate to workspace settings and copy the ID from the URL.
