BUMP_TYPE?=patch

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
	cargo clippy

lint-fix:
	cargo clippy --fix
