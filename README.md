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

## Actions Example

Define pre and post-deployment actions in your `config.toml`:
```toml
[packages.nvim]
src = "dotfiles/nvim"
dest = "~/.config/nvim/"

[packages.nvim.variables]
PLUGIN_MANAGER = "lazy.nvim"

# Actions support variable interpolation
pre_actions = [
    "echo 'Installing {{ PLUGIN_MANAGER }}...'",
    "mkdir -p ~/.local/share/nvim/site/pack"
]

post_actions = [
    "nvim --headless +PluginInstall +qall",
    "echo 'Neovim configuration deployed!'"
]
```

Actions are executed in order and can use all available variables (environment, config, package, profile, and user variables).

## Prompts Example

Define interactive prompts at config, package, or profile level to collect sensitive information on first run:

### Config-Level Prompts

Global prompts for values used across multiple packages:

```toml
# config.toml
[prompts]
GIT_EMAIL = "Enter your git email address"
GIT_NAME = "Enter your full name"
EDITOR = "Enter your preferred editor (vim/nvim/emacs)"
```

### Package-Level Prompts

Package-specific prompts for sensitive configuration:

```toml
[packages.aws-cli]
src = "dotfiles/aws"
dest = "~/.aws/"

[packages.aws-cli.prompts]
AWS_ACCESS_KEY_ID = "Enter your AWS Access Key ID"
AWS_SECRET_ACCESS_KEY = "Enter your AWS Secret Access Key"
AWS_REGION = "Enter your default AWS region (e.g., us-east-1)"

[packages.slack]
src = "dotfiles/slack"
dest = "~/.config/slack/"

[packages.slack.prompts]
SLACK_API_TOKEN = "Enter your Slack API token"
SLACK_WORKSPACE = "Enter your Slack workspace URL"
```

### Profile-Level Prompts

Environment-specific prompts for work, home, etc.:

```toml
[profiles.work]
dependencies = ["aws-cli", "slack", "vpn"]

[profiles.work.prompts]
WORK_EMAIL = "Enter your work email"
VPN_PASSWORD = "Enter your VPN password"
JIRA_TOKEN = "Enter your Jira API token"

[profiles.home]
dependencies = ["personal-git"]

[profiles.home.prompts]
PERSONAL_EMAIL = "Enter your personal email"
GITHUB_TOKEN = "Enter your GitHub personal access token"
```

### How Prompts Work

1. **First Run**: When you deploy, update, or diff, DotR checks all relevant prompts
2. **Interactive Input**: For any variable not in `.uservariables.toml`, you'll see:
   ```
   Enter your AWS Access Key ID
   >>> 
   ```
3. **Saved Automatically**: Your input is saved to `.uservariables.toml` (gitignored by default)
4. **Reuse Values**: On subsequent runs, saved values are reused - no re-prompting!
5. **Hierarchy**: Profile prompts override config prompts, package prompts are merged for deployed packages

### Use Cases

- **API Keys & Tokens**: Keep secrets out of your dotfiles repo
- **Email Addresses**: Different emails for work vs personal profiles
- **Machine-Specific Paths**: Prompt for custom installation directories
- **Credentials**: VPN passwords, database connections, etc.
- **Personal Preferences**: Editor choice, themes, font sizes

## Profiles Example

Define profiles for different environments in your `config.toml`:

```toml
[profiles.work]
dependencies = ["nvim", "git", "ssh"]

[profiles.work.variables]
GIT_EMAIL = "work@company.com"
SSH_KEY = "~/.ssh/id_rsa_work"
NVIM_THEME = "gruvbox"

[profiles.home]
dependencies = ["nvim", "git", "gaming"]

[profiles.home.variables]
GIT_EMAIL = "personal@email.com"
SSH_KEY = "~/.ssh/id_rsa_personal"
NVIM_THEME = "dracula"

# Override package destination for different profiles
[packages.ssh]
src = "dotfiles/ssh"
dest = "~/.ssh/config"

[packages.ssh.targets]
work = "~/.ssh/config.work"
home = "~/.ssh/config.home"

# Skip package unless explicitly deployed or in profile dependencies
[packages.gaming]
src = "dotfiles/gaming"
dest = "~/.config/gaming"
skip = true
```

Deploy with a profile:
```bash
# Deploy work profile - only deploys nvim, git, and ssh
dotr deploy --profile work

# Deploy home profile - deploys nvim, git, and gaming
dotr deploy --profile home

# Print variables for a specific profile
dotr print-vars --profile work
```

Profile features:
- **Dependencies**: Automatically deploy specific packages when using a profile
- **Variables**: Profile-specific variables that override other variable sources
- **Targets**: Deploy the same package to different locations per profile
- **Skip flag**: Mark packages to only deploy when explicitly requested or via profile dependencies

## Diff Command

The `diff` command shows you what changes would be made if you deployed your dotfiles, without actually modifying any files. This is useful for:
- **Previewing changes** before deploying to a new machine
- **Checking what you've modified** locally before updating back to your repository
- **Debugging templating** issues by seeing the compiled output
- **Verifying profile-specific** configurations

### Usage Examples

```bash
# Show differences for all packages
dotr diff

# Show differences for specific packages
dotr diff --packages bashrc,nvim

# Show differences with profile variables applied
dotr diff --profile work
```

### Output Format

The diff command shows a **line-by-line comparison** with color coding:
- **Lines starting with `-`** (in red): Lines that exist in the deployed file but not in the repository
- **Lines starting with `+`** (in green): Lines that exist in the repository but not in the deployed file
- **Lines starting with a space**: Unchanged lines (for context)

Example output:
```diff
[INFO] Diff for package 'f_bashrc':
[INFO] Differences for 'dotfiles/f_bashrc' at '/home/user/.bashrc':
 # Bashrc configuration
-export EDITOR=vim
+export EDITOR=nvim
 export PATH="$HOME/.local/bin:$PATH"
+alias ll='ls -la'
```

### Granular Changes

DotR now uses **granular copying and backups**:
- **Only changed files are deployed** - if a file's content hasn't changed, it won't be copied
- **Per-file backups** - backups are created with `.dotrbak` extension (e.g., `init.lua.dotrbak`, `.bashrc.dotrbak`)
- **Efficient updates** - reduces unnecessary file operations and backup clutter

This means:
1. Running `diff` before `deploy` shows exactly what will be changed
2. Only files that actually differ will be deployed and backed up
3. Unchanged files are skipped entirely, making deployments faster

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
