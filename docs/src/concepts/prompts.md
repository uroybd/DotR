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

## Where answers go: backends

By default, answers are saved to `.uservariables.toml` in the repository
root, which `dotr init` adds to `.gitignore`. That's fine for low-stakes
values, but real secrets (API tokens, passwords) then sit in plaintext on
disk indefinitely. Two other backends are available:

```toml
prompt_backend = "keychain"   # or "bitwarden" — repo-wide default

[profiles.work]
prompt_backend = "bitwarden"  # overrides the repo-wide default for this profile
```

- **`file`** (the default) — `.uservariables.toml`, as above.
- **`keychain`** — the OS keychain (macOS Keychain, Windows Credential
  Manager, Linux Secret Service), with entries namespaced per-repository
  so two dotr repos on one machine never collide.
- **`bitwarden`** — a single Bitwarden secure note shared by every
  bitwarden-backed variable in the repository, via the `bw` CLI. The note
  is named by the `bitwarden_note` setting (default `"dotr-secrets"`,
  also overridable per-profile the same way as `prompt_backend`) and
  created automatically the first time it's needed. If `bw` isn't logged
  in or the vault is locked, dotr drives `bw login`/`bw unlock`
  interactively right there in your terminal, syncs the latest vault data
  down before reading or writing (so a note edited on another device or
  the web vault isn't missed), and locks the vault back up when the
  command finishes — but only if *it* was the one that unlocked it; a
  session you already had open (e.g. an exported `BW_SESSION`) is left
  alone.

### Machine-local override for `bitwarden_note`

`bitwarden_note` (config- and profile-level) picks the note by policy —
the same for every machine using that profile. But which note a given
machine should use can differ even *within* the same profile — e.g. a
personal note on your laptop vs. a work note on a work machine sharing
the same `work` profile. For that, set `DOTR_BITWARDEN_NOTE` either as an
environment variable, or as a key in `.uservariables.toml` (so it
persists without exporting it every session — same idea as the
[`DOTR_PROFILE` override](./profiles.md)):

```toml
# .uservariables.toml — gitignored, machine-local
DOTR_BITWARDEN_NOTE = "my-work-laptop-secrets"
```

Resolution order, highest priority first: environment variable →
`.uservariables.toml` → profile's `bitwarden_note` → config's
`bitwarden_note` → built-in default `"dotr-secrets"`.

Whichever backend is active, the answer becomes a **user variable** — the
highest-priority source in variable resolution, overriding profile,
package, environment, and config variables. See
[Variables](./variables.md#priority). Only file-backend answers ever
appear in `.uservariables.toml`; keychain/Bitwarden values are resolved
fresh each run and never written to the plaintext file. Use
[`dotr dump-user-vars`](../reference/cli.md#dotr-dump-user-vars) to export
every prompted variable — any backend — to a portable TOML file, e.g. to
migrate a value from one backend to another.
