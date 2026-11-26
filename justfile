# Common tasks for this repository. `just` lists them. CI runs the same cargo commands directly
# (.github/workflows/pr-check.yml), not this file.

set shell := ["bash", "-euo", "pipefail", "-c"]

crate := "dotr-dear"

# List the recipes
default:
    @just --list --unsorted

# --- Checks (what CI runs, in AGENTS.md's order) ---

# Fix what clippy and rustfmt can fix themselves
lint-fix:
    cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features
    cargo fmt --all

# rustfmt check and clippy, every warning an error
lint:
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings

# Every test, one at a time: they share tests/playground and the environment
test *args:
    RUST_TEST_NOCAPTURE=1 RUST_BACKTRACE=1 cargo test {{ args }} -- --test-threads=1

# Everything before a change is done: fix, lint, test
check: lint-fix lint test

# --- Building ---

# Regenerate the man pages into man/
man:
    cargo run --bin mangen --features mangen

# Build the documentation site into docs/book
docs:
    mdbook build docs

# --- Releases ---

# Lines added under src and tests since the last `(smells)` commit (AGENTS.md: a review is due at 5,000)
smells:
    #!/usr/bin/env bash
    set -euo pipefail
    last=$(git log -1 --format=%H -E --grep='^[a-z]+\(smells\)!?:')
    git diff --shortstat "${last:-$(git rev-list --max-parents=0 HEAD)}"..HEAD -- src tests

# Release: bump (patch, minor or major), whitelist the version in cargo-vet, commit, publish, tag and push
release bump="patch":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ bump }}" in
      patch|minor|major) ;;
      *) echo "The bump is patch, minor or major, not \`{{ bump }}\`." >&2; exit 1 ;;
    esac
    old="$(cargo pkgid | sed -E 's/.*@//')"
    # Bumping apart from the commit/publish/tag/push steps lets cargo-vet whitelisting land in the same commit as the
    # bump: `cargo vet` runs on every push in CI and fails on a freshly bumped, not yet vetted version.
    cargo release version "{{ bump }}" --execute
    new="$(cargo pkgid | sed -E 's/.*@//')"
    if [ "$old" = "$new" ]; then
      echo "Version unchanged; release aborted." >&2
      exit 1
    fi
    just vet-whitelist
    cargo release commit --execute --no-confirm
    cargo release publish --execute --no-confirm
    cargo release tag --execute --no-confirm
    cargo release push --execute --no-confirm

# Whitelist the current version of the crate in cargo-vet
vet-whitelist:
    #!/usr/bin/env bash
    set -euo pipefail
    version="$(cargo pkgid | sed -E 's/.*@//')"
    echo "Whitelisting {{ crate }} $version in cargo-vet..."
    cargo vet add-exemption {{ crate }} "$version" --criteria safe-to-deploy --no-suggest
