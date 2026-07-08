# Profiles

A **profile** describes an environment — work, home, a particular server —
as a set of packages, variables, and prompts. The same repository can
deploy differently depending on which profile is active.

Every repository has a `default` profile, created automatically by
`dotr init`.

```toml
[profiles.work]
dependencies = ["nvim", "git"]

[profiles.work.variables]
GIT_EMAIL = "work@company.com"

[profiles.home]
dependencies = ["nvim", "gaming"]

[profiles.home.variables]
GIT_EMAIL = "personal@email.com"
```

## Selecting a profile

Most commands accept `-P`/`--profile`:

```bash
dotr deploy --profile work
dotr import ~/.ssh/config --profile work
dotr update --profile work
dotr diff --profile work
```

If no `--profile` is given, DotR resolves the active profile in this order:

1. A `DOTR_PROFILE` variable (from the environment or
   `.uservariables.toml`), if set.
2. Otherwise, `default`.

Referencing a profile that doesn't exist is an error (except `default`,
which is created on the fly if missing).

## Fields

| Field          | Type             | Purpose                                                        |
| -------------- | ---------------- | ---------------------------------------------------------------- |
| `dependencies` | list of strings  | Packages deployed when this profile is active and no `--packages` is given |
| `variables`    | table            | Profile-scoped variables — override package/config/env variables, see [Variables](./variables.md) |
| `prompts`      | table            | Profile-scoped prompts — see [Prompts](./prompts.md)            |

## How a profile decides which packages deploy

When you run `dotr deploy` (or `update`/`diff`) **without** `--packages`,
DotR deploys every package listed in the active profile's `dependencies`
that doesn't have `skip = true` (see [Packages](./packages.md#skipping-a-package-by-default-skip)).

When you pass `--packages` explicitly, the profile's `dependencies` list is
irrelevant to *selection* — only the named packages (and their own
[dependencies](./dependencies.md)) are used — but the profile still supplies
variables, prompts, and any [`targets`](./packages.md#per-profile-destinations-targets)
override.

## Managing profiles

```bash
dotr profiles list
dotr profiles list --verbose
dotr profiles add laptop
dotr profiles add laptop --set-as-current
dotr profiles remove work
dotr profiles remove work --remove-orphans
```

`--set-as-current` writes `DOTR_PROFILE = "laptop"` into
`.uservariables.toml`, so `laptop` becomes the implicit profile on this
machine without needing `--profile laptop` on every command.

`profiles remove` deletes the profile from `config.toml`. `--remove-orphans`
additionally removes any packages that were only referenced by that
profile's `dependencies` and no others.

Removing the `default` profile is not allowed.
