# Changelog

All notable changes to this project will be documented in this file.

## [unreleased]

**Documentation**

- Fix comment wording ([cb3b3bb](https://github.com/gabor-boros/minutes/commit/cb3b3bb9763bdb6c68d5e93d5f7f16de0605abfe))

**Miscellaneous Tasks**

- Upgrade dependencies ([cefd2af](https://github.com/gabor-boros/minutes/commit/cefd2af0da957cb462974aba2f4390950f44dcc3))

**Refactor**

- Remove tags-as-tasks flag ([8ea8769](https://github.com/gabor-boros/minutes/commit/8ea87697a14c59070d149bcca0823c2cc69228c7))
- Move tags-as-tasks in-house to FetchOpts ([6caa95a](https://github.com/gabor-boros/minutes/commit/6caa95a91e4b0573057868563d19570e90383659))
- Unify how regex is checked ([0c393c5](https://github.com/gabor-boros/minutes/commit/0c393c586ae1af8298f4d207f7459776efb24bfc))
- Split root command ([55723b6](https://github.com/gabor-boros/minutes/commit/55723b664b5eb8cec613783f886716c034354b42))

**Testing**

- Use `ElementsMatch` over `Equal` ([841e0df](https://github.com/gabor-boros/minutes/commit/841e0df9a0ccd4a6b6067b7cff494b89f4c7cbe5))

## [0.2.3] - 2021-11-08

**Build**

- Add homebrew formula publishing ([82d115c](https://github.com/gabor-boros/minutes/commit/82d115c8eda1e2724d5724da622a79b08acb0fb5))

## [0.2.2] - 2021-11-04

**Bug Fixes**

- Multiple small issues fix (#35) ([ebb0730](https://github.com/gabor-boros/minutes/commit/ebb07300af6def0a8338d5e28c63a7496279aa72))

**Build**

- Sign build artifacts ([d69bde3](https://github.com/gabor-boros/minutes/commit/d69bde3c0d7cfff81fd1cfc020a8f860fa0a465f))

## [0.2.1] - 2021-11-04

**Bug Fixes**

- Use 7 char long commit hash ([d137a63](https://github.com/gabor-boros/minutes/commit/d137a63d5fd7a399814922b5ea40769c09df188e))

## [0.2.0] - 2021-11-02

**Bug Fixes**

- Set timeout for tempo uploader ([afc16c1](https://github.com/gabor-boros/minutes/commit/afc16c14d1d8e4a1fc94e31c922ee4a45e1c0b7a))

**Documentation**

- Fix regex value string quotation marks ([f2a9051](https://github.com/gabor-boros/minutes/commit/f2a9051bcdd5f69ddef0dae39f616182d329ff35))

**Features**

- Add filtering for projects and clients (#29) ([ea5031f](https://github.com/gabor-boros/minutes/commit/ea5031f565780ab8476543c0f52a3a22d1ec543c))
- Add token name option for token auth ([cff5e53](https://github.com/gabor-boros/minutes/commit/cff5e53a677e66fc475aaf328307c00b438c1ed5))
- Add Harvest as source (#33) ([c949a0c](https://github.com/gabor-boros/minutes/commit/c949a0c4dbed01af6dafbbe583f52498fd0a68d3))

**Miscellaneous Tasks**

- Update changelog target ([bce0418](https://github.com/gabor-boros/minutes/commit/bce04188d00affa16725d7dfd02f156d7e0b915c))
- Update dependencies ([f1029a7](https://github.com/gabor-boros/minutes/commit/f1029a7da35c646f29750fef0ba8ae3b9056a2a6))

**Refactor**

- Rework client composition logic and remove unnecessary Toggl flag (#30) ([6658984](https://github.com/gabor-boros/minutes/commit/6658984618f7e3c156110f1ac2527390b468d0a8))

## [0.1.0] - 2021-10-20

**Bug Fixes**

- Solve time parsing issue when start or end date is defined ([3d9c7be](https://github.com/gabor-boros/minutes/commit/3d9c7be5fc5df0d259a3faca8976f42c38d83845))
- Upload related task entries sequentially (#14) ([55ffaed](https://github.com/gabor-boros/minutes/commit/55ffaed56218b9c7738bcc1c3d6217cb7a6c8ea6))
- Add missing user filtering for Toggl integration ([649f873](https://github.com/gabor-boros/minutes/commit/649f8738eb6f590df012d4967757ede9476a002e))

**Documentation**

- Add readme file ([6784310](https://github.com/gabor-boros/minutes/commit/6784310dd87618445dbf07d9894011e78d5183a3))
- Add code of conduct ([6b29c4f](https://github.com/gabor-boros/minutes/commit/6b29c4f160c740cf1a96c0f2e9c35f8dc1ec240b))
- Update go install instructions ([a91840f](https://github.com/gabor-boros/minutes/commit/a91840f69b4797f27fb707dedae767c53aff6f33))
- Add project documentation and changelog generation ([c82e766](https://github.com/gabor-boros/minutes/commit/c82e766bb29c436c49e909ef6123a64b50872407))
- Fix broken links ([9938b0b](https://github.com/gabor-boros/minutes/commit/9938b0b08d99022383357f4c9e2caded66323fcc))
- Update project home page in help output ([114dfdb](https://github.com/gabor-boros/minutes/commit/114dfdbcd07e86dbc49e545f0415aad6ef9b7291))
- Extend release document ([d1c24c2](https://github.com/gabor-boros/minutes/commit/d1c24c20e38cead13302eefc8517213438a6bca8))
- Correct some typos ([4c4eea6](https://github.com/gabor-boros/minutes/commit/4c4eea6b2adc1a08b39364e14f0147be56830fcd))
- Fix configuration option kind ([7da72a9](https://github.com/gabor-boros/minutes/commit/7da72a9300572c9bb4caeaa57d6839cabe60ccfd))
- Update bug report and feature request links ([fb79d57](https://github.com/gabor-boros/minutes/commit/fb79d57ec297bc535521e52e94b20ea1e20f7ab8))
- Update documentation generation path triggers ([cabc5d9](https://github.com/gabor-boros/minutes/commit/cabc5d9ec03533881d1c7fc5fcc65c832adb8449))
- Add migration guides ([aaebe2c](https://github.com/gabor-boros/minutes/commit/aaebe2c548ab5ddee972d9757d592a38c0dc361b))
- Change installation instructions ([b7a644f](https://github.com/gabor-boros/minutes/commit/b7a644f600682996ce7a6fe692b1f4bda577b4ea))
- Add example configuration for all sources ([693bf6a](https://github.com/gabor-boros/minutes/commit/693bf6afaf06f3f19de6c620467d2a877aa7a317))

**Features**

- Initial worklog implementation ([b73017b](https://github.com/gabor-boros/minutes/commit/b73017bc6e29c12af91848ee39304f7b65060d1b))
- Add basic client implementation ([2501bcc](https://github.com/gabor-boros/minutes/commit/2501bccb7e73982780e37454685822e50766ae9c))
- Add basic tempo client implementation ([202ac41](https://github.com/gabor-boros/minutes/commit/202ac41def09858d31809be3a6fa8cf5b9f95a00))
- Add basic clockify client implementation ([cb04282](https://github.com/gabor-boros/minutes/commit/cb04282b206bc1a926ab6e37b4cd67450e2c4766))
- Add initial CLI implementation ([98a6759](https://github.com/gabor-boros/minutes/commit/98a6759ec7557d5bdc5e313f00086cc468ee4197))
- Add initial timewarrior integration ([748a304](https://github.com/gabor-boros/minutes/commit/748a30424cc8ad61eb0be44c9e5bf3e32a905ace))
- Add upload status indicator (#10) ([d27c124](https://github.com/gabor-boros/minutes/commit/d27c12426b7c864261c31c43e9101f7599a31167))
- Add initial Toggl Track integration (#13) ([59c2a17](https://github.com/gabor-boros/minutes/commit/59c2a179b6ef21a94c4280017682862eedd41de8))

**Miscellaneous Tasks**

- Add MIT license ([3c3b64c](https://github.com/gabor-boros/minutes/commit/3c3b64cd2d05e93d25d9e9e4a100d9c323bd3e33))
- Add initial .gitignore ([47e5b92](https://github.com/gabor-boros/minutes/commit/47e5b9219274e9051bf207f85c9b2e3fe6b1f82d))
- Add dependencies ([1a24535](https://github.com/gabor-boros/minutes/commit/1a2453537aa3750a36b0883c6b7214e5f110385c))
- Add issue templates ([99fba16](https://github.com/gabor-boros/minutes/commit/99fba16dc5a695d42d9dfee21fc7dad64ce98afe))
- Add virtualenv to gitignore ([466aa6d](https://github.com/gabor-boros/minutes/commit/466aa6d7d3cba1aba26185873c606d16c3e59483))
- Refactor and add badges ([72f091f](https://github.com/gabor-boros/minutes/commit/72f091f8fcfb18584e51e9064d7691de2abc5217))
- Add pull request template ([21ce60a](https://github.com/gabor-boros/minutes/commit/21ce60a68125fe3bf22e6505becda6249b9cdcdf))
- Create PR welcome messages ([76f99b6](https://github.com/gabor-boros/minutes/commit/76f99b635f0ced3bfe64012454138a9fe5a75cf9))

**Refactor**

- Rename worklog search and create path ([b3d1ede](https://github.com/gabor-boros/minutes/commit/b3d1edee419da9858018e32fe3374b1ba96d6be1))
- Return a list of entries instead of a pointer to a list of entries ([000a6b7](https://github.com/gabor-boros/minutes/commit/000a6b7e8409288dba3d1de7ee3aabdbfd663568))
- Rename every occurance of item to entry ([38f37ab](https://github.com/gabor-boros/minutes/commit/38f37ab0b981ee51c2151cb30569a6619ca3c6fa))
- Update command headline ([e1fa381](https://github.com/gabor-boros/minutes/commit/e1fa3813de36951bba594004a4210b994703a9fa))
- Replace table printer and refactor utils ([67721bf](https://github.com/gabor-boros/minutes/commit/67721bfdd69e74ce043d870f13f9faffc91de7df))
- Rename tasks-as-tags to tags-as-tasks and tasks-as-tags-regex to tags-as-tasks-regex ([180126b](https://github.com/gabor-boros/minutes/commit/180126b8f22fbbfc56243f90007b260c82eef227))
- Rename ci.yml to build.yml ([4165ea4](https://github.com/gabor-boros/minutes/commit/4165ea4eddf529563c4b8b54ea914a71c53d5ff9))
- Rename codeql-analysis.yml to codeql.yml ([88edae1](https://github.com/gabor-boros/minutes/commit/88edae1c0741141b5750ba79ca14bbdbe7741976))
- Remove unused `verbose` flag ([28f865d](https://github.com/gabor-boros/minutes/commit/28f865da49f9568fbdf3a8a9da1033ed0006584c))
- Do not return pointer slice when splitting ([481eb3b](https://github.com/gabor-boros/minutes/commit/481eb3b23ca228c6d6e898a47de793e2e3a79d67))
- Add entry duration splitting as a method ([4fbb077](https://github.com/gabor-boros/minutes/commit/4fbb077aa7bc1bb8f214e981544b92ec13425164))
- Use outsourced entry duration splitting ([7be81c2](https://github.com/gabor-boros/minutes/commit/7be81c2431468679a753547a2a225c3b9560c8fb))
- Wrap errors into client.ErrFetchEntries ([90f3f2b](https://github.com/gabor-boros/minutes/commit/90f3f2bfe008e8c1d6e82ef0d8255dd50ba4ed0f))
- Simplify worklog creation ([15bdad7](https://github.com/gabor-boros/minutes/commit/15bdad721f648586f1175b403ca987daa114f400))
- Fix multiple quality issues (#27) ([08dff13](https://github.com/gabor-boros/minutes/commit/08dff13aa2dc28bfdf811339612dd95f33b8f70e))
- Adjust release workflow ([6854d9a](https://github.com/gabor-boros/minutes/commit/6854d9ad41006d414527ed9e088af5597c44cdcc))

**Testing**

- Add benchmarks for NewWorklog ([87f6767](https://github.com/gabor-boros/minutes/commit/87f6767ea04e5d74787b9c6ef348040cb4efb441))
- Remove unused mock server opts ([9fba963](https://github.com/gabor-boros/minutes/commit/9fba963788638154a3caa058f66ed624711d2dd0))
- Use UTC for time zone in tests ([145031e](https://github.com/gabor-boros/minutes/commit/145031e88bb97b8db68851b8173044edc90dd232))
- Fix annoying flaky tests ([48b57c6](https://github.com/gabor-boros/minutes/commit/48b57c676c6e60f503de5ad638cfa03c16a8464d))

**Build**

- Add initial Makefile ([d25eab8](https://github.com/gabor-boros/minutes/commit/d25eab83162bd8d14a6b949205030d084785034d))
- Add post build hook to call upx ([6391c0f](https://github.com/gabor-boros/minutes/commit/6391c0f16b0dab7d4693eb3d4f3215d6fecfffa2))
- User .Version in snapshot name ([d3299d3](https://github.com/gabor-boros/minutes/commit/d3299d3416836439a4400be3819ab152b19c322f))
- Add coverage reporting ([5911595](https://github.com/gabor-boros/minutes/commit/5911595e2c71b348eac7972bc52864e0140e7b76))
- Add several Makefile improvements ([291bc75](https://github.com/gabor-boros/minutes/commit/291bc754cdb2feb644a4d0733c0675ceddcaee05))
- Remove upx for now ([a05c010](https://github.com/gabor-boros/minutes/commit/a05c0101c35bc819e2b459df07f9a708b5ca13e3))
- Fix build removing test output ([d861564](https://github.com/gabor-boros/minutes/commit/d861564d9f467d86d17f2064139a76474c3b1eab))

**Ci**

- Add CodeQL integration ([29d4b74](https://github.com/gabor-boros/minutes/commit/29d4b74d8eada294703efd0be668685beb8672da))
- Setup PR builds ([210c58f](https://github.com/gabor-boros/minutes/commit/210c58f7423c04668c4982d7f536027c420f9d15))
- Update cron frequency ([05db753](https://github.com/gabor-boros/minutes/commit/05db7538cb9c4fd76a0b1e5fdb2a33207421d423))
- Disable build targets but ubuntu ([c4c04f5](https://github.com/gabor-boros/minutes/commit/c4c04f5ab6c109f9c6c483cfe8ce801e112faf01))
- Report coverage ([bb4982e](https://github.com/gabor-boros/minutes/commit/bb4982ec3978e0da62a7b4188e861fce0213b695))
- Checkout code before coverage reporting ([ecb3edb](https://github.com/gabor-boros/minutes/commit/ecb3edbeafa98f0ec8a5214747ec4c18ba1ac398))
- Fine-tune artifact stashing ([8e05ab3](https://github.com/gabor-boros/minutes/commit/8e05ab35c86d47c1da1369c08e51ebf40316fd25))
- Do not run docs deploy on pull requests ([637bb6e](https://github.com/gabor-boros/minutes/commit/637bb6ebbb7e7a3800ca07ce7d23353b3ef60a48))
- Replace PR welcome bot ([b4b7291](https://github.com/gabor-boros/minutes/commit/b4b729126fa6f068e2680f71e1172c08d938caf4))

