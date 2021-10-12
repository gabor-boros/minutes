# Changelog

All notable changes to this project will be documented in this file.

## [unreleased]

**Bug Fixes**

- Solve time parsing issue when start or end date is defined ([3d9c7be](https://github.com/gabor-boros/minutes/commit/3d9c7be5fc5df0d259a3faca8976f42c38d83845))

**Documentation**

- Add readme file ([6784310](https://github.com/gabor-boros/minutes/commit/6784310dd87618445dbf07d9894011e78d5183a3))
- Add code of conduct ([6b29c4f](https://github.com/gabor-boros/minutes/commit/6b29c4f160c740cf1a96c0f2e9c35f8dc1ec240b))
- Update go install instructions ([a91840f](https://github.com/gabor-boros/minutes/commit/a91840f69b4797f27fb707dedae767c53aff6f33))
- Add project documentation and changelog generation ([c82e766](https://github.com/gabor-boros/minutes/commit/c82e766bb29c436c49e909ef6123a64b50872407))
- Fix broken links ([9938b0b](https://github.com/gabor-boros/minutes/commit/9938b0b08d99022383357f4c9e2caded66323fcc))
- Update project home page in help output ([fe83511](https://github.com/gabor-boros/minutes/commit/fe8351104f745c6ef2a265a0e139cb9209f5ab9e))
- Extend release document ([b6d61e9](https://github.com/gabor-boros/minutes/commit/b6d61e9221bea664380277c700f8a1842c4b93f0))

**Features**

- Initial worklog implementation ([b73017b](https://github.com/gabor-boros/minutes/commit/b73017bc6e29c12af91848ee39304f7b65060d1b))
- Add basic client implementation ([2501bcc](https://github.com/gabor-boros/minutes/commit/2501bccb7e73982780e37454685822e50766ae9c))
- Add basic tempo client implementation ([202ac41](https://github.com/gabor-boros/minutes/commit/202ac41def09858d31809be3a6fa8cf5b9f95a00))
- Add basic clockify client implementation ([cb04282](https://github.com/gabor-boros/minutes/commit/cb04282b206bc1a926ab6e37b4cd67450e2c4766))
- Add initial CLI implementation ([98a6759](https://github.com/gabor-boros/minutes/commit/98a6759ec7557d5bdc5e313f00086cc468ee4197))
- Add initial timewarrior integration ([823c472](https://github.com/gabor-boros/minutes/commit/823c4720360850c1eaca6a6a7765e43c4a47877c))

**Miscellaneous Tasks**

- Add MIT license ([3c3b64c](https://github.com/gabor-boros/minutes/commit/3c3b64cd2d05e93d25d9e9e4a100d9c323bd3e33))
- Add initial .gitignore ([47e5b92](https://github.com/gabor-boros/minutes/commit/47e5b9219274e9051bf207f85c9b2e3fe6b1f82d))
- Add dependencies ([1a24535](https://github.com/gabor-boros/minutes/commit/1a2453537aa3750a36b0883c6b7214e5f110385c))
- Add issue templates ([99fba16](https://github.com/gabor-boros/minutes/commit/99fba16dc5a695d42d9dfee21fc7dad64ce98afe))
- Add virtualenv to gitignore ([466aa6d](https://github.com/gabor-boros/minutes/commit/466aa6d7d3cba1aba26185873c606d16c3e59483))
- Update changelog ([97d9867](https://github.com/gabor-boros/minutes/commit/97d986761306a892d1354228c650615a7146dfba))
- Use commit links in changelog ([8011d6a](https://github.com/gabor-boros/minutes/commit/8011d6af1d1e2ae917da871b16109991e3118812))
- Update changelog entries ([4b6dc29](https://github.com/gabor-boros/minutes/commit/4b6dc2911349587df3207afea4675b1e3e77033f))
- Update changelog ([2fccd28](https://github.com/gabor-boros/minutes/commit/2fccd287eae65a20160141f6091eb12fd1126040))

**Refactor**

- Rename worklog search and create path ([b3d1ede](https://github.com/gabor-boros/minutes/commit/b3d1edee419da9858018e32fe3374b1ba96d6be1))
- Return a list of entries instead of a pointer to a list of entries ([000a6b7](https://github.com/gabor-boros/minutes/commit/000a6b7e8409288dba3d1de7ee3aabdbfd663568))
- Rename every occurance of item to entry ([38f37ab](https://github.com/gabor-boros/minutes/commit/38f37ab0b981ee51c2151cb30569a6619ca3c6fa))
- Update command headline ([e1fa381](https://github.com/gabor-boros/minutes/commit/e1fa3813de36951bba594004a4210b994703a9fa))
- Replace table printer and refactor utils ([67721bf](https://github.com/gabor-boros/minutes/commit/67721bfdd69e74ce043d870f13f9faffc91de7df))
- Rename tasks-as-tags to tags-as-tasks and tasks-as-tags-regex to tags-as-tasks-regex ([180126b](https://github.com/gabor-boros/minutes/commit/180126b8f22fbbfc56243f90007b260c82eef227))
- Rename ci.yml to build.yml ([4165ea4](https://github.com/gabor-boros/minutes/commit/4165ea4eddf529563c4b8b54ea914a71c53d5ff9))
- Rename codeql-analysis.yml to codeql.yml ([88edae1](https://github.com/gabor-boros/minutes/commit/88edae1c0741141b5750ba79ca14bbdbe7741976))
- Remove unused `verbose` flag ([96c1e83](https://github.com/gabor-boros/minutes/commit/96c1e83bf70dfe62152d4ece1f61351e05834df5))
- Do not return pointer slice when splitting ([6a34847](https://github.com/gabor-boros/minutes/commit/6a34847c150815c25c04077daa557ea5855bf3ae))
- Add entry duration splitting as a method ([e657956](https://github.com/gabor-boros/minutes/commit/e657956f78e3fe37be22e3dfbb5dc65a6d345865))
- Use outsourced entry duration splitting ([e81f1fd](https://github.com/gabor-boros/minutes/commit/e81f1fd08b9ffb93c0d74b0d976e0fb915e4fb4d))
- Wrap errors into client.ErrFetchEntries ([5004245](https://github.com/gabor-boros/minutes/commit/5004245d63d9d8e32b4680b51c6edeb908fd162d))

**Testing**

- Add benchmarks for NewWorklog ([87f6767](https://github.com/gabor-boros/minutes/commit/87f6767ea04e5d74787b9c6ef348040cb4efb441))
- Remove unused mock server opts ([9fba963](https://github.com/gabor-boros/minutes/commit/9fba963788638154a3caa058f66ed624711d2dd0))
- Use UTC for time zone in tests ([145031e](https://github.com/gabor-boros/minutes/commit/145031e88bb97b8db68851b8173044edc90dd232))

**Build**

- Add initial Makefile ([d25eab8](https://github.com/gabor-boros/minutes/commit/d25eab83162bd8d14a6b949205030d084785034d))
- Add post build hook to call upx ([6391c0f](https://github.com/gabor-boros/minutes/commit/6391c0f16b0dab7d4693eb3d4f3215d6fecfffa2))
- User .Version in snapshot name ([4544742](https://github.com/gabor-boros/minutes/commit/4544742275930a371cacd0115167722a694a45c9))

**Ci**

- Add CodeQL integration ([29d4b74](https://github.com/gabor-boros/minutes/commit/29d4b74d8eada294703efd0be668685beb8672da))
- Setup PR builds ([210c58f](https://github.com/gabor-boros/minutes/commit/210c58f7423c04668c4982d7f536027c420f9d15))
- Update cron frequency ([05db753](https://github.com/gabor-boros/minutes/commit/05db7538cb9c4fd76a0b1e5fdb2a33207421d423))
- Disable build targets but ubuntu ([c4c04f5](https://github.com/gabor-boros/minutes/commit/c4c04f5ab6c109f9c6c483cfe8ce801e112faf01))

