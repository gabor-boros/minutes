Source documentation for [Clockify](https://clockify.me/).

## Field mappings

The source makes the following special mappings.

| From        | To                     | Description                                                                                                                                         |
| ----------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Tags        | Task                   | Turns tags into tasks and split the entry into as many pieces as the item has matching tags when `tags-as-tasks` is enabled                         |
| Task        | Summary or Description | Tasks will be used for defining the summary of an entry; in case the `tags-as-tasks` is enabled, Summary will be set to the Description of the item |
| Description | Notes                  |                                                                                                                                                     |

## CLI flags

The source provides to following extra CLI flags.

```plaintext
Flags:
    --clockify-api-key string      set the API key
    --clockify-url string          set the base URL
    --clockify-workspace string    set the workspace ID
```

## Configuration options

The source provides the following extra configuration options.

| Config option      | Kind   | Description                                                | Example                               |
| ------------------ | ------ | ---------------------------------------------------------- | ------------------------------------- |
| clockify-url       | string | URL for the Clockify installation without a trailing slash | clockify-url = "https://clockify.me"  |
| clockify-api-key   | string | API key gathered from Clockify[^1]                         | clockify-api-key = "<API KEY>"        |
| clockify-workspace | string | Clockify workspace ID[^2]                                  | clockify-workspace = "<WORKSPACE ID>" |

## Limitations

- It is not possible to filter for projects when fetching, though it is a [planned](https://github.com/gabor-boros/minutes/issues/1) feature.

[^1]: As described in the [API documentation](https://clockify.me/developers-api), visit the [settings](https://clockify.me/user/settings) page to get your API token.
[^2]: To get your workspace ID, navigate to workspace settings and copy the ID from the URL.
