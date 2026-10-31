# Contributing

Contributions are welcome, and they are greatly appreciated! Every little helps, and credit will always be given. You can contribute in many ways.

## Types of Contributions

### Report Bugs

Report bugs at <https://github.com/gabor-boros/minutes/issues>.

If you are reporting a bug, please use the bug report template, and include:

- your operating system name and version
- any details about your local setup that might be helpful in troubleshooting
- detailed steps to reproduce the bug

### Fix Bugs

Look through the GitHub issues for bugs. Anything tagged with "bug" and "help wanted" is open to whoever wants to implement it.

### Implement Features

Look through the GitHub issues for features. Anything tagged with "enhancement" and "help wanted" is open to whoever wants to implement it. In case you added a new source or target, do not forget to add them to the docs as well.

### Write Documentation

Minutes could always use more documentation, whether as part of the docs, in docstrings, or even on the web in blog posts, articles, and such.

### Submit Feedback

The best way to send feedback is to file an [issue](https://github.com/gabor-boros/minutes/issues).

If you are proposing a feature:

- explain in detail how it would work
- keep the scope as narrow as possible, to make it easier to implement
- remember that this is a volunteer-driven project, and that contributions are welcome :)

## Get Started!

Ready to contribute? Here's how to set up `minutes` for local development.

As step 0 make sure you have [mise](https://mise.jdx.dev/) installed.

1. Fork the repository
2. Clone your fork locally

```shell
$ git clone git@github.com:your_name_here/minutes.git
```

3. Install tools and hooks

```shell
$ cd minutes
$ mise trust
$ mise install
$ mise run pre-commit
$ mise run docs:deps
```

4. Create a branch for local development

```shell
$ git checkout -b github-username/bugfix-or-feature-name
```

5. When you're done making changes, check that your changes are formatted, passing linters, and tests are succeeding

```shell
$ mise run format
$ mise run lint
$ mise run test
```

6. Update documentation and check the results by running `mise run docs`
7. Commit your changes and push your branch to GitHub

We use [conventional commits](https://www.conventionalcommits.org/). Commit messages are checked by commitlint via the `commit-msg` hook.

```shell
$ git add .
$ git commit -m "feat(scope): summary"
$ git push origin github-username/bugfix-or-feature-name
```

8. Submit a pull request on GitHub

## Pull Request Guidelines

Before you submit a pull request, check that it meets these guidelines:

1. The pull request should include tests
2. Tests should pass for the PR
3. If the pull request adds new functionality, or changes existing one, the docs should be updated

## Releasing

A reminder for the maintainers on how to release.

Releases are driven by [Release Please](https://github.com/googleapis/release-please). Conventional commits on `main` open or update a release PR that bumps `[workspace.package].version` in the root `Cargo.toml` (member crates inherit via `version.workspace = true`) and updates `CHANGELOG.md`. The config uses `release-type: simple` because the `rust` strategy cannot rewrite inherited member versions.

1. Make sure every required PR is merged
2. Make sure CI is green on `main`
3. Wait for (or trigger) the Release Please PR
4. Review the changelog and version bump, then merge the Release Please PR
5. Merging creates the GitHub Release/tag; the release workflow builds multi-platform binaries, uploads them to the release, and updates the Homebrew formula in `gabor-boros/homebrew-brew`

### Maintainer secrets

Configure these repository secrets for CI:

- `HOMEBREW_TAP_TOKEN` — token with write access to `gabor-boros/homebrew-brew`
- Optionally a PAT for Release Please if the default `GITHUB_TOKEN` cannot open release PRs
