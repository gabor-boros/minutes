Source documentation for [Toggl Track](https://track.toggl.com/).

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
| toggl-url       | string | Set the base URL for Toggl Track without a trailing slash[^2] | toggl-url = "https://api.track.toggl.com" |
| toggl-workspace | int    | Set the workspace ID                                          | toggl-workspace = 123456789               |

## Limitations

- It is not possible to filter for projects when fetching, though it is a [planned](https://github.com/gabor-boros/minutes/issues/1) feature.
- No precise start and end date filtering is accepted by Toggl Track **report API** that is used for this source, therefore only ISO 8601 (`YYYY-MM-DD`) date format can be used. In Go it is translated to `2006-01-02` when setting `date-format` in config or flags.

[^1]: The API key can be generated as described in their [documentation](https://support.toggl.com/en/articles/3116844-where-is-my-api-key-located).
[^2]: The URL defaults to `https://api.track.toggl.com` and Toggl Track cannot be installed privately, though they are changing domains nowadays, so if Toggl track changes domain again or start offering private hosting, it can be set easily.
