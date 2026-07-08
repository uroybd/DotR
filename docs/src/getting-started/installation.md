# Installation

## Homebrew (macOS and Linux)

Supports both Apple Silicon and Intel Macs.

```bash
brew tap uroybd/tap
brew install dotr
```

## Cargo

```bash
cargo install dotr-dear
```

This installs a binary named `dotr` (the crate is published as `dotr-dear`
since `dotr` was already taken on crates.io).

## From source

```bash
cargo install --git https://github.com/uroybd/DotR
```

## Pre-built binaries

Download the latest release for your platform from the
[releases page](https://github.com/uroybd/DotR/releases):

| Platform                    | Archive                                      |
| ---------------------------- | --------------------------------------------- |
| Apple Silicon (M1/M2/M3)     | `dotr-aarch64-apple-darwin.tar.gz`            |
| Intel Mac                    | `dotr-x86_64-apple-darwin.tar.gz`             |
| Linux (x86_64)               | `dotr-x86_64-unknown-linux-gnu.tar.gz`        |
| Linux (aarch64)              | `dotr-aarch64-unknown-linux-gnu.tar.gz`       |
| Linux musl (x86_64)          | `dotr-x86_64-unknown-linux-musl.tar.gz`       |
| Linux musl (aarch64)         | `dotr-aarch64-unknown-linux-musl.tar.gz`      |

Extract and move the binary onto your `PATH`:

```bash
tar xzf dotr-*.tar.gz
sudo mv dotr /usr/local/bin/
```

## Man pages

`man dotr` (and `man dotr-deploy`, `man dotr-packages`, etc. for every
subcommand) is available if you installed via Homebrew — the formula
installs them automatically. The release tarballs also bundle the same
`.1` files alongside the binary; move them into your local man path to
use them:

```bash
tar xzf dotr-*.tar.gz
sudo mv dotr /usr/local/bin/
sudo mv dotr*.1 /usr/local/share/man/man1/
```

`cargo install` doesn't carry man pages — cargo has no mechanism for
installing them — so they aren't available that way.

## Shell completions

Unlike man pages, completions are generated at runtime by `dotr` itself,
so they work the same way no matter how you installed it:

```bash
# Bash
dotr completions bash > ~/.local/share/bash-completion/completions/dotr

# Zsh (any directory on your $fpath)
dotr completions zsh > ~/.zfunc/_dotr

# Fish
dotr completions fish > ~/.config/fish/completions/dotr.fish

# Nushell
dotr completions nushell > ~/.config/nushell/completions/dotr.nu

# Elvish and PowerShell are supported too — see their own docs for where
# to put a generated completion script.
```

Completions cover every command, including the nested `dotr packages` and
`dotr profiles` subcommands. See the
[CLI reference](../reference/cli.md#dotr-completions-shell) for details.

### Dynamic package/profile name completion (Carapace)

The scripts above are static — they know every command and flag, but
`--packages`/`--profile` values aren't completed with real names from your
`config.toml`, since that would require shelling out to `dotr` at
completion time.

If you use [Carapace](https://carapace.sh) as your completion engine (it
supports bash, zsh, fish, nushell, elvish, powershell, and more from one
binary), `dotr completions carapace` prints a Carapace spec that adds
real, live completion of package and profile names by calling
`dotr packages list --plain` / `dotr profiles list --plain` under the hood.
Save it directly into Carapace's specs directory:

```bash
# macOS
dotr completions carapace > \
  ~/Library/Application\ Support/carapace/specs/dotr.yaml

# Linux
dotr completions carapace > ~/.config/carapace/specs/dotr.yaml
```

Unlike the other shells, this spec isn't generated from `dotr`'s own
argument definitions (Carapace has no Rust/clap integration to hook into),
so it's hand-maintained in the DotR source and simply embedded into the
binary — regenerate it the same way after upgrading `dotr` to pick up any
new commands or flags.

`carapace --help` prints the exact specs directory it's currently using if
you're unsure. Once installed, `dotr deploy --packages <TAB>` (and the
equivalent `--profile` flag, and `dotr remove <TAB>`) complete with the
actual packages and profiles in the repository at your current working
directory, in any shell Carapace runs your completions in.

## Bitwarden-backed prompts (optional)

If you set `prompt_backend = "bitwarden"` (see
[Prompts](../concepts/prompts.md#where-answers-go-backends)), you'll need
the [Bitwarden CLI](https://bitwarden.com/help/cli/) (`bw`) installed and
reachable on `PATH`:

```bash
brew install bitwarden-cli
```

Nothing else to set up ahead of time — dotr drives `bw login`/`bw unlock`
interactively the first time it's needed, and creates its secure note
automatically.

## Verifying the install

```bash
dotr --version
dotr --help
```
