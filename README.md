# DotR

[![PR Check](https://github.com/uroybd/DotR/actions/workflows/pr-check.yml/badge.svg)](https://github.com/uroybd/DotR/actions/workflows/pr-check.yml)
[![codecov](https://codecov.io/gh/uroybd/DotR/branch/main/graph/badge.svg)](https://codecov.io/gh/uroybd/DotR)
[![License](https://img.shields.io/github/license/uroybd/DotR)](LICENSE)

A dotfiles manager that is as dear as a daughter.

## Documentation

For detailed documentation, guides, and examples, visit the [DotR Wiki](https://github.com/uroybd/DotR/wiki).

## Features

### 📦 Package Management
- **Import dotfiles** from any location into your repository
- **Deploy dotfiles** to their target locations
- **Update changes** back to your repository
- Support for both **files and directories**

### 🔧 Variables
- **Environment variables** automatically available in all templates
- **Custom user variables** defined in `config.toml`
- **Nested variable structures** with TOML tables and arrays
- **Print variables** command to view all available variables
- Config variables **override environment variables**

### 📝 Templating (Tera)
- **Full Tera template engine** support
- Use `{{ variable }}` for variable substitution
- Use `{% if condition %}` for conditional logic
- Use `{# comment #}` for template comments
- **Automatic template detection** - no configuration needed
- Templates are **compiled during deployment** with live variables
- Templated files are **never backed up** (source of truth stays in templates)

### 🎯 Smart Workflows
- Templated and regular files can coexist in the same repository
- Selective package deployment and updates
- Automatic backup before deployment
- Directory structure preservation

## Quick Start

1. **Initialize** a dotfiles repository:
```bash
dotr init
```

2. **Import** your existing dotfiles:
```bash
dotr import ~/.bashrc
dotr import ~/.config/nvim/
```

3. **Deploy** dotfiles to a new machine:
```bash
dotr deploy
```

4. **Update** after making changes:
```bash
dotr update
```

## Variables Example

Define variables in `config.toml`:
```toml
[variables]
EDITOR = "nvim"
THEME = "dracula"

[variables.git]
name = "Your Name"
email = "you@example.com"
```

Use in your dotfiles:
```bash
# ~/.bashrc (can be templated)
export EDITOR="{{ EDITOR }}"
export PS1="{% if THEME == 'dracula' %}🧛{% endif %} $ "
```

## Templating Example

Create a templated config file:
```toml
# ~/.config/myapp/config.toml
[user]
name = "{{ git.name }}"
email = "{{ git.email }}"
editor = "{{ EDITOR }}"

[paths]
home = "{{ HOME }}"
data = "{{ HOME }}/Data"
```

When deployed, variables are automatically substituted. Template files are never backed up during `update` - they remain as templates in your repository.

## WARNING!

This is still pre-alpha. The schema is evolving, performance is sub-par. Use it with caution.

## Installation

### Homebrew (macOS and Linux)

Supports both Apple Silicon and Intel Macs.

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
- **Intel Mac**: `dotr-x86_64-apple-darwin.tar.gz`
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
  init        Intialize dotfiles repository.
  import      Import dotfile and update configuration.
  deploy      Deploy dotfiles from repository.
  update      Update dotfiles to repository.
  print-vars  Print all user variables.
  help        Print this message or the help of the given subcommand(s)

Options:
  -w, --working-dir <WORKING_DIR>
  -h, --help                       Print help
```

## TODO
- [x] Import configs
- [x] Copy configs
- [x] Update configs
- [x] Variables (with nested structures)
- [x] Templating (Tera engine)
- [ ] Actions (pre/post hooks)
- [ ] Symlinking config
