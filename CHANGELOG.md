# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.3.4 - 2020-09-23

### Fixed

- Build fix.

## 0.3.3 - 2020-09-22

### Fixed

- Only pass `--quiet` to `pull` if we were asked.

## 0.3.2 - 2020-09-20

### Added

- vault: Allow overriding the Vault option `default_policies` on a per-target basis.

### Fixed

- vault: Correctly respect per-target TTLs.

## 0.3.1 - 2020-09-20

### Added

- Allow overriding the Vault options `default_ttl` and `extra_environment` on a per-target basis.

## 0.3.0 - 2020-09-18

### Added

- Support `cage pull --quiet` for use during unit tests.

## 0.3.0-alpha.5 - 2020-09-13

### Added

- Allow `@sha256:`-style digest versions for images.

## 0.3.0-alpha.4 - 2020-09-12

### Added

- We now support `cage source unmount --all` and `cage source unmount s1 s2`.

### Changed

- Vault tokens are now cached in the `.cage` directory, making it feasible to use `config/vault.yml` in development mode.
- Sources are no longer mounted by default, for better monorepo support.

## 0.3.0-alpha.3 - 2020-04-10

### Added

- When using `config/vault.yml`, it is now possible to use `no_default_policies` to indicate that a pod should not receive the default Vault policies.
- Some Windows debugging code has been added to try to figure out the template failures on Appveyor.

## 0.3.0-alpha.2 - 2020-04-09

### Added

- When running on Linux, we now set up `host.docker.internal` in the internal DNS.

### Changed

- The Vault plugin is now enabled on all platforms, including Windows.
- `boondock` now uses `rustls` on all platforms, which should help make Windows support a bit easier.

### Fixed

- The `cage status` command works again, thanks to an updated version of `boondock`.
- Logging and error message newlines have been fixed.

### Removed

- There are no longer any special `cargo` features to disable SSL, since it should now work everywhere.

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
