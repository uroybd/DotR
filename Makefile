BUMP_TYPE ?= patch

test:
	RUST_TEST_NOCAPTURE=1 RUST_TEST_THREADS=1 RUST_BACKTRACE=1 cargo test

bump:
	bump-version Cargo.toml --bump-type $(BUMP_TYPE)
