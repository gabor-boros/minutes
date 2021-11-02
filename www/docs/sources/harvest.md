Source documentation for [Harvest](https://getharvest.com/).

## Field mappings

The source makes the following special mappings.

| From  | To             | Description                                                                       |
| ----- | -------------- | --------------------------------------------------------------------------------- |
| Notes | Notes, Summary | Notes are mapped to both notes and summary as that was the most meaningful option |

## CLI flags

The source provides to following extra CLI flags.

```plaintext
Flags:
    --harvest-account int          set the Account ID
    --harvest-api-key string       set the API key
```

## Configuration options

The source provides the following extra configuration options.

| Config option   | Kind   | Description                                 | Example                       |
| --------------- | ------ | ------------------------------------------- | ----------------------------- |
| harvest-account | string | The account ID where the API key belongs to | harvest-account = 123456789   |
| harvest-api-key | string | API key gathered from Harvest[^1]           | harvest-api-key = "<API KEY>" |

## Limitations

* Harvest does not support tags which makes it impossible to get tasks from tags. A workaround is [planned](https://github.com/gabor-boros/minutes/issues/32).

## Example configuration

```toml
# Source config
source = "harvest"
source-user = "<YOUR USER ID>"

harvest-account = "<YOUR ACCOUNT ID>"
harvest-api-key = "<YOUR API KEY>"

# Target config
target = "tempo"
target-user = "<jira username>"

tempo-url = "https://<org>.atlassian.net"
tempo-username = "<jira username>"
tempo-password = "<jira password>"

# General config
round-to-closest-minute = true
force-billed-duration = true
```

[^1]: Create a new "Personal Access Token" on the [developer settings](https://id.getharvest.com/developers) panel.
