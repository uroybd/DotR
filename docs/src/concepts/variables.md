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
- **User variables** — everything in the gitignored `.uservariables.toml`:
  values you add there by hand, plus answers to
  [prompts](./prompts.md) that use the `file` backend. Prompted answers
  from the `keychain`/`bitwarden` backends are resolved into user
  variables too, at the same priority, but stored elsewhere and never
  written into this file — see
  [Prompts § Where answers go](./prompts.md#where-answers-go-backends).

`DOTR_PROFILE` and `DOTR_BITWARDEN_NOTE` are a special case: whichever
value wins during [profile](./profiles.md#selecting-a-profile) or
[Bitwarden note](./prompts.md#machine-local-override-for-bitwarden_note)
resolution is folded into the config-level tier too, so it's visible like
any other variable — but they're never *user* variables (declaring them
as prompts isn't supported).

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

The gitignored (by `dotr init`), machine-local home for anything that
shouldn't be committed to the repository — secrets, personal emails,
machine-specific paths. Every key in it becomes a user variable, whether
you typed it in yourself or it's a saved prompt answer:

```toml
# .uservariables.toml — edit freely, no [prompts] entry required
EDITOR_OVERRIDE = "hx"
```

Answers to [prompts](./prompts.md) only land here for the `file` backend
(the default). With `keychain` or `bitwarden` configured, an answer is
stored in that backend instead and never appears in this file in
plaintext — but it still resolves into a user variable at the same
priority, so templates and `dotr print-vars` see it either way.
