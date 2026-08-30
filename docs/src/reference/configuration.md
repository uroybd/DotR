# Configuration File Reference

Everything DotR manages is described in a single `config.toml` at the
repository root, created by `dotr init`.

## Editor support (schema validation)

`dotr init` writes a [taplo](https://taplo.tamasfe.dev/) `#:schema`
directive as the first line of the generated `config.toml`:

```toml
#:schema https://raw.githubusercontent.com/uroybd/DotR/main/schema/config.schema.json
```

This associates the file with a [JSON Schema](https://github.com/uroybd/DotR/blob/main/schema/config.schema.json)
describing every field on this page — giving you inline validation,
autocomplete, and hover documentation as you edit, in any editor that uses
taplo as its TOML language server:

- **VS Code** — install the [Even Better TOML](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml)
  extension; it picks up the directive automatically.
- **Neovim** — via [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig)'s
  built-in `taplo` preset: `require('lspconfig').taplo.setup {}`.
- **Vim, Emacs, Helix, Sublime** — any setup that runs `taplo` as the TOML
  language server (via `vim-lsp`/`coc.nvim`, `lsp-mode`, etc.) picks it up
  the same way, since the directive is parsed by taplo itself, not by a
  particular editor plugin.

This isn't published on [SchemaStore](https://www.schemastore.org/) —
`config.toml` is too generic a filename for filename-based catalog
matching (Hugo's own `config.toml` is SchemaStore's canonical example of a
pattern they reject for exactly this reason). The inline directive avoids
that ambiguity entirely, since it points at a specific schema explicitly
rather than relying on the filename.

If you have a `config.toml` from before this was added, add the line
above to the top of the file yourself, or configure your editor's schema
association for the file manually (e.g. `evenBetterToml.schema.associations`
in VS Code settings).

## Top level

```toml
banner = true
symlink = false
prompt_backend = "file"
bitwarden_note = "dotr-secrets"

[variables]
# ...

[prompts]
# ...

[packages.<name>]
# ...

[profiles.<name>]
# ...

[platforms.<name>]
# ...
```

| Field       | Type   | Default | Purpose                                                             |
| ----------- | ------ | ------- | --------------------------------------------------------------------- |
| `banner`    | bool   | `true`  | Print the DotR ASCII banner on commands. Set `false` for quiet output. |
| `symlink`   | bool   | `false` | Deploy every directory package as a symlink, without setting `symlink = true` on each one individually — see [Symlinks](../concepts/symlinks.md#enabling-symlinking-globally). |
| `variables` | table  | `{}`    | Config-level variables — see [Variables](../concepts/variables.md).   |
| `prompts`   | table  | `{}`    | Config-level prompts — see [Prompts](../concepts/prompts.md).         |
| `prompt_backend` | `"file"` \| `"keychain"` \| `"bitwarden"` | unset (behaves as `"file"`) | Repo-wide default storage backend for every prompt — see [Prompts](../concepts/prompts.md#where-answers-go-backends). A profile's own `prompt_backend` overrides this. |
| `bitwarden_note` | string | `"dotr-secrets"` | Name of the Bitwarden secure note used when `prompt_backend = "bitwarden"` — see [Prompts](../concepts/prompts.md#where-answers-go-backends). Can be overridden per-machine via `DOTR_BITWARDEN_NOTE` — see [machine-local override](../concepts/prompts.md#machine-local-override-for-bitwarden_note). |
| `packages`  | table  | `{}`    | Package definitions, keyed by name — see below.                       |
| `profiles`  | table  | `{ default = {} }` | Profile definitions, keyed by name — see below. A `default` profile always exists. |
| `platforms` | table  | `{}`    | Platform definitions, keyed by name — see below and [Platforms](../concepts/platforms.md). A profile whose `platform` field names one picks up its variables. |

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
| `targets`      | table (profile/platform → path) | `{}` | Per-profile (or per-platform) destination override — [Packages](../concepts/packages.md#per-profile-destinations-targets). |
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
prompt_backend = "keychain"
platform = "macos"
pre_actions = ["echo 'Deploying the work profile'"]

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
| `prompt_backend` | `"file"` \| `"keychain"` \| `"bitwarden"` | unset (follows the top-level `prompt_backend`) | Overrides the repo-wide default backend while this profile is active — [Prompts](../concepts/prompts.md#where-answers-go-backends). |
| `bitwarden_note` | string          | unset (follows the top-level `bitwarden_note`) | Overrides which Bitwarden secure note this profile's `bitwarden`-backed prompts use — [Prompts](../concepts/prompts.md#where-answers-go-backends). Can itself be overridden per-machine via `DOTR_BITWARDEN_NOTE` — see [machine-local override](../concepts/prompts.md#machine-local-override-for-bitwarden_note). |
| `platform`     | string          | unset           | Names an entry in the top-level `platforms` table. Shares that platform's `variables`, `pre_actions`/`post_actions`, and a package's `targets` destination with every other profile that sets the same value. A `targets` entry keyed by the profile's own name beats one keyed by its platform, and the profile's own `variables` beat the platform's. — [Platforms](../concepts/platforms.md), [Packages](../concepts/packages.md#sharing-a-target-across-profiles-by-platform). |
| `pre_actions`  | list of strings | `[]`    | Shell commands run once before a `dotr deploy` under this profile, wrapping the per-package actions — [Actions](../concepts/actions.md#profile-and-platform-actions). |
| `post_actions` | list of strings | `[]`    | Shell commands run once after such a deploy — [Actions](../concepts/actions.md#profile-and-platform-actions). |

## `[platforms.<name>]`

```toml
[platforms.macos]
variables = { CLIPBOARD = "pbcopy" }
pre_actions = ["softwareupdate --install-rosetta --agree-to-license || true"]

[platforms.linux]
variables = { CLIPBOARD = "xclip -selection clipboard" }
```

Keyed by platform name — a free-form string (`macos`, `linux`, `wsl`, …);
DotR matches it against profiles' `platform` fields and never inspects the
running OS. A profile whose `platform` names a platform here inherits its
variables and actions; several profiles can name the same platform and all
inherit them. Naming a platform that isn't declared here is allowed and
simply yields nothing.

| Field          | Type            | Default | Purpose                                                                 |
| -------------- | --------------- | ------- | ---------------------------------------------------------------------- |
| `variables`    | table           | `{}`    | Variables shared by every profile naming this platform. Override config-level and environment variables; overridden by package, profile, and user variables — [Variables](../concepts/variables.md#priority), [Platforms](../concepts/platforms.md). |
| `pre_actions`  | list of strings | `[]`    | Shell commands run once before a `dotr deploy` for any profile on this platform — outermost, wrapping the profile's own actions — [Actions](../concepts/actions.md#profile-and-platform-actions). |
| `post_actions` | list of strings | `[]`    | Shell commands run once after such a deploy — [Actions](../concepts/actions.md#profile-and-platform-actions). |

## Other files DotR creates

| File                     | Tracked in git? | Purpose                                                                 |
| ------------------------ | ---------------- | ------------------------------------------------------------------------ |
| `config.toml`            | Yes              | The configuration described above.                                     |
| `dotfiles/`              | Yes              | Package sources — the actual file/directory content for each package.  |
| `.gitignore`             | Yes              | Written by `dotr init`; excludes `.uservariables.toml` and `deployed`. |
| `.uservariables.toml`    | **No**           | Answers to [prompts](../concepts/prompts.md) — secrets live here.      |
| `deployed/`              | **No**           | Staging directory for [symlinked](../concepts/symlinks.md) packages.   |
