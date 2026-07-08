# Installation

## Homebrew (macOS and Linux)

Supports both Apple Silicon and Intel Macs.

```bash
brew tap uroybd/tap
brew install dotr
```

## Cargo

```bash
cargo install dotr-dear
```

This installs a binary named `dotr` (the crate is published as `dotr-dear`
since `dotr` was already taken on crates.io).

## From source

```bash
cargo install --git https://github.com/uroybd/DotR
```

## Pre-built binaries

Download the latest release for your platform from the
[releases page](https://github.com/uroybd/DotR/releases):

| Platform                    | Archive                                      |
| ---------------------------- | --------------------------------------------- |
| Apple Silicon (M1/M2/M3)     | `dotr-aarch64-apple-darwin.tar.gz`            |
| Intel Mac                    | `dotr-x86_64-apple-darwin.tar.gz`             |
| Linux (x86_64)               | `dotr-x86_64-unknown-linux-gnu.tar.gz`        |
| Linux (aarch64)              | `dotr-aarch64-unknown-linux-gnu.tar.gz`       |
| Linux musl (x86_64)          | `dotr-x86_64-unknown-linux-musl.tar.gz`       |
| Linux musl (aarch64)         | `dotr-aarch64-unknown-linux-musl.tar.gz`      |

Extract and move the binary onto your `PATH`:

```bash
tar xzf dotr-*.tar.gz
sudo mv dotr /usr/local/bin/
```

## Verifying the install

```bash
dotr --version
dotr --help
```
