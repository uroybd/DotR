# Platforms

A **platform** is a named bundle of settings shared by every
[profile](./profiles.md) that opts into it. Where a profile describes one
machine, a platform describes something several machines have in common —
"this is a macOS box", "this is a Linux box" — so the settings that follow
from that don't have to be repeated under each profile.

```toml
[platforms.macos]
variables = { EDITOR = "vim", CLIPBOARD = "pbcopy" }

[platforms.linux]
variables = { EDITOR = "nano", CLIPBOARD = "xclip -selection clipboard" }

[profiles.laptop]
platform = "macos"

[profiles.work]
platform = "macos"

[profiles.server]
platform = "linux"
```

Both `laptop` and `work` now resolve `EDITOR` to `vim` and `CLIPBOARD` to
`pbcopy` without either profile spelling it out; `server` gets the Linux
values.

## Opting a profile in

A profile joins a platform by setting its `platform` field to the
platform's name:

```toml
[profiles.work]
platform = "macos"
```

This is the same `platform` field that shares a package's
[`targets`](./packages.md#sharing-a-target-across-profiles-by-platform)
destination across profiles — a profile's `platform` value now does both
jobs at once: it shares platform variables *and* shares platform-keyed
`targets` entries.

The `platform` name is just a string you choose. `macos` and `linux` are
the obvious ones, but `wsl`, `headless`, `corp`, or anything else works
just as well — DotR never inspects the actual operating system, it only
matches the string.

A profile can set `platform` without a matching `[platforms.<name>]` table
existing. That's not an error: the profile simply picks up no platform
variables (platform-keyed `targets` still work). This is why you can use
`platform` purely for `targets` sharing and never declare a `[platforms]`
table at all.

## What a platform can hold

| Field       | Type  | Purpose                                                                 |
| ----------- | ----- | ---------------------------------------------------------------------- |
| `variables` | table | Variables shared by every profile whose `platform` names this platform — see [Variables](./variables.md). |

`variables` is the only field today. Nested tables and arrays are
supported, exactly as in any other `[*.variables]` block.

## Where platform variables sit in the priority order

Platform variables are, in effect, the platform-specific slice of your
config-level variables — so they sit just above config-level and
environment variables, and below anything more specific:

```text
user variables  >  profile variables  >  package variables  >  platform variables  >  environment variables  >  config variables
```

In practice:

- A `[platforms.macos]` variable overrides a `[variables]` entry of the
  same name (and a same-named environment variable) whenever a macOS
  profile is active.
- A profile's own `[profiles.<name>.variables]` still wins over the
  platform's — the profile is the more specific place.
- A package's `[packages.<name>.variables]` also wins over the platform's.

See [Variables § Priority](./variables.md#priority) for the full picture,
and run `dotr print-vars --profile <name>` to see the resolved result —
platform variables appear there like any other.

## Resolution timing

A profile's platform variables are resolved once, when `config.toml` is
parsed, because a profile's `platform` is fixed for the life of the
config. Editing `[platforms.<name>.variables]` and re-running any command
picks up the change; there is nothing to invalidate or refresh.
