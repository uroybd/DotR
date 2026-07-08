# Prompts

Prompts ask for a value interactively the first time it's needed, then
remember the answer — so secrets and machine-specific values never have to
be hard-coded in `config.toml`.

```toml
# Config-level (global)
[prompts]
GIT_EMAIL = "Enter your git email"

# Package-level
[packages.aws]
[packages.aws.prompts]
AWS_ACCESS_KEY = "Enter AWS access key"

# Profile-level
[profiles.work]
[profiles.work.prompts]
WORK_EMAIL = "Enter work email"
```

Each entry maps a variable name to the message shown when prompting for it.

## When prompts run

Prompts are collected — from config-level, the active profile, and every
package being operated on — and checked before `deploy`, `update`, and
`diff`. Any key not already present in `.uservariables.toml` triggers an
interactive prompt; the rest are skipped silently.

## Where answers go

Answers are saved to `.uservariables.toml` in the repository root, which
`dotr init` adds to `.gitignore`. Once a key has an answer there, it's never
prompted for again — delete the line (or the whole file) to be asked again.

Values from `.uservariables.toml` become **user variables** — the
highest-priority source in variable resolution, overriding profile,
package, environment, and config variables. See
[Variables](./variables.md#priority).
