# Introduction

DotR is a dotfiles manager that is as dear as a daughter.

It keeps a repository of your configuration files ("dotfiles") separate from
where they're actually used on disk, and manages copying — or symlinking —
them into place. The repository is the source of truth; your `~/.bashrc`,
`~/.config/nvim/`, and friends are deployments of it.

## Why DotR?

- **Plain files, not magic.** Your dotfiles repository is just files and
  directories under `dotfiles/`, described by a single `config.toml`.
- **Copy or symlink, your choice.** Deploy as copies for a snapshot-based
  workflow, or as symlinks for live-editing.
- **Environment-aware.** [Profiles](./concepts/profiles.md) let the same
  repository describe different machines — work, home, server — with
  different packages, variables, and destinations.
- **Templated when you need it.** Config files can embed
  [Tera](https://keats.github.io/tera/) template syntax, compiled at deploy
  time with live variables, so the same template can render differently per
  machine or profile.
- **Hooks for the messy parts.** [Pre/post actions](./concepts/actions.md)
  run shell commands around deployment — installing dependencies, reloading a
  service, fixing permissions.
- **Safe by default.** [Dry-run mode](./concepts/dry-run.md) previews every
  operation, [diff](./reference/cli.md#dotr-diff) shows exactly what would
  change, and deploys only touch files that actually changed.

## How it fits together

1. You **import** an existing file or directory into the repository:
   `dotr import ~/.bashrc`. DotR copies it into `dotfiles/` and registers it
   as a *package* in `config.toml`.
2. On a new machine, you **deploy**: `dotr deploy`. DotR copies (or
   symlinks) every package from the repository to its destination.
3. After editing a deployed file, you **update**:
   `dotr update`. DotR copies the changed file back into the repository, so
   `dotfiles/` always reflects what's actually deployed.

Everything else in this book — profiles, variables, templating, actions,
prompts, symlinks, clean mode, ignoring files, dependencies — builds on that
loop.

## Where to go next

- New to DotR? Start with [Installation](./getting-started/installation.md)
  and [Quick Start](./getting-started/quick-start.md).
- Looking for a specific flag or config field? Jump straight to the
  [CLI Reference](./reference/cli.md) or
  [Configuration File Reference](./reference/configuration.md).
