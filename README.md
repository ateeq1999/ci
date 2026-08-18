# ci

A CLI for scaffolding NestJS starter projects.

## Install

Download the archive for your platform from the
[latest release](https://github.com/ateeq1999/ci/releases/latest), then
extract it and put the binary on your `PATH`.

| Platform            | Asset                                 |
| ------------------- | -------------------------------------- |
| Windows (x64)        | `ci-x86_64-pc-windows-msvc.zip`        |
| Linux (x64)           | `ci-x86_64-unknown-linux-gnu.tar.gz`   |
| macOS (Intel)         | `ci-x86_64-apple-darwin.tar.gz`        |
| macOS (Apple Silicon) | `ci-aarch64-apple-darwin.tar.gz`       |

On macOS/Linux, remember to `chmod +x ci` after extracting.

## Usage

```sh
ci init my-api
```

## Updating

```sh
ci update
```

Checks the [releases page](https://github.com/ateeq1999/ci/releases) for a
newer version and replaces the running binary in place. Pass `-y`/`--yes` to
skip the confirmation prompt.

## Releasing

Push a tag matching `v*.*.*` (e.g. `v0.2.0`) to build and publish binaries
for Windows, Linux, and macOS (Intel + Apple Silicon) via
[`.github/workflows/release.yml`](.github/workflows/release.yml).
