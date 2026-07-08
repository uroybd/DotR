# Actions

Actions are shell commands run around a package's deployment — useful for
installing dependencies, reloading a service, fixing permissions, or
anything else that isn't just "put this file here."

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

## Dry run

Under `--dry-run`, actions are not executed — each one is printed as
`(Dry Run) Would execute action: <command>` instead. See
[Dry Run Mode](./dry-run.md).
