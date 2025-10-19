# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.1](https://github.com/anaxo-io/multicast-rs/compare/v1.2.0...v1.2.1) (2025-10-19)


### Bug Fixes

* all clippy warnings with -D warnings flag ([d44c9c7](https://github.com/anaxo-io/multicast-rs/commit/d44c9c79520983537884706e69bfe0f4a9291e93))
* **ci:** add permissions for release-please job ([77f12c7](https://github.com/anaxo-io/multicast-rs/commit/77f12c736dd6e933c8baad79e2eb8a19fe960a22))
* **ci:** add rustfmt component to CI workflow ([45c9bf6](https://github.com/anaxo-io/multicast-rs/commit/45c9bf6580cef9919aa27b566ebfa3fac2704967))
* **ci:** update release-please action token ([910fbd2](https://github.com/anaxo-io/multicast-rs/commit/910fbd2b8990b2b32333743f4dd2d72176b69bce))
* **ci:** update release-please action token ([f425a10](https://github.com/anaxo-io/multicast-rs/commit/f425a109adc5bf0401f2bd31872ad7a64019efe1))
* **ci:** use GITHUB_TOKEN for release-please action ([b582dda](https://github.com/anaxo-io/multicast-rs/commit/b582dda759b4913c69a488c2fd784e02bd643158))
* **release:** update release token and configuration ([71e6e77](https://github.com/anaxo-io/multicast-rs/commit/71e6e777f3c08683740f8aaa6b8f779aefa376cc))
* update bincode to version 2.x ([9735657](https://github.com/anaxo-io/multicast-rs/commit/9735657c0098219a621391761e9dd3ba4b7114d0))
* update bincode to version 2.x ([00eb008](https://github.com/anaxo-io/multicast-rs/commit/00eb0086e9a343fc1a4f386974ea8af6cf03e092))

## [1.2.0](https://github.com/hmedkouri/multicast-rs/compare/v1.1.1...v1.2.0) (2025-05-17)


### Features

* **stats:** implement publisher and subscriber statistics tracking ([e78e2e3](https://github.com/hmedkouri/multicast-rs/commit/e78e2e39c442eb01b1e1aae2ed5d77d58cb58c91))

## [1.1.1](https://github.com/hmedkouri/multicast-rs/compare/v1.1.0...v1.1.1) (2025-05-17)


### Bug Fixes

* **subscriber:** in receive interpret timeout: None as non-blocking socket mode ([bc92d70](https://github.com/hmedkouri/multicast-rs/commit/bc92d70ca45d0bd787f8bacfe3c1f17b0630e68d))

## [1.1.0](https://github.com/hmedkouri/multicast-rs/compare/v1.0.0...v1.1.0) (2025-05-17)


### Features

* **subscriber:** add generic deserialization methods to MulticastSubscriber ([cf3c376](https://github.com/hmedkouri/multicast-rs/commit/cf3c376ed4016ab24c32be24ebe095d355596022))

## [1.0.0](https://github.com/hmedkouri/multicast-rs/compare/v0.2.0...v1.0.0) (2025-05-17)


### ⚠ BREAKING CHANGES

* **subscriber:** Removed the legacy `process_raw_batch` method and replaced it with the new deserialization-based API for better performance and usability

### Features

* **subscriber:** implement efficient batch deserialization with form… ([f470a93](https://github.com/hmedkouri/multicast-rs/commit/f470a936427b9e664192b42ba52449cfe0e17617))
* **subscriber:** implement efficient batch deserialization with format selection ([274d07a](https://github.com/hmedkouri/multicast-rs/commit/274d07ab8362f2f1d7e6730379da3062e556dc33))


### Bug Fixes

* **tests:** update sender socket creation to include options ([c61ce4f](https://github.com/hmedkouri/multicast-rs/commit/c61ce4fd994ef3141cdd88d2fd29a81684710740))

## [0.2.0](https://github.com/hmedkouri/multicast-rs/compare/v0.1.0...v0.2.0) (2025-05-17)


### Features

* add string-based API for multicast publisher/subscriber ([4898477](https://github.com/hmedkouri/multicast-rs/commit/4898477fe633d1bf4488de514b722f327cd0c36b))
* add string-based API for multicast publisher/subscriber ([5fa4517](https://github.com/hmedkouri/multicast-rs/commit/5fa451772a92403206bf128983e7349b6fea5dca))

## 0.1.0 (2025-05-17)


### Bug Fixes

* update release-please action reference ([f93a2d4](https://github.com/hmedkouri/multicast-rs/commit/f93a2d416b7a2956e41fcbbe747858b0b7bd173c))

## [Unreleased]

### Added
- Initial project setup

Note: This CHANGELOG will be automatically updated by the GitHub Release-Please action based on conventional commit messages.
