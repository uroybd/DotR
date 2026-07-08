# Clean Mode

By default, `deploy` and `update` remove files found in the destination
that don't exist in the repository — keeping deployed configs in sync with
`dotfiles/`, rather than merely additive.

```bash
# Deploy with cleaning (default)
dotr deploy

# Deploy without cleaning extra files
dotr deploy --clean=false

# Update with cleaning (default)
dotr update

# Update without cleaning
dotr update --clean=false
```

## What's protected from cleaning

- **Backup files** (`.dotrbak` extension, see below) are never removed.
- **Files matching an [ignore pattern](./ignoring-files.md)** for that
  package are left alone.
- Anything that *is* part of the current deployment, obviously.

## Per-package configuration

```toml
[packages.nvim]
src = "dotfiles/nvim"
dest = "~/.config/nvim/"
clean = false  # disable cleaning for this package specifically
```

`clean` defaults to `true`. The `--clean` CLI flag, when passed, overrides
whatever the package specifies for that one invocation; when omitted, the
package's own setting is used.

## Backups

When a file at the destination would be overwritten, DotR writes a
per-file backup (`<file>.dotrbak`) alongside it before copying — a
lightweight safety net, distinct from `dotr update`'s job of syncing
intentional edits back into the repository. Backup files are always
excluded from cleaning, so they won't be swept away by the same operation
that created them.
