Source documentation for [Tempo](https://tempo.io/).

## Field mappings

The source makes the following special mappings.

| From       | To      | Description                                                                                                                                         |
| ---------- | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| AccountKey | Client  |                                                                                                                                                     |
| ProjectKey | Project | Tasks will be used for defining the summary of an entry; in case the `tags-as-tasks` is enabled, Summary will be set to the Description of the item |
| IssueKey   | Task    |                                                                                                                                                     |
| Comment    | Notes   |                                                                                                                                                     |

## CLI flags

The source provides to following extra CLI flags.

```plaintext
Flags:
    --tempo-password string        set the login password
    --tempo-url string             set the base URL
    --tempo-username string        set the login user ID
```

## Configuration options

The source provides the following extra configuration options.

| Config option  | Kind   | Description                                            | Example                                     |
| -------------- | ------ | ------------------------------------------------------ | ------------------------------------------- |
| tempo-password | string | Jira password                                          | tempo-password = "<SECRET>"                 |
| tempo-url      | string | URL for the Jira installation without a trailing slash | tempo-url = "https://example.atlassian.net" |
| tempo-username | string | Jira username                                          | tempo-username = "gabor-boros"              |

## Limitations

- It is not possible to filter for projects when fetching, though it is a [planned](https://github.com/gabor-boros/minutes/issues/1) feature.
