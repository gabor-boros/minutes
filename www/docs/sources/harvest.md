Source documentation for [Harvest](https://getharvest.com/).

## Field mappings

The source makes the following special mappings.

| From | To | Description |
| ---- | -- | ----------- |
| Notes | Notes, Summary | Notes are mapped to both notes and summary as that was the most meaningful option |

## CLI flags

```plaintext
Flags:
    --harvest-account <ID>              Harvest account ID
    --harvest-access-token <TOKEN>      Harvest personal access token
    --harvest-user <ID>                 Harvest user ID
```

## Configuration options

| Config option | Kind | Description | Example |
| ------------- | ---- | ----------- | ------- |
| harvest-account | int | The account ID where the token belongs | harvest-account = 123456789 |
| harvest-access-token | string | Personal access token gathered from Harvest[^1] | harvest-access-token = "<TOKEN>" |
| harvest-user | string | Harvest user ID | harvest-user = "<USER ID>" |

## Limitations

* Harvest does not support tags which makes it impossible to get tasks from tags. A workaround is [planned](https://github.com/gabor-boros/minutes/issues/32).

## Example configuration

```toml
source = "harvest"
target = "tempo"

harvest-account = 123456789
harvest-access-token = "<YOUR ACCESS TOKEN>"
harvest-user = "<YOUR USER ID>"

tempo-url = "https://<org>.atlassian.net/"
tempo-username = "<jira username>"
tempo-password = "<jira password or API token>"
tempo-worker = "<jira username>"

round-to-closest-minute = true
force-billed-duration = true
```

[^1]: Create a new "Personal Access Token" on the [developer settings](https://id.getharvest.com/developers) panel.
