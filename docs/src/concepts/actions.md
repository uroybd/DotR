# Actions

Actions are shell commands run around deployment — useful for installing
dependencies, reloading a service, fixing permissions, or anything else
that isn't just "put this file here."

```toml
[packages.nvim]
src = "dotfiles/nvim"
dest = "~/.config/nvim/"

pre_actions = ["mkdir -p ~/.local/share/nvim"]
post_actions = ["nvim --headless +PluginInstall +qall"]
```

- `pre_actions` run **before** the package's files are copied/symlinked.
- `post_actions` run **after**.
- Both are lists — multiple actions run in order, each waiting for the
  previous one to finish.

A [profile](./profiles.md) and a [platform](./platforms.md) can carry
`pre_actions`/`post_actions` too — see
[Profile and platform actions](#profile-and-platform-actions) below.

## Execution details

- Each action string is compiled through [Tera](./templating.md) first, so
  `{{ variable }}` interpolation works exactly like in file templates.
- Actions run via `$SHELL -c "<action>"`, falling back to `/bin/sh` if
  `$SHELL` isn't set.
- The working directory is the repository root (the directory containing
  `config.toml`), not the package's `src`/`dest`.
- If an action exits non-zero, the whole `deploy`/`update` operation fails
  immediately — later actions and the rest of that package's deployment do
  not run, unless `--ignore-errors` was passed (which moves on to the next
  *package*, not the next action within a failed one).

```toml
[packages.aws]
src = "dotfiles/aws"
dest = "~/.aws/"
variables = { PROFILE = "default" }
pre_actions = ["echo 'Using AWS profile: {{ PROFILE }}'"]
```

## Profile and platform actions

`pre_actions` / `post_actions` aren't only a package field. A
[profile](./profiles.md) and a [platform](./platforms.md) can each declare
their own, to run something once around the whole deploy rather than once
per package:

```toml
[platforms.macos]
pre_actions = ["softwareupdate --install-rosetta --agree-to-license || true"]

[profiles.work]
platform = "macos"
pre_actions = ["echo 'Deploying the work profile'"]
post_actions = ["launchctl kickstart -k gui/$(id -u)/com.example.agent"]
```

They nest around the package actions, platform outermost:

```text
platform pre_actions
  profile pre_actions
    for each package:  its pre_actions → files → its post_actions
  profile post_actions
platform post_actions
```

- They run **only on `dotr deploy`** (and `dotr packages deploy`) — never
  on `diff` or `update`.
- They run **only when the deploy actually has packages to deploy.** If
  package selection comes out empty, nothing runs — actions included.
- Everything under [Execution details](#execution-details) applies: Tera
  compilation, `$SHELL -c`, working directory at the repository root, and
  abort-on-non-zero-exit. `--ignore-errors` does **not** cover them — a
  failing profile or platform action always stops the deploy (it has no
  "next package" to skip to).
- The variables in scope are the profile-level set — profile + platform +
  config + user, exactly what `dotr print-vars --profile <name>` reports.
  There is no package scope, since these actions aren't tied to a package.
- The platform's actions come from the `[platforms.<name>]` table named by
  the active profile's [`platform`](./platforms.md) field; a profile with
  no `platform`, or one naming a platform that isn't declared, simply
  contributes no platform actions.

## Skipping actions

`dotr deploy` (and `dotr packages deploy`) accept flags to skip actions for
that invocation without editing `config.toml`:

```bash
# Skip both pre- and post-actions
dotr deploy --skip-actions

# Skip only pre-actions
dotr deploy --skip-pre-actions

# Skip only post-actions
dotr deploy --skip-post-actions
```

This is useful when actions are expensive (e.g. reinstalling plugins) and
you only want to sync files, or when debugging a failing action by first
confirming the file deployment itself is fine.

Each flag applies to every layer at once — package, profile, and platform.
`--skip-actions` suppresses all of them; `--skip-pre-actions` /
`--skip-post-actions` suppress that half everywhere.

## Dry run

Under `--dry-run`, no actions are executed — at any layer. Each one is
printed as `(Dry Run) Would execute action: <command>` instead. See
[Dry Run Mode](./dry-run.md).
