# Packages

A **package** is a single managed unit — one file or one directory — with a
source inside the repository and a destination on disk. Every package lives
under `[packages.<name>]` in `config.toml`.

```toml
[packages.nvim]
src = "dotfiles/nvim"
dest = "~/.config/nvim/"
```

- `src` is relative to the repository root (the directory containing
  `config.toml`).
- `dest` is the deployment target. `~` is expanded to the user's home
  directory.

## Creating packages

Packages are usually created with `dotr import`, which copies a file or
directory into `dotfiles/` and writes the corresponding `[packages.*]`
entry:

```bash
dotr import ~/.bashrc
dotr import ~/.config/nvim/ --name neovim
```

The package name defaults to a sanitized version of the path (leading `.`
stripped, `.`/`-` replaced with `_`, directories prefixed with `d_`, files
with `f_`), or you can set it explicitly with `--name`.

## Fields

| Field          | Type                  | Purpose                                                                 |
| -------------- | --------------------- | ------------------------------------------------------------------------ |
| `src`          | string                | Path to the file/directory in the repository                            |
| `dest`         | string                | Deployment destination                                                  |
| `dependencies` | list of strings       | Other packages that must be deployed alongside this one — see [Dependencies](./dependencies.md) |
| `variables`    | table                 | Package-scoped variables — see [Variables](./variables.md)              |
| `pre_actions`  | list of strings       | Shell commands run before deploying — see [Actions](./actions.md)       |
| `post_actions` | list of strings       | Shell commands run after deploying — see [Actions](./actions.md)        |
| `targets`      | table (profile/platform → path)| Per-profile (or per-platform) destination override, see below  |
| `skip`         | bool                  | If `true`, excluded from profile-driven deploys (see below)             |
| `prompts`      | table                 | Package-scoped prompts — see [Prompts](./prompts.md)                    |
| `ignore`       | list of glob patterns | Files to exclude from deployment/cleaning — see [Ignoring Files](./ignoring-files.md) |
| `symlink`      | bool                  | Deploy as a symlink instead of a copy; overrides the global `symlink` setting either way when set explicitly — see [Symlinks](./symlinks.md#opting-a-package-out) |
| `unfold_symlink` | bool                | Symlink individual files rather than the whole directory — see [Symlinks](./symlinks.md#unfolding-per-file-symlinks) |
| `clean`        | bool (default `true`) | Remove stray files in the destination — see [Clean Mode](./clean-mode.md) |

## Per-profile destinations (`targets`)

A package can deploy to a different path depending on the active profile:

```toml
[packages.gitconfig]
src = "dotfiles/gitconfig"
dest = "~/.gitconfig"

[packages.gitconfig.targets]
work = "~/.gitconfig-work"
```

When the `work` profile is active, `gitconfig` deploys to
`~/.gitconfig-work` instead of the default `dest`. Any profile not listed in
`targets` falls back to `dest`.

### Sharing a target across profiles by platform

`targets` can also be keyed by a profile's
[`platform`](./profiles.md) value instead of its name, so multiple profiles
that set the same `platform` share one override without repeating it under
each profile's own name:

```toml
[profiles.home]
platform = "macos"

[profiles.work]
platform = "macos"

[packages.gitconfig.targets]
macos = "~/.gitconfig-mac"
```

Both `home` and `work` deploy `gitconfig` to `~/.gitconfig-mac`. If a
package's `targets` has entries for both a profile's own name *and* its
platform, the profile-name entry wins — it's the more specific override.

## Skipping a package by default (`skip`)

```toml
[packages.experimental]
src = "dotfiles/experimental"
dest = "~/.config/experimental"
skip = true
```

A package with `skip = true` is left out when packages are selected
implicitly (via a profile, i.e. no `--packages` flag). It's still deployed
if you name it explicitly: `dotr deploy --packages experimental`.

## Operating on packages directly

The `packages` subcommand groups `import`/`deploy`/`update`/`diff`/`remove`/
`list` under one namespace — each behaves the same as its top-level
equivalent (`dotr import`, `dotr deploy`, ... `dotr remove`):

```bash
dotr packages list
dotr packages list --verbose
dotr packages import ~/.tmux.conf
dotr packages deploy --packages nvim
dotr packages update --packages nvim
dotr packages diff --packages nvim
dotr packages remove nvim
dotr packages remove nvim --remove-orphans
```

`remove` (equivalently, top-level `dotr remove`) deletes the package's entry
from `config.toml` and its files from `dotfiles/`. It refuses to remove a
package that other packages or profiles still depend on unless you pass
`--force`. `--remove-orphans` additionally removes any of its dependencies
that are no longer referenced elsewhere.
