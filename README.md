# DotR

[![PR Check](https://github.com/uroybd/DotR/actions/workflows/pr-check.yml/badge.svg)](https://github.com/uroybd/DotR/actions/workflows/pr-check.yml)
[![codecov](https://codecov.io/gh/uroybd/DotR/branch/main/graph/badge.svg)](https://codecov.io/gh/uroybd/DotR)
[![License](https://img.shields.io/github/license/uroybd/DotR)](LICENSE)

A dotfiles manager that is as dear as a daughter.

## WARNING!

This is still pre-alpha. The schema is evolving, performance is sub-par. Use it with caution.

## Installation

### Homebrew (macOS and Linux)
```bash
brew tap uroybd/tap
brew install dotr
```

### From Source
```bash
cargo install --git https://github.com/uroybd/DotR
```

### Pre-built Binaries
Download the latest release for your platform from the [releases page](https://github.com/uroybd/DotR/releases):
- **Apple Silicon (M1/M2/M3)**: `dotr-aarch64-apple-darwin.tar.gz`
- **Linux (x86_64)**: `dotr-x86_64-unknown-linux-gnu.tar.gz`

Extract and move the binary to your PATH:
```bash
tar xzf dotr-*.tar.gz
sudo mv dotr /usr/local/bin/
```

## Usage
```
Usage: dotr [OPTIONS] [COMMAND]

Commands:
  init    Intialize dotfiles repository.
  import  Import dotfile and update configuration.
  deploy  Deploy dotfiles from repository.
  update  Update dotfiles to repository.
  help    Print this message or the help of the given subcommand(s)

Options:
  -w, --working-dir <WORKING_DIR>
  -h, --help                       Print help
```

## TODO
- [x] Import configs
- [x] Copy configs
- [x] Update configs
- [ ] Templating and variables
- [ ] Actions
- [ ] Symlinking config
