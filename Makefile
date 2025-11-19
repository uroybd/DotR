BUMP_TYPE ?= patch

test:
	RUST_TEST_NOCAPTURE=1 RUST_TEST_THREADS=1 RUST_BACKTRACE=1 cargo test

release:
	cargo release ${BUMP_TYPE} --execute --no-publish

lint:
	cargo clippy lint

lint-fix:
	cargo clippy --fix
