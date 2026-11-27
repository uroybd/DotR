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
package being operated on — and checked before any command that reads
variables: `deploy`, `diff`, `print-vars`, and `dump-user-vars` — plus their
`packages` equivalents. `update` checks them only when a selected package's
`dest` (or the `targets` entry used in its place) is templated, or the
package holds a templated file, since those are what it renders. `import`, `packages list`, `remove` and the
`profiles` commands never touch prompts, since none of them reads variables. Any key not already answered (via
whichever [backend](#where-answers-go-backends) is configured) triggers an
interactive prompt; the rest are skipped silently. An answer is saved as typed, without
the line ending Enter adds (spaces you type are kept). Answers saved by dotr
2.4.0 and earlier kept that newline, and a template using one renders it: delete the
key from `.uservariables.toml` (or the keychain or Bitwarden note) to be asked
again.

## Where answers go: backends

By default, answers are saved to `.uservariables.toml` in the repository
root, which `dotr init` adds to `.gitignore`. That's fine for low-stakes
values, but real secrets (API tokens, passwords) then sit in plaintext on
disk indefinitely. Should the file ever stop being ignored (the entry
removed, or the file committed), every command that reads the config warns;
run `dotr init` again to put the entry back. Two other backends are
available:

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
  created automatically the first time it's needed. If a session is
  already open — a valid `BW_SESSION` in the environment — dotr uses it
  without asking for anything, and leaves it open. Otherwise (not logged
  in, the vault locked, or `BW_SESSION` expired) it drives
  `bw login`/`bw unlock` interactively right there in your terminal, and
  locks the vault back up when the command finishes. Either way it syncs
  the latest vault data down before reading or writing, so a note edited
  on another device or the web vault isn't missed.

  To be asked for the master password once per shell rather than once per
  command, unlock it yourself and export the session:

  ```sh
  export BW_SESSION="$(bw unlock --raw)"
  ```

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

Whichever value wins is also folded into [variables](./variables.md) as a
low-priority fallback, so `DOTR_BITWARDEN_NOTE` shows up in
`dotr print-vars` and can be referenced in templates as
`{{ DOTR_BITWARDEN_NOTE }}` — a profile, package, or user variable of the
same name still overrides it. This is separate from `bitwarden_note`
itself (the config-/profile-level setting), which isn't a variable.

Whichever backend is active, the answer becomes a **user variable** — the
highest-priority source in variable resolution, overriding profile,
package, environment, and config variables. See
[Variables](./variables.md#priority). Only file-backend answers ever
appear in `.uservariables.toml`; keychain/Bitwarden values are resolved
fresh each run and never written to the plaintext file. Use
[`dotr dump-user-vars`](../reference/cli.md#dotr-dump-user-vars) to export
every prompted variable — any backend — to a portable TOML file, e.g. to
migrate a value from one backend to another.

## Keeping answers out of the repository

Everything in `.uservariables.toml`, and every answer read from the keychain
or Bitwarden, counts as a secret: `dotr update` never writes one into your
repository as text.

- **`.uservariables.toml` stays ignored.** `dotr init` adds it to
  `.gitignore`, and every command warns if git would commit it.
- **In a templated file**, a secret's value is written back as its tag
  (`{{ API_TOKEN }}`), whether the template used it before or not. See
  [Backing up templated files](./templating.md#backing-up-templated-files).
- **A plain file** (no template tags, text or binary) whose deployed copy
  holds a secret's value is not backed up. You get a warning naming the file,
  the line and the variable, never the value:

  ```
  [WARNING] Not backing up 'dotfiles/f_npmrc': line 3 of '~/.npmrc' holds the value of the secret `NPM_TOKEN`. Put {{ NPM_TOKEN }} in its place in 'dotfiles/f_npmrc' (making it a template, which `update` then merges), or take the value out of the deployed file.
  ```

If you do mean to commit the value, `dotr update --allow-secrets` backs the
file up anyway, with a warning saying so. Values shorter than four
characters are not looked for in plain files, since they turn up by chance.

## Checking before a commit

`update` is not the only way a secret reaches the repository: you can paste
one into a file yourself. `dotr scan` looks for every secret's value in the
repository (in git: every file it tracks or would add; `.uservariables.toml`
and `deployed` are skipped), and `dotr scan --staged` in what is staged for
the next commit. Each finding names the file, the line and the variable,
never the value, and the command exits 1, so it can stop a commit:

```sh
# .git/hooks/pre-commit (make it executable)
#!/bin/sh
exec dotr scan --staged
```

With [pre-commit](https://pre-commit.com), the same as a local hook:

```yaml
repos:
  - repo: local
    hooks:
      - id: dotr-scan
        name: dotr scan
        entry: dotr scan --staged
        language: system
        pass_filenames: false
```

`scan` never prompts: it looks for the answers already saved, in
`.uservariables.toml` and in the keychain or Bitwarden note the profile uses
(a locked Bitwarden vault still asks to be unlocked). `git commit --no-verify`
skips the hook when you do mean to commit a value.

Two limits to know: only a secret's exact text is recognised (not, say, its
base64 encoding), and keychain or Bitwarden answers are only at hand when
`update` loads them, which it does when any package it updates has a
template (a templated file or destination). `.uservariables.toml` is always read.
