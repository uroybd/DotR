# Variables

Variables are the values available to [templates](./templating.md) and
[actions](./actions.md) — things like `{{ EDITOR }}` or `{{ git.email }}`.

## Sources

Variables come from five places:

- **Config-level** — `[variables]` in `config.toml`, available everywhere.
- **Environment variables** — every variable in your shell environment.
- **Package-level** — `[packages.<name>.variables]`, scoped to that package.
- **Profile-level** — `[profiles.<name>.variables]`, active only when that
  profile is selected.
- **User variables** — answers to [prompts](./prompts.md), saved to the
  gitignored `.uservariables.toml` so secrets never end up in the
  repository.

```toml
[variables]
EDITOR = "nvim"

[variables.git]
name = "Your Name"
email = "you@example.com"
```

Used in a template as `{{ EDITOR }}` and `{{ git.email }}`. Nested tables
and arrays are supported.

## Priority

When the same key is defined in more than one place, the more specific
source wins:

```text
user variables  >  profile variables  >  package variables  >  environment variables  >  config variables
```

In other words: a package's own `[packages.<name>.variables]` can override
`[variables]` in `config.toml` or a same-named environment variable; the
active profile's `[profiles.<name>.variables]` can override the package;
and anything answered via a prompt (stored in `.uservariables.toml`) wins
over all of it.

> Environment variables sit *above* config-level variables but *below*
> package/profile/user variables — if your shell exports a variable with
> the same name as a `[variables]` entry in `config.toml`, the environment
> wins there, but a package or profile can still override it.

## Viewing resolved variables

```bash
dotr print-vars
dotr print-vars --profile work
```

Shows every variable currently resolved for that profile, useful for
debugging why a template rendered the way it did.

## `.uservariables.toml`

Created automatically the first time a [prompt](./prompts.md) is answered,
and listed in `.gitignore` by `dotr init`. It's the right place for
secrets — API tokens, personal emails, machine-specific paths — that
shouldn't be committed to the dotfiles repository.
