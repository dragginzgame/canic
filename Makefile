.DEFAULT_GOAL := help
override TOKO_TOOLING_ROOT := $(CURDIR)
include make/consumer.mk

.PHONY: help check test clippy doc ci
help:
	@printf '%s\n' \
	  'make check               Check the canic library' \
	  'make test                Run canic tests' \
	  'make clippy              Check canic lints' \
	  'make doc                 Check canic documentation' \
	  'make fmt / fmt-check     Format / check Rust and Cargo manifests' \
	  'make ci                  Run all repository checks (aggregate)' \
	  'make install-tools       Prepare pinned development tools' \
	  'make tools-check         Verify prepared tools' \
	  'make install-hooks       Activate the formatting commit hook' \
	  'make toko-tooling-check  Verify the installed tooling snapshot'

check:
	cargo check --locked -p canic

test:
	cargo test --locked -p canic

clippy:
	cargo clippy --locked -p canic --all-targets -- -D warnings

doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked -p canic --no-deps

ci: fmt-check clippy test doc
