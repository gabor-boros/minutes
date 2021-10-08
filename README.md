[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![MIT License][license-shield]][license-url]

<br />
<div align="center">
  <h3 align="center">Minutes</h3>

  <p align="center">
    Sync worklogs between time trackers, invoicing, and bookkeeping software.
    <br />
    <a href="https://github.com/minutes/tree/master/docs/README.md"><strong>Explore the docs</strong></a>
    <br />
    <br />
    <a href="https://github.com/gabor-boros/minutes/issues">Bug report</a>
    ·
    <a href="https://github.com/gabor-boros/minutes/issues">Feature request</a>
  </p>
</div>

<!-- ABOUT THE PROJECT -->
## About The Project

```shell
Incomplete entries:
| Task	| Summary		| Billed| Unbilled	|
| 	| 			| 	| 		|
| 	| 			| 26m16s| 0s		|
| 	| 			| 	| 		|
| 	| Total time spent:	| 26m16s| 0s		|

Complete entries:
| Task		| Summary						| Billed	| Unbilled	|
| 		| 							| 		| 		|
| TA-4685	| Read developer reviews				| 8m43s		| 0s		|
| TA-4305	| Check ticket updates and clean up after the ticket	| 23m25s	| 0s		|
| TA-4815	| Review ticket						| 43m22s	| 0s		|
| TA-4869	| Continue the discovery document			| 3h24m2s	| 0s		|
| TA-4869	| Read the API developer documentations			| 1h56m15s	| 0s		|
| TA-4909	| Participate in firefighting				| 25m37s	| 0s		|
| 		| 							| 		| 		|
| 		| Total time spent:					| 7h1m24s	| 0s		|

Continue? [y/n]:
```

Minutes is a CLI tool for synchronizing work logs between multiple time trackers, invoicing, and bookkeeping software to make entrepreneurs' daily work easier.  Every source and destination comes with their specific flags. Before using any flags, check the related documentation.

Minutes come with absolutely **NO WARRANTY**; before and after synchronizing any logs, please ensure you got the expected result.

## Getting Started

### Prerequisites

Based on the nature of the project, prerequisites depending on what tools you are using. In case you are using Clockify as a time tracker and Tempo as your sync target, you should have an account at Clockify and Jira.

### Installation

To install `minutes`, use one of the [release artifacts](https://github.com/gabor-boros/minutes/releases). If you have `go` installed, you can simply run `go install https://github.com/gabor-boros/minutes` as well.

`minutes` has numerous flags and there will be more when other sources or targets are added. Therefore, `minutes` comes with a config file, that can be placed to the user's home directory or the config directory.

_To read more about the config file, please refer to the [Documentation](https://github.com/minutes/tree/master/docs/README.md)_

## Usage

Below you can find more information about how to use `minutes`.

```plaintext
Usage:
  minutes [flags]

Flags:
      --clockify-api-key string      set the API key
      --clockify-url string          set the base URL
      --clockify-workspace string    set the workspace ID
      --config string                config file (default is $HOME/.minutes.yaml)
      --date-format string           set start and end date format (in Go style) (default "2006-01-02 15:04:05")
      --dry-run                      fetch entries, but do not sync them
      --end string                   set the end date (defaults to now)
      --force-billed-duration        treat every second spent as billed
  -h, --help                         help for minutes
      --round-to-closest-minute      round time to closest minute
  -s, --source string                set the source of the sync [clockify tempo]
      --source-user string           set the source user ID
      --start string                 set the start date (defaults to 00:00:00)
  -t, --target string                set the target of the sync [tempo]
      --target-user string           set the source user ID
      --tasks-as-tags                treat tags matching the value of tasks-as-tags-regex as tasks
      --tasks-as-tags-regex string   regex of the task pattern
      --tempo-password string        set the login password
      --tempo-url string             set the base URL
      --tempo-username string        set the login user ID
      --verbose                      print verbose messages
      --version                      show command version
```



### Usage examples

Depending on the config file, the number of flags can change.

#### Simplest command

```shell
# No arguments, no flags, just running the command
$ minutes
```

#### Set specific date and time

```shell
# Set the date and time to fetch entries in the given time frame
$ minutes --start "2021-10-07 00:00:00" --end "2021-10-07 23:59:59"
```

```shell
# Specify the start and end date format
$ minutes --date-format "2006-01-02" --start "2021-10-07" --end "2021-10-08"
```

#### Use tags for tasks

```shell
# Specify how a tag should look like to be considered as a task
$ minutes --tasks-as-tags --tasks-as-tags-regex '[A-Z]{2,7}-\d{1,6}'
```

#### Minute based rounding

```shell
# Set the billed and unbilled time separately
# to round to the closest minute (even if it is zero)
$ minutes --round-to-closest-minute
```

### Simple config file

```toml
# Source config
source = "clockify"
source-user = "<clockify user ID>"

clockify-url = "https://api.clockify.me"
clockify-api-key = "<clockify API token>"
clockify-workspace = "<clockify workspace ID>"

# Target config
target = "tempo"
target-user = "<jira username>"

tempo-url = "https://tasks.opencraft.com"
tempo-username = "<jira username>"
tempo-password = "<jira password>"

# General config
tasks-as-tags = true
tasks-as-tags-regex = '[A-Z]{2,7}-\d{1,6}'
round-to-closest-minute = true
force-billed-duration = true
```

## Supported tools

| Tool        | Use as source     | Use as target |
| ----------- | ----------------- | ------------- |
| Clockify    | **yes**           | upon request  |
| Everhour    | upon request      | upon request  |
| FreshBooks  | upon request      | **planned**   |
| Harvest     | upon request      | upon request  |
| QuickBooks  | upon request      | upon request  |
| Tempo       | **yes**           | **yes**       |
| Time Doctor | upon request      | upon request  |
| TimeCamp    | upon request      | upon request  |
| Timewarrior | upon request      | upon request  |
| Toggl Track | **planned**       | upon request  |
| Zoho Books  | upon request      | **planned**   |

See the [open issues](https://github.com/gabor-boros/minutes/issues) for a full list of proposed features, tools and known issues.

## Unsupported features

The following list of features are not supported at the moment:

* Cost rate sync
* Hourly rate sync
* Estimate sync
* Multiple source and target user support

## Contributing

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

If you have a suggestion that would make this tool better, please fork the repo and create a pull request. You can also simply open an issue.
Don't forget to give the project a star!

1. Fork the Project
2. Create your Feature Branch (`git checkout -b github-username/amazing-feature`)
3. Commit your Changes (`git commit -m 'feat(new tool): add my favorite tool as a source`)
4. Push to the Branch (`git push origin github-username/amazing-feature`)
5. Open a Pull Request

<!-- MARKDOWN LINKS & IMAGES -->
[contributors-shield]: https://img.shields.io/github/contributors/gabor-boros/minutes.svg?style=for-the-badge
[contributors-url]: https://github.com/gabor-boros/minutes/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/gabor-boros/minutes.svg?style=for-the-badge
[forks-url]: https://github.com/gabor-boros/minutes/network/members
[stars-shield]: https://img.shields.io/github/stars/gabor-boros/minutes.svg?style=for-the-badge
[stars-url]: https://github.com/gabor-boros/minutes/stargazers
[issues-shield]: https://img.shields.io/github/issues/gabor-boros/minutes.svg?style=for-the-badge
[issues-url]: https://github.com/gabor-boros/minutes/issues
[license-shield]: https://img.shields.io/github/license/gabor-boros/minutes.svg?style=for-the-badge
[license-url]: https://github.com/gabor-boros/minutes/blob/master/LICENSE
