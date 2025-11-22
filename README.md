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
- **Profile-based deployment** for different environments (work, home, server)
- **Profile dependencies** to automatically deploy required packages
- **Package targets** to override destinations per profile

### 🎭 Profiles
- **Environment-specific configurations** (work, home, server, laptop, etc.)
- **Profile variables** that override package and config variables
- **Package dependencies** per profile for automatic deployment
- **Target overrides** to deploy same package to different locations per profile
- Switch profiles with `--profile` flag on deploy, import, and update commands

### 🔧 Variables
- **Environment variables** automatically available in all templates
- **Custom user variables** defined in `config.toml`
- **Package-level variables** for package-specific configurations
- **Profile variables** that override other variables when a profile is active
- **Nested variable structures** with TOML tables and arrays
- **Print variables** command to view all available variables
- **Variable priority**: User variables > Profile variables > Package variables > Config variables > Environment variables
- Secret `uservariables.toml` file to save secrets you don't want to share in VCS

### 💬 Interactive Prompts
- **Config-level prompts** - Global prompts for values used across all packages
- **Package-level prompts** - Package-specific prompts for sensitive configuration
- **Profile-level prompts** - Environment-specific prompts (work credentials, personal tokens, etc.)
- **Smart prompting** - Only prompts once, saves answers to `.uservariables.toml`
- **Skip existing values** - Won't prompt for variables already defined
- Prompts are displayed during deploy, update, and diff commands

### 📝 Templating (Tera)
- **Full Tera template engine** support
- Use `{{ variable }}` for variable substitution
- Use `{% if condition %}` for conditional logic
- Use `{# comment #}` for template comments
- **Automatic template detection** - no configuration needed
- Templates are **compiled during deployment** with live variables
- Templated files are **never backed up** (source of truth stays in templates)

### ⚡ Actions (Pre/Post Hooks)
- **Pre-deployment actions** run before package deployment
- **Post-deployment actions** run after package deployment
- Execute **shell commands** with full variable interpolation
- Multiple actions per package, executed in order
- Perfect for: installing dependencies, reloading services, setting permissions, etc.

### 🎯 Smart Workflows
- Templated and regular files can coexist in the same repository
- **Granular file deployment** - only deploys files when content has changed
- **Granular backups** - creates per-file backups (`.dotrbak`) instead of directory backups
- **Diff command** to preview changes before deployment
- Selective package deployment and updates
- Profile-based deployments for different machines/environments
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

# Import for a specific profile
dotr import ~/.ssh/config --profile work
```

3. **Deploy** dotfiles to a new machine:
```bash
# Deploy all packages
dotr deploy

# Deploy with a specific profile
dotr deploy --profile work

# Deploy specific packages
dotr deploy --packages nvim,tmux
```

4. **Check differences** before deploying:
```bash
# See what would change if you deployed
dotr diff

# Diff specific packages
dotr diff --packages nvim,bashrc

# Diff with a profile
dotr diff --profile work
```

5. **Update** after making changes:
```bash
dotr update

# Update with a profile
dotr update --profile work
```

## Variables Example

```toml
[variables]
EDITOR = "nvim"

[variables.git]
name = "Your Name"
email = "you@example.com"
```

Use in templates: `{{ EDITOR }}` and `{{ git.email }}`

📖 **[Learn more about Variables in the Wiki](https://github.com/uroybd/DotR/wiki)**

## Templating Example

```toml
# config file with Tera templates
[user]
name = "{{ git.name }}"
email = "{{ git.email }}"

{% if HOME %}
[paths]
data = "{{ HOME }}/Data"
{% endif %}
```

📖 **[Learn more about Templating in the Wiki](https://github.com/uroybd/DotR/wiki)**

## Actions Example

```toml
[packages.nvim]
src = "dotfiles/nvim"
dest = "~/.config/nvim/"

pre_actions = ["mkdir -p ~/.local/share/nvim"]
post_actions = ["nvim --headless +PluginInstall +qall"]
```

Actions support variable interpolation and run before/after deployment.

📖 **[Learn more about Actions in the Wiki](https://github.com/uroybd/DotR/wiki)**

## Prompts Example

```toml
# Config-level (global)
[prompts]
GIT_EMAIL = "Enter your git email"

# Package-level
[packages.aws]
[packages.aws.prompts]
AWS_ACCESS_KEY = "Enter AWS access key"

# Profile-level
[profiles.work]
[profiles.work.prompts]
WORK_EMAIL = "Enter work email"
```

Prompts are asked once on first deploy, saved to `.uservariables.toml` (gitignored).

📖 **[Learn more about Prompts in the Wiki](https://github.com/uroybd/DotR/wiki)**

## Profiles Example

```toml
[profiles.work]
dependencies = ["nvim", "git"]

[profiles.work.variables]
GIT_EMAIL = "work@company.com"

[profiles.home]
dependencies = ["nvim", "gaming"]

[profiles.home.variables]
GIT_EMAIL = "personal@email.com"
```

Deploy with: `dotr deploy --profile work`

📖 **[Learn more about Profiles in the Wiki](https://github.com/uroybd/DotR/wiki)**

## Diff Command

```bash
# Preview changes before deployment
dotr diff

# Diff specific packages
dotr diff --packages bashrc,nvim

# Diff with profile
dotr diff --profile work
```

Shows line-by-line differences with color coding (+ green for additions, - red for deletions).

📖 **[Learn more about Diff in the Wiki](https://github.com/uroybd/DotR/wiki)**

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
  init        Initialize dotfiles repository.
  import      Import dotfile and update configuration.
  deploy      Deploy dotfiles from repository.
  update      Update dotfiles to repository.
  diff        Show differences between deployed and repository files.
  print-vars  Print all user variables.
  help        Print this message or the help of the given subcommand(s)

Options:
  -w, --working-dir <WORKING_DIR>  Specify working directory
  -h, --help                       Print help

Profile Support:
  Most commands support the --profile flag to use profile-specific settings:
  
  dotr deploy --profile work       Deploy with work profile
  dotr import ~/.bashrc --profile home
  dotr update --profile server
  dotr diff --profile work         Show differences with profile variables
  dotr print-vars --profile work   Show variables with profile applied
```

## TODO
- [x] Import configs
- [x] Copy configs
- [x] Update configs
- [x] Variables (with nested structures)
- [x] Templating (Tera engine)
- [x] Actions (pre/post hooks)
- [x] Profiles (environment-specific configs)
- [x] Diff command (preview changes)
- [x] Granular copying and backups
- [x] Interactive prompts (config/package/profile level)
- [ ] Symlinking config
