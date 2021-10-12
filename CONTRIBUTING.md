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

As step 0 make sure you have Go 1.17+ and Python 3 installed.

1. Fork the repository
2. Clone your fork locally

```shell
$ git clone git@github.com:your_name_here/minutes.git
```

3. Install prerequisites

```shell
$ cd minutes
$ make prerequisites
$ make deps
$ python -m virtualenv -p python3 virtualenv
$ pip install -r www/requirements.txt
```

4. Create a branch for local development

```shell
$ git checkout -b github-username/bugfix-or-feature-name
```

5. When you're done making changes, check that your changes are formatted, passing linters, and tests are succeeding

```shell
$ make format
$ make lint
$ make test
```

6. Update documentation and check the results by running `make docs`
7. Commit your changes and push your branch to GitHub

We use [conventional commits](https://www.conventionalcommits.org/en/v1.0.0-beta.2/), and we require every commit to
follow this pattern.

```shell
$ git add .
$ git commit -m "action(scope): summary"
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

Before doing anything, ensure you have [git-cliff](https://github.com/orhun/git-cliff) installed, and you already
executed `make prerequisites`.

1. Make sure every required PR is merged
2. Make sure every test is passing both on GitHub and locally
3. Make sure that formatters are not complaining (`make format` returns 0)
4. Make sure that linters are not complaining (`make lint` returns 0)
5. Take a note about the next release version, keeping semantic versioning in mind
6. Update the CHANGELOG.md using `TAG="<CURRENT RELEASE VERSION>" make changelog`
7. Compare the CHANGELOG.md changes and push to master
8. Cut a new tag for the next release version 
9. Run `GITHUB_TOKEN="<TOKEN>" make release` to package the tool and create a GitHub release
10. Create a new milestone following the `v<NEXT RELEASE VERSION>` pattern