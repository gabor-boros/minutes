Target documentation for [Tempo](https://tempo.io/).

!!! warning

    Tempo can go crazy when not a whole minute is uploaded. It is highly recommended using the `round-to-closest-minute` option.

## Field mappings

The target makes the following special mappings.

| From       | To           | Description                                                                                   |
| ---------- | ------------ | --------------------------------------------------------------------------------------------- |
| Summary    | Comment      | The entry summary will be used as the comment                                                 |
| Task       | OriginTaskID | Since OriginTaskID must be an Issue Key, the Issue Key defined by Task must represent in Jira |
| tempo-user | Worker       |                                                                                               |

## CLI flags

The target does not provide additional CLI flags.

## Configuration options

The target does not provide additional configuration options.

## Limitations

- It is not possible to filter for projects when fetching, though it is a [planned](https://github.com/gabor-boros/minutes/issues/1) feature.
- Tempo entries cannot have Summary and Notes at the same time, therefore we use Summary for the comment field during upload.
- At the moment, it is not possible to upload an entry in the name of someone else.
