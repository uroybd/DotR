---
name: smell-review
description: >
  Holistic code-smell review of this repository: measure, find the smells (with the pitfalls models fall into when
  they write Rust), verify each against the code and a real run of `dotr`, fix them, and commit under the `(smells)`
  scope. Use when uroybd asks for a smell review, asks whether there is room to simplify or abstract, when the release
  reminder in AGENTS.md says one is due, or (sections 2 and the probe only) on the code a change adds, before calling
  it done.
---

# Smell review

A review that ends in fixes, not a list. Findings are verified before they are reported; a smell you did not check
against the code is a guess.

Two sizes:
- **Full review**: every section, over the whole crate. When asked, or when AGENTS.md says one is due.
- **Change check**: sections 2 and 3 and the clippy probe, over the code the current change adds or rewrites. Part of
  "Before you say a change is done"; findings are fixed in the change itself, not under the `smells` scope.

## 1. Measure

First, how much has changed since the last review (the last commit with the `smells` scope, else the first commit):

```bash
last=$(git log -1 --format=%H -E --grep='^[a-z]+\(smells\)!?:')
git diff --shortstat "${last:-$(git rev-list --max-parents=0 HEAD)}"..HEAD -- src tests
```

Say the number in the report; at 5,000 lines added or more a review is due (AGENTS.md).

```bash
# biggest files (tests cut off)
git ls-files '*.rs' | grep -v test | xargs wc -l | sort -rn | sed -n 2,12p
# lint escape hatches and leftovers
git grep -nE "#\[(allow|expect)" -- src tests; git grep -nE "TODO|FIXME|HACK" -- src
# stringly typed errors, string-mapped errors, silent fallbacks
git grep -c 'Result<[^>]*, String>' -- src
git grep -cE 'map_err\(\|e\| (e\.to_string\(\)|format!)' -- src
git grep -nE '\.ok\(\)\?|\.ok\(\);|unwrap_or_default\(\)|from_(str|slice|value)\([^)]*\)\.ok\(\)' -- src
# hidden global state
git grep -nE 'static .*(Mutex|RwLock|OnceLock|LazyLock)|thread_local!' -- src
# the test fixtures, one per file: has a helper drifted between them?
git grep -n "fn get_cli\|fn deploy\|fn update\|fn import" -- tests | sort -t: -k3
```

Then let clippy look for the pitfalls below with lints the crate does not turn on (a probe, not a gate; a scratch
target so the user's builds are untouched):

```bash
CARGO_TARGET_DIR=<scratchpad>/clippy-probe cargo clippy -q --all-targets --all-features -- \
  -W clippy::redundant_clone -W clippy::needless_pass_by_value -W clippy::unused_self \
  -W clippy::trivially_copy_pass_by_ref -W clippy::needless_collect -W clippy::redundant_allocation \
  -W clippy::rc_buffer -W clippy::significant_drop_tightening -W clippy::type_complexity -W clippy::too_many_lines
```

(Baseline when this skill was adopted, 2026-11-26, before any review: one `too_many_lines`, `run_cli` in
`src/cli/mod.rs`, a flat dispatch over the commands; five `redundant_clone`, one in `src/config/mod.rs` and four in
tests.) `option_if_let_else` is noise here; leave it out. A lint that stays at zero over two reviews can move into
`[lints.clippy]` in `Cargo.toml`, with its reason.

A long function or a big file is a lead, not a finding: read it before saying it is a smell.

Leads already known, for the first full review to verify:
- `&Option<Vec<String>>` / `&Option<String>` throughout the public API (`Config::filter_packages`,
  `Context::get_prompts`, `Context::new`); two private functions carry `#[expect(clippy::ref_option)]` because of it.
- `Package::diff_file` reads an unreadable deployed file as empty (`unwrap_or_default`), so the diff shows the whole
  file as added instead of saying it could not be read.
- Each `tests/*.rs` has its own `TestFixture` with near-identical helpers.
- `is_templated` reads an unreadable file as "not a template", so `backup` would copy the deployed file over a
  template it could not read.
- In a directory package, `backup` counts the package as backed up even when one of its templated files ended in a
  conflict (the warning says so, the summary does not).
- `merge_template` is 117 lines: the conflict-resolution arm could move into its own function.

## 2. Rust pitfalls to look for

What models (and people in a hurry) get wrong when they write Rust. Each is a smell only when the code bears it out.
For the reasoning and good/bad examples, read the matching rule of the `rust-skills` skill
(`~/.claude/skills/rust-skills/rules/<id>.md`), named in brackets; where this repo decides otherwise, this file wins.

**Ownership and allocation**
- `.clone()` to quiet the borrow checker, where a borrow, a move or a reordered statement would do. A clone of a
  value never used again after it is a move written the long way. [`own-borrow-over-clone`, `anti-clone-excessive`]
  The variable tables (`toml::Table`) are cloned and extended per package; cloning one is fine, cloning it per file
  is not.
- `String` / `Vec<T>` parameters that are only read (`&str`, `&[T]`); `to_owned()` then `&` back again.
  [`own-slice-over-vec`, `anti-string-for-str`]

**Abstraction**
- A generic or a trait with one implementation and one caller; trait bounds piled on that the body does not use.
  Prefer the concrete type until a second caller needs another. [`anti-over-abstraction`, `type-generic-bounds`]
  (`PromptStoreBackend` has three implementations; it is not this.)
- A wrapper, builder or newtype that only forwards; a helper used once that names nothing the call site did not.
- The opposite, too: the same check written in several places that has drifted. Watch for template detection
  (`is_templated`, `is_templated_str`), path resolution (`resolve_path`, `resolve_dest`) and the variable merge order
  (`Context::get_context_variables` against `Package::get_context_variables`). One function, called from each.

**Errors**
- `Result<_, String>` where a caller has to tell failures apart. [`anti-stringly-typed`, `type-no-stringly`]
  `anyhow` is right for a CLI's errors that only a person reads; a typed error earns its place only where code
  branches on the failure.
- A failure turned into a value: `.ok()`, `unwrap_or_default()`, `let _ =` that makes "unreadable" read as "empty"
  or "missing" read as "unchanged". Fall back only where the function says so.
- Two functions that answer the same question differently (diff saying "no changes" where deploy would write).

**State and control flow**
- Global mutable state where an argument would do; when it is a deliberate cache, its comment says why.
- A function over ~150 lines that mixes deciding with printing or with I/O; nesting a guard clause (`let … else`)
  would flatten. A flat `match` over the commands is not this.
- `bool` flags or string modes that are really an enum (`UpdateArgs`/`DeployArgs` already carry several).
  [`type-no-stringly`, `api-newtype-safety`]

**This crate's own rules** (AGENTS.md, Conventions)
- Anything `update` writes into the repository that differs from the deployed bytes, with no console line saying so.
- A prompted variable's value that can reach a tracked file, or the console unmasked when printing about a template.

The crate's lints already forbid `unwrap`, `expect`, `panic!`, printing outside `src/utils` and ignored
`#[must_use]`; don't re-find those.

**Suppressions.** A lint is silenced with `#[expect(lint, reason = "...")]`, not `#[allow]`: when a later change
removes what the lint was about, the compiler says the expectation is unfulfilled. `#[allow]` stays only where the lint
fires in some builds and not others (shared test helpers in `tests/common`), with its reason. An `#[allow]` without
that reason is a finding.

## 3. Not smells here

Check AGENTS.md before calling something a smell; these are by design:

- The integration tests share `tests/playground` and the process environment, so they run single-threaded
  (`make test`, CI). Serial tests are not a finding; a test that only passes in one order is.
- `unsafe` around `std::env::set_var` in tests, with its SAFETY comment: that is the edition-2024 API.
- `anyhow` everywhere in `src`: a CLI's errors are read by a person.

## 4. Verify, then report

- For behaviour, run the real `dotr` (`cargo run --`) against a scratch working directory and a scratch `HOME`,
  never the user's own dotfiles repository or home directory.
- Report findings in order: bugs first (behaviour that is wrong), then smells worth fixing (with the fix), then what
  looks like a smell and isn't. Say what you did not check.

## 5. Fix and commit

- Fix with tests: a test that fails before the fix for each bug, and the existing tests green after.
- Run the checks of "Before you say a change is done" in AGENTS.md (`make lint-fix`, `make lint`, `make test`).
- Commit (when asked) under the `smells` scope with the honest type: `fix(smells): …` for a bug the review found,
  `refactor(smells): …` for the rest; a formatting-only change in its own commit. That scope is how the next reminder
  finds this review.

## Where the checklist comes from

Adapted from the Meghnaad repository's `smell-review` skill. Practitioner reports, not studies: Microsoft's Pragmatic
Rust Guidelines (prefer concrete types, then generics, then `dyn`), Google's Rust style guide (no premature
generalization), Effective Rust item 4 (idiomatic error types), and accounts of reviewing large amounts of
model-written Rust (clone and `unwrap` overuse, traits and generics for one caller).
