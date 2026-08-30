# DotR

[![Codecov](https://img.shields.io/codecov/c/github/uroybd/dotr?style=for-the-badge&label=Coverage&link=https%3A%2F%2Fapp.codecov.io%2Fgithub%2Furoybd%2FDotR&link=https%3A%2F%2Fapp.codecov.io%2Fgithub%2Furoybd%2FDotR)](https://app.codecov.io/github/uroybd/DotR)
[![Deps.rs Crate Dependencies (latest)](https://img.shields.io/deps-rs/dotr-dear/latest?style=for-the-badge)](https://deps.rs/crate/dotr-dear/)
[![GitHub License](https://img.shields.io/github/license/uroybd/dotr?style=for-the-badge)](https://github.com/uroybd/DotR/blob/main/LICENSE)

---
[![GitHub Release](https://img.shields.io/github/v/release/uroybd/dotr?style=for-the-badge&link=https%3A%2F%2Fgithub.com%2Furoybd%2FDotR%2Freleases%2Flatest&link=https%3A%2F%2Fgithub.com%2Furoybd%2FDotR%2Freleases%2Flatest)](https://github.com/uroybd/DotR/releases/latest)
![GitHub Downloads (all assets, all releases)](https://img.shields.io/github/downloads/uroybd/dotr/total?style=for-the-badge&label=Github%20Downloads)
[![Crates.io Total Downloads](https://img.shields.io/crates/d/dotr-dear?style=for-the-badge&label=Crates.io%20Downloads)](https://crates.io/crates/dotr-dear)

A dotfiles manager that is as dear as a daughter.

📖 **[Read the full documentation →](https://dotr.utsob.me/)**

## ✨ Features

- 📦 **Package management** — import, deploy, and update files or whole directories, individually or by profile
- 🎭 **Profiles** — different packages, variables, and destinations per machine (work, home, server, ...)
- 🖥️ **Platforms** — profiles that share an OS share variables and destination overrides without repeating them
- 🔧 **Variables** — environment, config, platform, package, profile, and prompt-sourced, with nested TOML structures
- 📝 **Templating** — full [Tera](https://keats.github.io/tera/) support, detected automatically, no config needed
- ⚡ **Pre/post actions** — run shell commands around deployment, with variable interpolation
- 💬 **Interactive prompts** — ask once for secrets, remember the answer outside version control
- 🔗 **Symlink support** — deploy as symlinks for live-editing workflows
- 🧹 **Clean mode** — deployed directories stay in sync, stray files get removed automatically
- 🔍 **Diff & dry run** — preview every change before it happens

Full details for all of the above are in the [documentation](https://dotr.utsob.me/).

## 📦 Installation

```bash
# Homebrew (macOS and Linux)
brew tap uroybd/tap
brew install dotr

# Cargo
cargo install dotr-dear

# From source
cargo install --git https://github.com/uroybd/DotR
```

Pre-built binaries for macOS and Linux (glibc and musl, x86_64 and aarch64)
are on the [releases page](https://github.com/uroybd/DotR/releases). See the
[installation guide](https://dotr.utsob.me/getting-started/installation.html)
for details.

## 🚀 Quick Start

![DotR quick start demo](docs/src/assets/quick-start.gif)

```bash
dotr init                        # initialize a repository
dotr import ~/.bashrc            # bring an existing dotfile under management
dotr deploy                      # deploy on a (new) machine
dotr diff                        # preview changes before deploying
dotr update                      # pull local edits back into the repository
```

See the [Quick Start guide](https://dotr.utsob.me/getting-started/quick-start.html)
for a complete walkthrough, and the [CLI reference](https://dotr.utsob.me/reference/cli.html)
for every command and flag.

## 🤝 Contributing

Issues and pull requests are welcome.

## 📝 License

[MIT](./LICENSE)
