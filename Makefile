BUMP_TYPE?=patch
CRATE_NAME?=dotr-dear

.PHONY: help release release-minor release-major release-patch test lint lint-fix vet-whitelist

help:
	@echo "Available targets:"
	@echo "  make test             - Run all tests"
	@echo "  make lint             - Run formatting and clippy checks"
	@echo "  make lint-fix         - Auto-fix formatting and clippy issues"
	@echo "  make release          - Release with patch version bump (default)"
	@echo "  make release-minor    - Release with minor version bump"
	@echo "  make release-major    - Release with major version bump"
	@echo "  make release-patch    - Release with patch version bump"
	@echo "  make vet-whitelist    - Whitelist the current package version in cargo-vet"
	@echo ""
	@echo "Variables:"
	@echo "  BUMP_TYPE=<patch|minor|major>  - Override version bump type"

# Bumping the version separately from the commit/publish/tag/push steps lets us
# slot cargo-vet whitelisting in between: it must land in the same commit as the
# bump, since `cargo vet` runs on every push in CI and otherwise fails on the
# freshly bumped, not-yet-vetted version.
release:
	@OLD_VERSION="$$(cargo pkgid | sed -E 's/.*@//')"; \
	cargo release version $(BUMP_TYPE) --execute; \
	NEW_VERSION="$$(cargo pkgid | sed -E 's/.*@//')"; \
	if [ "$$OLD_VERSION" = "$$NEW_VERSION" ]; then \
		echo "Version unchanged; release aborted."; \
		exit 1; \
	fi; \
	$(MAKE) vet-whitelist; \
	cargo release commit --execute --no-confirm; \
	cargo release publish --execute --no-confirm; \
	cargo release tag --execute --no-confirm; \
	cargo release push --execute --no-confirm

vet-whitelist:
	@NEW_VERSION="$$(cargo pkgid | sed -E 's/.*@//')"; \
	echo "Whitelisting $(CRATE_NAME) $$NEW_VERSION in cargo-vet..."; \
	cargo vet add-exemption $(CRATE_NAME) "$$NEW_VERSION" --criteria safe-to-deploy --no-suggest

release-minor:
	$(MAKE) release BUMP_TYPE=minor

release-major:
	$(MAKE) release BUMP_TYPE=major

release-patch:
	$(MAKE) release BUMP_TYPE=patch

test:
	RUST_TEST_NOCAPTURE=1 RUST_TEST_THREADS=1 RUST_BACKTRACE=1 cargo test

lint:
	@echo "Running cargo fmt check..."
	@cargo fmt --all -- --check
	@echo "\nRunning clippy..."
	@cargo clippy --all-targets --all-features -- -D warnings

lint-fix:
	@echo "Running cargo fmt..."
	@cargo fmt --all
	@echo "\nRunning clippy with auto-fix..."
	@cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features
	@echo "\nLinting complete! Please review changes."
