# Changelog

All notable changes to this project will be documented in this file.

## [unreleased]

**Bug Fixes**

- Solve time parsing issue when start or end date is defined (3d9c7be)

**Documentation**

- Add readme file (6784310)
- Add code of conduct (6b29c4f)
- Update go install instructions (a91840f)
- Add project documentation and changelog generation (48b7f98)

**Features**

- Initial worklog implementation (b73017b)
- Add basic client implementation (2501bcc)
- Add basic tempo client implementation (202ac41)
- Add basic clockify client implementation (cb04282)
- Add initial CLI implementation (98a6759)

**Miscellaneous Tasks**

- Add MIT license (3c3b64c)
- Add initial .gitignore (47e5b92)
- Add dependencies (1a24535)
- Add issue templates (99fba16)
- Add virtualenv to gitignore (466aa6d)

**Refactor**

- Rename worklog search and create path (b3d1ede)
- Return a list of entries instead of a pointer to a list of entries (000a6b7)
- Rename every occurance of item to entry (38f37ab)
- Update command headline (e1fa381)
- Replace table printer and refactor utils (67721bf)
- Rename tasks-as-tags to tags-as-tasks and tasks-as-tags-regex to tags-as-tasks-regex (180126b)
- Rename ci.yml to build.yml (4165ea4)
- Rename codeql-analysis.yml to codeql.yml (88edae1)

**Testing**

- Add benchmarks for NewWorklog (87f6767)
- Remove unused mock server opts (9fba963)
- Use UTC for time zone in tests (145031e)

**Build**

- Add initial Makefile (d25eab8)
- Add post build hook to call upx (6391c0f)

**Ci**

- Add CodeQL integration (29d4b74)
- Setup PR builds (210c58f)
- Update cron frequency (05db753)

