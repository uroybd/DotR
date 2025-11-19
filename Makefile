BUMP_TYPE?=patch

.PHONY: help release release-minor release-major release-patch test lint lint-fix

help:
	@echo "Available targets:"
	@echo "  make test             - Run all tests"
	@echo "  make lint             - Run formatting and clippy checks"
	@echo "  make lint-fix         - Auto-fix formatting and clippy issues"
	@echo "  make release          - Release with patch version bump (default)"
	@echo "  make release-minor    - Release with minor version bump"
	@echo "  make release-major    - Release with major version bump"
	@echo "  make release-patch    - Release with patch version bump"
	@echo ""
	@echo "Variables:"
	@echo "  BUMP_TYPE=<patch|minor|major>  - Override version bump type"

release:
	cargo release $(BUMP_TYPE) --execute --no-publish

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
