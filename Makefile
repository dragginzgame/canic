.DEFAULT_GOAL := help
FORMAT_RUST_ARGS := --extra-workspace qualification
IC_TOOLS := icp
include vendor/toko-tooling/make/consumer.mk
include vendor/toko-tooling/make/cargo-dependencies.mk

.PHONY: help canic check test clippy doc lint ci m0-setup m0-test m0-maintenance-test m0-artifact-test m0-artifact-clippy m0-artifact-doc m0-clippy m0-doc m1-test m1-clippy m1-doc
help:
	@printf '%s\n' \
	  'make canic               Build the Canic CLI (target/debug/canic)' \
	  'make check               Check the canic library' \
	  'make test                Run canic tests' \
	  'make clippy              Check canic lints' \
	  'make doc                 Check canic documentation' \
	  'make lint                Check all lints, docs, formatting and tooling agreement' \
	  'make fmt / fmt-check     Format / check Rust and Cargo manifests' \
	  'make deps-list           List product and qualification package versions' \
	  'make deps-update         Update product and qualification dependencies' \
	  'make ci                  Run all repository checks (aggregate)' \
	  'make m0-setup            Explicitly prepare the local M0 PocketIC server' \
	  'make m0-test             Run effect-boundary and owner-maintenance experiments' \
	  'make m0-maintenance-test Rehearse owner upgrade and failed-upgrade recovery' \
	  'make m0-artifact-test    Prove retained artifact installation without operator' \
	  'make m0-artifact-clippy  Lint the artifact custody fixture' \
	  'make m0-artifact-doc     Check artifact custody documentation' \
	  'make m0-clippy           Lint the M0 native and Wasm fixture' \
	  'make m0-doc              Check M0 fixture documentation' \
	  'make m1-test             Rehearse CLI deployment and upgrade locally' \
	  'make m1-clippy           Lint CLI, host and demo (native and Wasm)' \
	  'make m1-doc              Check CLI, host and demo documentation' \
	  'make install-tools       Prepare pinned development tools' \
	  'make tools-check         Verify prepared tools' \
	  'make install-hooks       Activate the formatting commit hook' \
	  'make toko-tooling-check  Verify the installed tooling snapshot'

canic:
	cargo build --locked -p canic-cli --bin canic
	@printf '%s\n' 'Run: ./target/debug/canic --help'

check:
	cargo check --locked -p canic

test:
	cargo test --locked -p canic

clippy:
	cargo clippy --locked -p canic --all-targets -- -D warnings

doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked -p canic --no-deps

# M1 owns product Clippy/Rustdoc coverage, including the shared canic library.
lint: fmt-check toko-tooling-check m0-clippy m0-doc m0-artifact-clippy m0-artifact-doc m1-clippy m1-doc

ci: lint test m0-test m0-artifact-test m1-test

# The runner selects the Wasm path from Cargo artifact messages.
m0-setup:
	cargo run --locked --manifest-path qualification/Cargo.toml -p canic-m0-tests --example prepare_server

m0-test:
	bash scripts/dev/test-m0.sh

m0-maintenance-test:
	bash scripts/dev/test-m0.sh maintenance

m0-artifact-test:
	bash scripts/dev/test-m0-artifact.sh

m0-artifact-clippy:
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-m0-artifact --all-targets -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-m0-artifact --target wasm32-unknown-unknown --lib --all-features -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-m0-tests --test artifact -- -D warnings

m0-artifact-doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --manifest-path qualification/Cargo.toml -p canic-m0-artifact --no-deps

m0-clippy:
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-m0-probe --all-targets -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-m0-tests --lib --example prepare_server --test creation --test funding --test maintenance -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-m0-probe --target wasm32-unknown-unknown --lib --all-features -- -D warnings

m0-doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --manifest-path qualification/Cargo.toml -p canic-m0-probe -p canic-m0-tests --no-deps

m1-test:
	bash scripts/dev/test-m1.sh

m1-clippy:
	cargo clippy --locked -p canic -p canic-host -p canic-cli -p canic-hello --all-targets -- -D warnings
	cargo clippy --locked -p canic-hello --target wasm32-unknown-unknown --all-features -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-m1-tests --all-targets -- -D warnings

m1-doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked -p canic -p canic-host -p canic-hello --no-deps
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --manifest-path qualification/Cargo.toml -p canic-m1-tests --no-deps
