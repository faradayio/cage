# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 0.4.1-pre7 - 2026-10-19

### Changed (breaking)

- Host ports now move by a per-checkout offset derived from the project name, so two checkouts of one repo can run on the same machine. Hard-coded `localhost:<port>` URLs need to read `CAGE_HOST_PORT_<POD>_<SERVICE>_<CONTAINER_PORT>` (see `cage status`, or the list `cage up` prints). Set `CAGE_HOST_PORT_OFFSET` when two checkouts clash.

### Changed

- Updated Rust dependencies, including `dirs` 6 → 7.
- Replaced `serde_yaml` with `serde-saphyr`. `serde_yaml` depends on `unsafe-libyaml`, which is unmaintained.

## 0.4.1-pre6 - 2026-08-19

### Fixed

- Clippy on Rust 1.97 (`useless_borrows_in_formatting`).

## 0.4.1-pre5 - 2026-08-19

### Changed

- Updated Rust dependencies, including `itertools` 0.14 → 0.15.
- Replaced the unmaintained `hashicorp_vault` client with `vaultrs` 0.8.

### Fixed

- Retry transient Vault HTTP errors (connection reset, timeout) when issuing
  service tokens. GitHub Actions runners reset connections to Vault often
  enough that a single `lookup-self` was failing `cage pull`.

## 0.4.1-pre4 - 2026-06-16

### Changed

- When stderr is not a terminal (e.g. GitHub Actions), set `COMPOSE_PROGRESS=quiet` on all `docker-compose` invocations so implicit image pulls during `run`/`test` do not spam layer download progress.
- `cage pull` now defaults to `--quiet` when stderr is not a terminal.
- `cage test` passes `--pull never` to `docker-compose run` so tests use the already-pulled image instead of re-pulling with verbose progress.

## 0.4.1-pre3 - 2026-05-14

### Fixed

- `cage up --init` no longer hangs at "Waiting for pod ... to be listening on all ports". The `bollard 0.19 → 0.21` upgrade in pre2 started populating container IPs from `NetworkSettings.Networks` (the compose-managed custom network), and the existing `TcpListener::bind`-based port probe always errored with `EADDRNOTAVAIL` against those non-local addresses, causing the wait loop to spin forever.

### Removed

- Deleted the long-broken `ContainerInfo::is_listening_to_ports` port probe and its supporting fields/methods (`ip_addr`, `container_tcp_ports`, `socket_addrs`). The probe was a no-op for docker-compose-managed containers in every Docker version since user-defined networks became the compose default (~early 2016). `init_pod` now waits on `RuntimeState::all_services_in_pod_are_running`, which actually verifies that every service's containers reached the `Running` state.

## 0.4.1-pre2 - 2026-05-13

### Changed

- Made `cage pull` faster.

## 0.4.0 - 2025-10-29

### Changed

- Replaced `boondock` Docker client with `bollard` for better async/await support and active maintenance.
- Updated `tokio` runtime from 0.2 to 1.x for better async compatibility.
- Switched from openssl to rustls

## 0.3.6 - 2021-05-10

### Fixed

- Fix warning that broke build.
- Update dependencies.

## 0.3.5 - 2021-05-10

### Added

- Support `cage test --export-test-output`.

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
