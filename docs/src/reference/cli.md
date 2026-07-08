# CLI Reference

```
dotr [OPTIONS] [COMMAND]
```

## Global options

| Flag                       | Description                                                        |
| --------------------------- | -------------------------------------------------------------------- |
| `-w, --working-dir <PATH>`  | Run as if invoked from `<PATH>` instead of the current directory. Accepted by every command. |
| `-h, --help`                | Print help for the current command.                                |
| `-V, --version`              | Print the `dotr` version (top-level only).                        |

## `dotr init`

Initialize a dotfiles repository in the working directory: writes
`config.toml`, creates `dotfiles/`, and writes a `.gitignore`. Safe to
re-run — if `config.toml` already exists, it's left untouched.

## `dotr import <PATH>`

Copy a file or directory into the repository and register it as a package.

| Flag                    | Description                                                                    |
| ------------------------ | -------------------------------------------------------------------------------- |
| `-s, --symlink`          | Deploy as a symlink instead of a copy, and deploy immediately. See [Symlinks](../concepts/symlinks.md). |
| `-n, --name <NAME>`      | Override the auto-derived package name.                                        |
| `-p, --profile <NAME>`   | Add the package to this profile's `dependencies` instead of `default`.         |

## `dotr deploy`

Deploy packages from the repository to their destinations.

| Flag                        | Description                                                                 |
| ---------------------------- | ------------------------------------------------------------------------------ |
| `-p, --packages <NAMES>...`  | Deploy only these packages (plus their [dependencies](../concepts/dependencies.md), unless `--ignore-dependencies`). Omit to deploy the active profile's packages. |
| `-P, --profile <NAME>`       | Use this profile instead of the resolved default. See [Profiles](../concepts/profiles.md). |
| `--ignore-errors`             | Keep deploying remaining packages if one fails, instead of aborting.        |
| `--clean <true\|false>`       | Override the [clean mode](../concepts/clean-mode.md) setting for this run.  |
| `--dry-run`                   | Preview without changing anything — see [Dry Run Mode](../concepts/dry-run.md). |
| `--skip-actions`               | Skip both pre- and post-actions. See [Actions](../concepts/actions.md#skipping-actions). |
| `--skip-pre-actions`           | Skip only pre-actions.                                                     |
| `--skip-post-actions`          | Skip only post-actions.                                                    |
| `--ignore-dependencies`        | Deploy only the named/selected packages, without pulling in dependencies. See [Dependencies](../concepts/dependencies.md#skipping-dependency-resolution). |

## `dotr update`

Copy changes from deployed files back into the repository.

| Flag                        | Description                                                                 |
| ---------------------------- | ------------------------------------------------------------------------------ |
| `-p, --packages <NAMES>...`  | Update only these packages. Omit for the active profile's packages.        |
| `-P, --profile <NAME>`       | Use this profile instead of the resolved default.                          |
| `--ignore-errors`             | Keep updating remaining packages if one fails.                             |
| `--clean <true\|false>`       | Override clean mode for this run.                                          |
| `--dry-run`                   | Preview without changing anything.                                        |

Templated packages are skipped by `update` — see
[Templating](../concepts/templating.md#templated-files-are-never-backed-up).

## `dotr diff`

Show a colored, line-by-line diff between the repository and what's
currently deployed.

| Flag                        | Description                                        |
| ---------------------------- | ----------------------------------------------------- |
| `-p, --packages <NAMES>...`  | Diff only these packages.                          |
| `-P, --profile <NAME>`       | Use this profile instead of the resolved default.  |
| `--ignore-errors`             | Keep diffing remaining packages if one fails.      |

## `dotr remove [PACKAGES]...`

Remove one or more managed packages: deletes their `config.toml` entry and
their files under `dotfiles/`. Equivalent to `dotr packages remove`.

| Flag                  | Description                                                                       |
| ---------------------- | ------------------------------------------------------------------------------------ |
| `-f, --force`          | Remove even if another package or profile still depends on it.                     |
| `--remove-orphans`     | Also remove dependencies that end up unreferenced after this removal. See [Dependencies](../concepts/dependencies.md#removing-a-package-with-dependents). |
| `--dry-run`            | Preview what would be removed.                                                     |
| `-P, --profile <NAME>` | Profile context for dependency checks.                                             |

## `dotr print-vars`

Print every variable resolved for the given (or default) profile — see
[Variables](../concepts/variables.md#viewing-resolved-variables).

| Flag                   | Description                       |
| ----------------------- | ------------------------------------ |
| `-p, --profile <NAME>` | Resolve variables for this profile. |

## `dotr packages`

Groups package-scoped commands under one namespace. Each behaves the same
as its top-level equivalent, plus `list`:

```
dotr packages list [-v|--verbose] [--plain]
dotr packages import <IMPORT_PATH> [-s|--symlink] [-n|--name <NAME>] [-p|--profile <NAME>]
dotr packages deploy [same flags as `dotr deploy`]
dotr packages update [same flags as `dotr update`]
dotr packages diff   [same flags as `dotr diff`]
dotr packages remove [same flags as `dotr remove`]
```

`dotr packages` itself also accepts `-P, --profile <NAME>` as context for
its subcommands. `packages list --verbose` additionally prints each
package's `src`, `dest`, dependencies, and other fields. `packages list
--plain` prints just the bare package names, one per line, with no banner
and no other output — meant for scripting and shell completion (see
[Shell completions](../getting-started/installation.md#shell-completions)).
`--plain` and `--verbose` are mutually exclusive.

## `dotr profiles`

Manage profiles — see [Profiles](../concepts/profiles.md).

```
dotr profiles list [-v|--verbose] [--plain]
dotr profiles add <PROFILE_NAME> [--set-as-current]
dotr profiles remove <PROFILE_NAME> [--dry-run] [--remove-orphans]
```

`profiles list --plain` behaves the same way as `packages list --plain`,
printing bare profile names only.

- `add --set-as-current` writes `DOTR_PROFILE` into `.uservariables.toml`,
  making the new profile the implicit default on this machine.
- `remove` cannot remove the `default` profile. `--remove-orphans` also
  removes packages that were only referenced by the removed profile.

## `dotr completions <SHELL>`

Prints a shell completion script to stdout, covering every command shown
on this page — including the nested `packages` and `profiles`
subcommands. `SHELL` is one of `bash`, `zsh`, `fish`, `elvish`,
`powershell`, `nushell`, `carapace`.

```bash
dotr completions bash > ~/.local/share/bash-completion/completions/dotr
dotr completions zsh > ~/.zfunc/_dotr
dotr completions fish > ~/.config/fish/completions/dotr.fish
dotr completions nushell > ~/.config/nushell/completions/dotr.nu
dotr completions carapace > ~/.config/carapace/specs/dotr.yaml
```

The script always reflects whatever version of `dotr` generated it —
regenerate it after upgrading if a new command doesn't show up in
completions yet.

`carapace` is not a shell — it prints a spec file for the
[Carapace](https://carapace.sh) completion engine, which adds dynamic
completion of real package/profile names on top of the static commands
and flags. See [Dynamic package/profile name
completion](../getting-started/installation.md#dynamic-packageprofile-name-completion-carapace).
