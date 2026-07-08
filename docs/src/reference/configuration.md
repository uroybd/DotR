# Configuration File Reference

Everything DotR manages is described in a single `config.toml` at the
repository root, created by `dotr init`.

## Top level

```toml
banner = true
symlink = false

[variables]
# ...

[prompts]
# ...

[packages.<name>]
# ...

[profiles.<name>]
# ...
```

| Field       | Type   | Default | Purpose                                                             |
| ----------- | ------ | ------- | --------------------------------------------------------------------- |
| `banner`    | bool   | `true`  | Print the DotR ASCII banner on commands. Set `false` for quiet output. |
| `symlink`   | bool   | `false` | Deploy every directory package as a symlink, without setting `symlink = true` on each one individually — see [Symlinks](../concepts/symlinks.md#enabling-symlinking-globally). |
| `variables` | table  | `{}`    | Config-level variables — see [Variables](../concepts/variables.md).   |
| `prompts`   | table  | `{}`    | Config-level prompts — see [Prompts](../concepts/prompts.md).         |
| `packages`  | table  | `{}`    | Package definitions, keyed by name — see below.                       |
| `profiles`  | table  | `{ default = {} }` | Profile definitions, keyed by name — see below. A `default` profile always exists. |

## `[packages.<name>]`

```toml
[packages.nvim]
src = "dotfiles/nvim"
dest = "~/.config/nvim/"
dependencies = ["fonts"]
pre_actions = ["mkdir -p ~/.local/share/nvim"]
post_actions = ["nvim --headless +PluginInstall +qall"]
skip = false
symlink = false
unfold_symlink = false
clean = true
ignore = ["*.log"]

[packages.nvim.variables]
THEME = "gruvbox"

[packages.nvim.targets]
work = "~/work-config/nvim/"

[packages.nvim.prompts]
NVIM_TOKEN = "Enter your plugin registry token"
```

| Field          | Type                    | Default | Purpose                                                        |
| -------------- | ------------------------ | ------- | ------------------------------------------------------------------ |
| `src`          | string                   | —       | Path to the file/directory in the repository. **Required.**      |
| `dest`         | string                   | —       | Deployment destination. `~` and template variables are expanded. **Required.** |
| `dependencies` | list of strings          | none    | Other packages deployed alongside this one — [Dependencies](../concepts/dependencies.md). |
| `variables`    | table                    | `{}`    | Package-scoped variables — [Variables](../concepts/variables.md). |
| `pre_actions`  | list of strings          | `[]`    | Shell commands run before deploy — [Actions](../concepts/actions.md). |
| `post_actions` | list of strings          | `[]`    | Shell commands run after deploy — [Actions](../concepts/actions.md). |
| `targets`      | table (profile → path)   | `{}`    | Per-profile destination override — [Packages](../concepts/packages.md#per-profile-destinations-targets). |
| `skip`         | bool                     | `false` | Excluded from profile-driven (implicit) selection — [Packages](../concepts/packages.md#skipping-a-package-by-default-skip). |
| `prompts`      | table                    | `{}`    | Package-scoped prompts — [Prompts](../concepts/prompts.md). |
| `ignore`       | list of glob patterns    | `[]`    | Files excluded from deploy/clean — [Ignoring Files](../concepts/ignoring-files.md). |
| `symlink`      | bool                     | unset (follows the global `symlink` setting) | Deploy as a symlink instead of a copy. An explicit `true`/`false` here always overrides the global flag — [Symlinks](../concepts/symlinks.md#opting-a-package-out). |
| `unfold_symlink` | bool                   | `false` | Symlink individual files instead of the whole directory, so untracked content can coexist at `dest` — implied by a non-empty `ignore` — [Symlinks](../concepts/symlinks.md#unfolding-per-file-symlinks). |
| `clean`        | bool                     | `true`  | Remove stray files at the destination — [Clean Mode](../concepts/clean-mode.md). |

## `[profiles.<name>]`

```toml
[profiles.work]
dependencies = ["nvim", "git"]

[profiles.work.variables]
GIT_EMAIL = "work@company.com"

[profiles.work.prompts]
WORK_TOKEN = "Enter your work VPN token"
```

| Field          | Type            | Default | Purpose                                                         |
| -------------- | --------------- | ------- | -------------------------------------------------------------- |
| `dependencies` | list of strings | `[]`    | Packages deployed when this profile is active and no `--packages` is given — [Profiles](../concepts/profiles.md). |
| `variables`    | table           | `{}`    | Profile-scoped variables — [Variables](../concepts/variables.md). |
| `prompts`      | table           | `{}`    | Profile-scoped prompts — [Prompts](../concepts/prompts.md). |

## Other files DotR creates

| File                     | Tracked in git? | Purpose                                                                 |
| ------------------------ | ---------------- | ------------------------------------------------------------------------ |
| `config.toml`            | Yes              | The configuration described above.                                     |
| `dotfiles/`              | Yes              | Package sources — the actual file/directory content for each package.  |
| `.gitignore`             | Yes              | Written by `dotr init`; excludes `.uservariables.toml` and `deployed`. |
| `.uservariables.toml`    | **No**           | Answers to [prompts](../concepts/prompts.md) — secrets live here.      |
| `deployed/`              | **No**           | Staging directory for [symlinked](../concepts/symlinks.md) packages.   |
