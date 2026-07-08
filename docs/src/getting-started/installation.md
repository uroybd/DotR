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

## Man pages

`man dotr` (and `man dotr-deploy`, `man dotr-packages`, etc. for every
subcommand) is available if you installed via Homebrew — the formula
installs them automatically. The release tarballs also bundle the same
`.1` files alongside the binary; move them into your local man path to
use them:

```bash
tar xzf dotr-*.tar.gz
sudo mv dotr /usr/local/bin/
sudo mv dotr*.1 /usr/local/share/man/man1/
```

`cargo install` doesn't carry man pages — cargo has no mechanism for
installing them — so they aren't available that way.

## Verifying the install

```bash
dotr --version
dotr --help
```
