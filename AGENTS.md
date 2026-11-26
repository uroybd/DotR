# Working on DotR

A dotfiles manager: one Rust crate (`dotr-dear`, binary `dotr`) that deploys files and directories from a repository
to their places on a machine, compiling Tera templates on the way, and backs changes made there up into the repository
(`dotr update`). The README says what it does for a user and `docs/` (an mdBook) is the full guide; this file is how
to change it.

## Map

```text
src/cli          clap commands and `run_cli`, which loads the config and context and dispatches
src/config       config.toml: packages, profiles, platforms; deploy/backup over a profile's packages
src/package      one package: deploy, backup (update), diff, clean, symlinks
src/merge        backing up a templated file: three-way merge plus reverse substitution
src/context      the variables a template sees: env, config, platform, package, profile, prompted
src/prompt_store where prompted (user) variables live: a file, the OS keychain, or a Bitwarden note
src/profile, src/platform   their config types
src/utils        paths, Tera compiling, and the crate's only way to print (`out!`, `outln!`, `cprintln`)
tests/           integration tests on a playground directory (`tests/playground`), run single-threaded
```

## Before you say a change is done

Format, lint and test, in that order. CI (`.github/workflows/pr-check.yml`) runs the same and fails on any of them.

```bash
make lint-fix     # clippy --fix and cargo fmt, in place
make lint         # rustfmt check + clippy --all-targets --all-features -D warnings
make test         # cargo test, single-threaded: the tests share tests/playground and the environment
```

- Clippy is clean; keep it so. Fix a warning rather than silencing it.
- The lints are set in `[lints]` in `Cargo.toml`, and CI makes every one an error. Beyond clippy's defaults they forbid
  `unwrap`, `expect`, `panic!`, `todo!`, `dbg!`, printing, and `let _ =` on a `#[must_use]` value in code that ships:
  return the error, and where a value truly cannot be missing say so with a pattern (`let Some(x) = .. else`) rather
  than a panic. To ignore a result on purpose use `.ok();` with a comment saying why. `clippy.toml` lets `#[test]` code
  unwrap, panic and print; an integration test file under `tests/` opens with one
  `#![allow(.., reason = "test code: a failure is the point")]` for its helpers.
- Print only through `out!`, `outln!` or `cprintln` (`src/utils`).
- A lint that is wrong for one place is silenced there with `#[expect(lint, reason = "...")]`, which warns once the
  reason is gone; `#[allow]` only where the lint fires in some builds and not others, and with its reason too. Add to
  the list when a new lint earns its place; don't remove one to get a change through.
- Don't hand-format around rustfmt (default settings, 100 columns).

## Conventions

- **Docs move with the code.** A new or changed behaviour updates its page in `docs/src/` (and
  `docs/src/reference/cli.md` for a flag), and the README if a user would notice.
- **Nothing changes silently.** Whenever `update` writes something other than the deployed bytes back into the
  repository (a value turned back into a `{{ variable }}`, a template line kept over the deployed one, deployed lines
  taken or added beside the template's, an edit saved from the editor that renders differently), or leaves a file or a
  part of one out, it says so on the console, one line per change, with the file and line. A new kind of change gets
  a `merge::Change` variant and a line in `report_merge` (`src/package`).
- **Secrets stay out of the repository.** Prompted variables can hold secrets: `update` never writes their values into
  a tracked file (they go back as `{{ name }}`), and anything it prints about a templated file shows them as `***`.
  Keep tests that prove it, and add one for any new path that copies content into the repository.
- **The template stays the source of truth.** Backing up a templated file must round-trip: compiling what is written
  must give the deployed file exactly (or, where the user chose to keep the template's lines, what the template gave
  before), or nothing is written. An edit saved from the editor is the one exception, and is reported when it renders
  differently.
- **Binary files are copied, never diffed.** Text means UTF-8 without a NUL byte (`is_templated_str`,
  `utils::as_text`); anything else is copied byte for byte and compared as bytes.
- **Tests never wait on a terminal.** `update` asks about conflicts when stdin and stdout are terminals, which they
  are under `make test`: a test that builds `UpdateArgs` for a templated package sets `conflict` explicitly.

## Releases

`make release` (or `release-minor`, `release-major`) bumps the version, whitelists it in cargo-vet, then commits,
publishes, tags and pushes. Commit messages are conventional (`feat:`, `fix:`, `doc:`, `test:`, `refactor:`, `style:`,
`chore:`); `cliff.toml` groups them into the changelog.

**Smell reviews.** A commit that fixes what a holistic code-smell review found takes the scope `smells`, with its
honest type: `fix(smells): …` for a bug the review turned up, `refactor(smells): …` for the rest. Nothing else uses
that scope. The last such commit is the last review:

```bash
last=$(git log -1 --format=%H -E --grep='^[a-z]+\(smells\)!?:')
git diff --shortstat "${last:-$(git rev-list --max-parents=0 HEAD)}"..HEAD -- src tests
```

When you cut a release, run that; at 5,000 lines added or more, say in your reply that a smell review is due (the
`smell-review` skill in `.claude/skills/`). Don't start one unasked. Code you write in a change gets the skill's
section 2 checklist and its clippy probe before you call it done.

## Git

Don't commit or push unless asked. When asked, use a conventional message that says what and why, and keep a
formatting-only change in its own commit.
