Source documentation for [Clockify](https://clockify.me/).

## CLI flags

```plaintext
Flags:
    --clockify-api-key string      set the API key
    --clockify-url string          set the base URL
    --clockify-workspace string    set the workspace ID
```

## Configuration options

| Config option      | Kind   | Description                                                | Example                               |
|--------------------|--------|------------------------------------------------------------|---------------------------------------|
| clockify-url       | string | URL for the Clockify installation without a trailing slash | clockify-url = "https://clockify.me"  |
| clockify-api-key   | string | API key gathered from Clockify[^1]                         | clockify-api-key = "<API KEY>"        |
| clockify-workspace | string | Clockify workspace ID[^2]                                 | clockify-workspace = "<WORKSPACE ID>" |

## Limitations

* It is not possible to filter for projects when fetching, though it is a [planned](https://github.com/gabor-boros/minutes/issues/1) feature.

[^1]: As described in the [API documentation](https://clockify.me/developers-api), visit the [settings](https://clockify.me/user/settings) page to get your API token.
[^2]: To get your workspace ID, navigate to workspace settings and copy the ID from the URL.