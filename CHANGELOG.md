# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.3.0-alpha.1 - 2020-04-08

### Added

- Vault and OpenSSL support are enabled in Mac binaries by default.

### Changed

- The code has been updated to reasonably idiomatic Rust 2018.
- We have replaced our `boondock` Docker client with `dockworker`, which is a better-maintained fork of `boondock`. Many thanks to the `dockworker` maintainers!
- We have upgraded all of our immediate dependencies to something reasonably modern, and replaced a few of them.
- `compose_yml` has been upgraded to a similarly modernized version.

### Fixed

- `docker-compose.yml` validation has been re-enabled, thanks to the `compose_yml` update.
