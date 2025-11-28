# DotR


[![Codecov](https://img.shields.io/codecov/c/github/uroybd/dotr?style=for-the-badge&label=Coverage&link=https%3A%2F%2Fapp.codecov.io%2Fgithub%2Furoybd%2FDotR&link=https%3A%2F%2Fapp.codecov.io%2Fgithub%2Furoybd%2FDotR)](https://app.codecov.io/github/uroybd/DotR)
[![Deps.rs Crate Dependencies (latest)](https://img.shields.io/deps-rs/dotr-dear/latest?style=for-the-badge)](https://deps.rs/crate/dotr-dear/)
[![GitHub License](https://img.shields.io/github/license/uroybd/dotr?style=for-the-badge)](https://github.com/uroybd/DotR/blob/main/LICENSE)

---
[![GitHub Release](https://img.shields.io/github/v/release/uroybd/dotr?style=for-the-badge&link=https%3A%2F%2Fgithub.com%2Furoybd%2FDotR%2Freleases%2Flatest&link=https%3A%2F%2Fgithub.com%2Furoybd%2FDotR%2Freleases%2Flatest)](https://github.com/uroybd/DotR/releases/latest)
![GitHub Downloads (all assets, all releases)](https://img.shields.io/github/downloads/uroybd/dotr/total?style=for-the-badge&label=Github%20Downloads)
[![Crates.io Total Downloads](https://img.shields.io/crates/d/dotr-dear?style=for-the-badge&label=Crates.io%20Downloads)](https://crates.io/crates/dotr-dear)

A dotfiles manager that is as dear as a daughter.

> [!WARNING]
> This is still in beta. The schema is evolving, performance is sub-par. Use it with caution.

## Documentation

For detailed documentation, guides, and examples, visit the [DotR Wiki](https://github.com/uroybd/DotR/wiki).

## Features

### 📦 Package Management
- **Import dotfiles** from any location into your repository
- **Import with symlinks** - use `--symlink` flag to deploy as symlinks instead of copies
- **Deploy dotfiles** to their target locations
- **Update changes** back to your repository
- **Remove packages** - clean up managed packages with `dotr packages remove`
- Support for both **files and directories**
- **Profile-based deployment** for different environments (work, home, server)
- **Profile dependencies** to automatically deploy required packages
- **Package targets** to override destinations per profile

### 🎭 Profiles
- **Environment-specific configurations** (work, home, server, laptop, etc.)
- **Add profiles** - create new profiles with `dotr profiles add`
- **Remove profiles** - delete profiles with `dotr profiles remove`
- **List profiles** - view all available profiles with `dotr profiles list`
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
- **Symlink support** - deploy as symlinks for live-editing workflows
- **Granular file deployment** - only deploys files when content has changed
- **Granular backups** - creates per-file backups (`.dotrbak`) instead of directory backups
- **Diff command** to preview changes before deployment
- **Dry run mode** - preview deploy and update operations without making any changes
- **Remove orphans** - clean up packages/profiles and their orphaned dependencies
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

# Import with symlink (deploy as symlink instead of copy)
dotr import ~/.config/nvim/ --symlink

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

# Dry run to preview what would be deployed without making changes
dotr deploy --dry-run
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

# Dry run to preview what would be updated without making changes
dotr update --dry-run
```

6. **Manage packages and profiles**:
```bash
# List all packages
dotr packages list

# List packages with verbose details
dotr packages list --verbose

# Remove a package
dotr packages remove nvim

# Remove package and its orphaned dependencies
dotr packages remove nvim --remove-orphans

# List all profiles
dotr profiles list

# List profiles with verbose details
dotr profiles list --verbose

# Add a new profile
dotr profiles add laptop

# Remove a profile
dotr profiles remove work

# Remove profile and orphaned packages
dotr profiles remove work --remove-orphans
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

📖 **[Learn more about Variables](https://github.com/uroybd/DotR/wiki/Configuration#variables)**

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

📖 **[Learn more about Templating](https://github.com/uroybd/DotR/wiki/Templating)**

## Actions Example

```toml
[packages.nvim]
src = "dotfiles/nvim"
dest = "~/.config/nvim/"

pre_actions = ["mkdir -p ~/.local/share/nvim"]
post_actions = ["nvim --headless +PluginInstall +qall"]
```

Actions support variable interpolation and run before/after deployment.

📖 **[Learn more about Actions](https://github.com/uroybd/DotR/wiki/Actions)**

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

📖 **[Learn more about Prompts](https://github.com/uroybd/DotR/wiki/Configuration#prompts)**

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

📖 **[Learn more about Profiles](https://github.com/uroybd/DotR/wiki/Profiles)**

## Symlink Support

Import and deploy dotfiles as symlinks instead of copies for live-editing workflows:

```bash
# Import with symlink
dotr import ~/.config/nvim/ --symlink

# Files are deployed to a 'deployed' directory and symlinked to their destinations
# Edit files at ~/.config/nvim/ and changes immediately reflect in the repository
```

**How it works:**
1. Files are copied from source to `dotfiles/nvim/` (your repository)
2. During deployment, files are copied to `deployed/nvim/` 
3. A symlink is created from `~/.config/nvim/` → `deployed/nvim/`
4. Edit at the symlink location, changes are immediately in `deployed/`, ready to update back to `dotfiles/`

**Configuration:**
```toml
[packages.nvim]
src = "dotfiles/nvim"
dest = "~/.config/nvim/"
symlink = true  # Enable symlink deployment
```

📖 **[Learn more about Symlinks](https://github.com/uroybd/DotR/wiki/Symlinks)**

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

📖 **[Learn more about Diff](https://github.com/uroybd/DotR/wiki/Diff)**

## Dry Run Mode

```bash
# Preview deploy without making any changes
dotr deploy --dry-run

# Preview update without making any changes
dotr update --dry-run

# Combine with other options
dotr deploy --dry-run --profile work --packages nvim,bashrc
dotr update --dry-run --clean
```

Dry run mode shows what would happen during deploy or update operations without:
- Creating or modifying any files
- Creating backups
- Executing pre/post actions
- Removing files (when using --clean)

## Installation

### Homebrew (macOS and Linux)

Supports both Apple Silicon and Intel Macs.

```bash
brew tap uroybd/tap
brew install dotr
```

### Cargo

```bash
cargo install dotr-dear
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
- **Linux (aarch64)**: `dotr-aarch64-unknown-linux-gnu.tar.gz `
- **Linux Muslc (x86_64)**: `dotr-x86_64-unknown-linux-musl.tar.gz`
- **Linux Muslc (aarch64)**: `dotr-aarch64-unknown-linux-musl.tar.gz`


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
  update      Update dotfiles from deployed versions.
  diff        Show differences between dotfiles.
  print-vars  Print all user variables.
  packages    Manage packages (list, import, deploy, update, remove, diff).
  profiles    Manage profiles (list, add, remove).
  help        Print this message or the help of the given subcommand(s)

Options:
  -w, --working-dir <WORKING_DIR>
  -h, --help                       Print help
  -V, --version                    Print version

```

### Package Commands
```bash
# List packages
dotr packages list [OPTIONS]
  --verbose          # Show detailed package information

# Remove packages
dotr packages remove <PACKAGE>... [OPTIONS]
  --force            # Force removal without confirmation
  --remove-orphans   # Remove orphaned dependencies
  --dry-run          # Preview what would be removed
```

### Profile Commands
```bash
# List profiles
dotr profiles list [OPTIONS]
  --verbose          # Show detailed profile information

# Add a profile
dotr profiles add <PROFILE_NAME>

# Remove a profile
dotr profiles remove <PROFILE_NAME> [OPTIONS]
  --remove-orphans   # Remove packages only used by this profile
  --dry-run          # Preview what would be removed
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
- [x] Dry run mode (preview deploy/update without changes)
- [x] Symlinking config (import and deploy with symlinks)
- [x] Package management (list, remove with orphan cleanup)
- [x] Profile management (add, remove with orphan cleanup)
