# Quick Start

## 1. Initialize a repository

```bash
dotr init
```

This creates, in the current directory:

- `config.toml` — the repository's configuration (packages, profiles,
  variables, prompts)
- `dotfiles/` — where imported files actually live
- `.gitignore` — pre-populated to exclude `.uservariables.toml` (your local
  secrets) and `deployed` (the symlink staging directory, see
  [Symlinks](../concepts/symlinks.md))

Run this inside a git repository you intend to push somewhere, so the
dotfiles themselves are version-controlled.

## 2. Import your existing dotfiles

```bash
dotr import ~/.bashrc
dotr import ~/.config/nvim/

# Import as a symlink instead of a copy (live-editing workflow)
dotr import ~/.config/nvim/ --symlink

# Import into a specific profile
dotr import ~/.ssh/config --profile work
```

Each import copies the file or directory into `dotfiles/`, and registers a
*package* for it in `config.toml` with a source and destination — see
[Packages](../concepts/packages.md).

## 3. Deploy dotfiles on a (new) machine

```bash
# Deploy every package
dotr deploy

# Deploy only packages in the "work" profile
dotr deploy --profile work

# Deploy specific packages by name
dotr deploy --packages nvim,tmux

# Preview what would happen, without touching disk
dotr deploy --dry-run
```

## 4. Check what would change before deploying

```bash
dotr diff
dotr diff --packages nvim,bashrc
dotr diff --profile work
```

`diff` shows a colored, line-by-line diff between what's in the repository
and what's currently deployed.

## 5. Pull local edits back into the repository

```bash
dotr update
dotr update --profile work
dotr update --dry-run
```

If you edited a deployed file directly (e.g. tweaked `~/.bashrc` by hand),
`update` copies those changes back into `dotfiles/` so the repository stays
the source of truth.

## 6. Manage packages and profiles

```bash
# Packages
dotr packages list
dotr packages list --verbose
dotr packages remove nvim
dotr packages remove nvim --remove-orphans

# Profiles
dotr profiles list
dotr profiles list --verbose
dotr profiles add laptop
dotr profiles remove work
dotr profiles remove work --remove-orphans
```

## Next steps

- [Profiles](../concepts/profiles.md) for per-machine configuration
- [Variables](../concepts/variables.md) and [Templating](../concepts/templating.md)
  for config files that adapt to their environment
- [Actions](../concepts/actions.md) for shell hooks around deployment
- The full [CLI Reference](../reference/cli.md) and
  [Configuration File Reference](../reference/configuration.md)
