.DEFAULT_GOAL := help
FORMAT_RUST_ARGS := --extra-workspace qualification
IC_TOOLS := icp
CARGO_PUBLISH_PACKAGES := canic-macros canic canic-dormant canic-management-protocol canic-management canic-root canic-host canic-cli
CARGO_PUBLISH_REGISTRY := crates-io
CARGO_PUBLISH_VERIFY_TARGETS := publication-admission
include vendor/toko-tooling/make/consumer.mk
include vendor/toko-tooling/make/cargo-dependencies.mk

.PHONY: help canic check test clippy doc lint ci qualification-setup effect-test effect-maintenance-test artifact-custody-test artifact-custody-clippy artifact-custody-doc effect-clippy effect-doc journey-test journey-clippy journey-doc
help:
	@printf '%s\n' \
	  'make canic                   Build and install canic on PATH' \
	  'make demo                    Start a local counter fleet session (Enter to stop)' \
	  'make counter-test            Qualify counter persistence, fences and upgrade rollback' \
	  'make profile-test            Run retained profile ownership/upgrade regressions' \
	  'make growth-test             Qualify one autonomous worker and interrupted effects' \
	  'make endpoint-test           Qualify management mounts, guards and reply rollback' \
	  'make root-test               Qualify Root creation, upgrades and child isolation' \
	  'make blob-test               Qualify embedded Blob/catalog composition' \
	  'make db-test                 Qualify typed database mounts and rollback' \
	  'make auth-test               Qualify optional Auth composition and rollback' \
	  'make store-test              Qualify Canic Root/Store integration and maintenance' \
	  'make store-readiness-test    Qualify Store observation after Root handover' \
	  'make publication-test        Qualify CLI Store bootstrap and publication' \
	  'make check                   Check the canic library' \
	  'make test                    Run canic tests' \
	  'make clippy                  Check canic lints' \
	  'make doc                     Check canic documentation' \
	  'make lint                    Check all lints, docs, formatting and tooling agreement' \
	  'make fmt / fmt-check         Format / check Rust and Cargo manifests' \
	  'make deps-list               List product and qualification package versions' \
	  'make deps-update             Update product and qualification dependencies' \
	  'make ci                      Run all repository checks (aggregate)' \
	  'make backup-signer-setup     Prepare the external signer for local qualification' \
	  'make backup-signer-test      Exercise direct Backup recovery with a disposable keyring' \
	  'make qualification-setup     Explicitly prepare the shared PocketIC server' \
	  'make effect-test             Run effect-boundary and owner-maintenance experiments' \
	  'make effect-maintenance-test Rehearse owner upgrade and failed-upgrade recovery' \
	  'make artifact-custody-test   Prove retained artifact installation without operator' \
	  'make artifact-custody-clippy Lint the artifact custody fixture' \
	  'make artifact-custody-doc    Check artifact custody documentation' \
	  'make effect-clippy           Lint effect-boundary fixtures' \
	  'make effect-doc              Check effect-boundary documentation' \
	  'make journey-test            Rehearse CLI deployment and upgrade locally' \
	  'make journey-clippy          Lint CLI, host and demo (native and Wasm)' \
	  'make journey-doc             Check CLI, host and demo documentation' \
	  'make install-tools           Prepare pinned development tools' \
	  'make tools-check             Verify prepared tools' \
	  'make install-hooks           Activate the formatting commit hook' \
	  'make toko-tooling-check      Verify the installed tooling snapshot'

canic:
	cargo install --locked --debug --path crates/canic-cli --root "$(HOME)/.local" --force
	@test "$$(command -v canic)" = "$(HOME)/.local/bin/canic" || { printf '%s\n' 'The installed ~/.local/bin/canic is shadowed or unavailable on PATH.' >&2; exit 1; }
	@canic --version
	@printf '%s\n' 'Ready: canic --help'

.PHONY: demo
DEMO_APP ?= counter
demo: canic
	$(MAKE) ic-tools-check
	cargo run --locked --manifest-path qualification/Cargo.toml -p canic-qualification --example demo -- --app "$(DEMO_APP)"

.PHONY: counter-test counter-clippy counter-doc
counter-test:
	bash scripts/dev/test-counter.sh

counter-clippy:
	cargo clippy --locked -p ex-counter --all-targets -- -D warnings
	cargo clippy --locked -p ex-counter --lib --target wasm32-unknown-unknown --all-features -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test counter -- -D warnings

counter-doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked -p ex-counter --lib --target wasm32-unknown-unknown --no-deps

check:
	cargo check --locked -p canic

test:
	cargo test --locked -p canic --features storage

clippy:
	cargo clippy --locked -p canic --all-targets -- -D warnings

doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked -p canic --no-deps

# These aggregates share Cargo outputs and installed-test artifact directories.
# Use the prerequisite-free form supported by the existing GNU Make baseline.
ifneq ($(filter ci lint,$(MAKECMDGOALS)),)
.NOTPARALLEL:
endif

# The CLI journey owns product Clippy/Rustdoc coverage, including the shared canic library.
lint: fmt-check toko-tooling-check check-app-endpoints effect-clippy effect-doc artifact-custody-clippy artifact-custody-doc journey-clippy journey-doc counter-clippy counter-doc profile-clippy profile-doc root-clippy root-doc store-clippy blob-clippy blob-doc auth-clippy auth-doc db-clippy

# Build through the same app declaration used in the local walkthrough.
.PHONY: miner-test
miner-test:
	cargo run --locked --quiet -p canic-cli -- app build --config apps/examples/miner-pool/canic.toml
	cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test miner
	cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test store store_root_tests::miner_pool -- --nocapture

.PHONY: notebook-test
notebook-test:
	cargo run --locked --quiet -p canic-cli -- app build --config apps/examples/notebook/canic.toml
	cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test notebook

.PHONY: metrics-test
metrics-test:
	cargo run --locked --quiet -p canic-cli -- app build --config apps/examples/metrics/canic.toml --canister root
	cargo run --locked --quiet -p canic-cli -- app build --config apps/examples/metrics/canic.toml --canister ex-metrics
	cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test metrics_example

CI_TEST_TARGETS := metrics-test notebook-test test effect-test artifact-custody-test journey-test counter-test profile-test growth-test root-test endpoint-test store-test publication-test blob-test auth-test db-test
ci: lint
	$(MAKE) -j1 $(CI_TEST_TARGETS)

# Publication keeps the complete CI roster. Only installed effect executions
# have a qualified retry contract. Compilation and other checks run normally.
.PHONY: publication-admission publication-effect-test
publication-admission:
	bash scripts/ci/publication-preflight.sh
	$(MAKE) -j1 lint
	$(MAKE) -j1 $(subst effect-test,publication-effect-test,$(CI_TEST_TARGETS))

publication-effect-test:
	bash scripts/dev/test-effects.sh publication

# Routine hosted coverage: product packages only, no signer/server provisioning.
# Installed journeys remain explicit through the existing qualification runners.
.PHONY: check-app-endpoints check-pins ci-native ci-wasm
check-app-endpoints:
	perl scripts/ci/check-app-endpoints.pl

check-pins:
	bash vendor/toko-tooling/scripts/ci/check-dependency-pins.sh --cargo-inheritance

ci-native:
	cargo clippy --locked --workspace --all-targets -- -D warnings
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
	cargo test --locked --workspace

ci-wasm:
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-greeting-witness --lib --target wasm32-unknown-unknown --all-features -- -D warnings
	cargo clippy --locked -p canic -p canic-dormant -p ex-counter -p ex-metrics -p canic-management -p canic-root --lib --target wasm32-unknown-unknown --all-features -- -D warnings
	RUSTDOCFLAGS="-D warnings" cargo doc --locked -p canic-management --target wasm32-unknown-unknown --no-deps
	RUSTDOCFLAGS="-D warnings" cargo doc --locked -p canic --features blob,auth --target wasm32-unknown-unknown --no-deps

# Match the mounted Wire contracts to the two built Wasms and the reviewed API.
.PHONY: check-wire-endpoints
check-wire-endpoints:
	bash vendor/toko-tooling/scripts/ci/check-api-baseline.sh apps/examples/user-directory/wire-baseline/ex-user-directory -- \
		cargo run --locked --quiet --manifest-path qualification/Cargo.toml -p canic-qualification --features inspection --example user_directory_manifests --
	cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --features inspection --test inspection
	bash scripts/dev/test-wire-starter.sh

# The runner selects the Wasm path from Cargo artifact messages.
qualification-setup:
	bash scripts/dev/prepare-server.sh

effect-test:
	bash scripts/dev/test-effects.sh

effect-maintenance-test:
	bash scripts/dev/test-effects.sh maintenance

artifact-custody-test:
	bash scripts/dev/test-artifact-custody.sh

artifact-custody-clippy:
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-artifact-probe --all-targets -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-artifact-probe --target wasm32-unknown-unknown --lib --all-features -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test artifact -- -D warnings

artifact-custody-doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --manifest-path qualification/Cargo.toml -p canic-artifact-probe --no-deps

effect-clippy:
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-effect-probe --all-targets -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-qualification --example prepare_server --test creation --test funding --test maintenance -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-effect-probe --target wasm32-unknown-unknown --lib --all-features -- -D warnings

effect-doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --manifest-path qualification/Cargo.toml -p canic-effect-probe --no-deps

journey-test:
	bash scripts/dev/test-journey.sh

journey-clippy:
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-greeting-witness --all-targets -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-qualification --features backup --test backup -- -D warnings
	cargo clippy --locked -p canic-macros -p canic -p canic-host -p canic-cli --features canic-host/backup --all-targets -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-greeting-witness --target wasm32-unknown-unknown --all-features -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-qualification --lib --example demo --test journey --test publication -- -D warnings

journey-doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --manifest-path qualification/Cargo.toml -p canic-greeting-witness --no-deps
	RUSTDOCFLAGS="-D warnings" cargo doc --locked -p canic-macros -p canic -p canic-host --features canic-host/backup --no-deps
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --manifest-path qualification/Cargo.toml -p canic-qualification --no-deps

.PHONY: profile-test profile-clippy profile-doc
profile-test:
	bash scripts/dev/test-profile.sh

profile-clippy:
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-profile-protocol -p canic-profile-worker -p canic-profile-entry -p canic-profile-client -p canic-profile-manager --all-targets -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-profile-worker -p canic-profile-entry -p canic-profile-manager --target wasm32-unknown-unknown --all-features -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test profile --test growth -- -D warnings

profile-doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --manifest-path qualification/Cargo.toml -p canic-profile-protocol -p canic-profile-worker -p canic-profile-entry -p canic-profile-client -p canic-profile-manager --no-deps

.PHONY: growth-test
growth-test:
	bash scripts/dev/test-growth.sh

# Focused management mounting/admission and reply rollback; no CLI journey suite.
.PHONY: endpoint-test
endpoint-test:
	cargo test --locked -p canic --features method --test named_contract
	bash scripts/dev/test-endpoints.sh

.PHONY: root-test root-clippy root-doc
root-test:
	bash scripts/dev/test-root.sh

root-clippy:
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-directory-witness --target wasm32-unknown-unknown -- -D warnings
	cargo clippy --locked -p canic-management-protocol -p canic-management -p canic-root --features backup,sharding --all-targets -- -D warnings
	cargo clippy --locked -p canic-management -p canic-root --lib --target wasm32-unknown-unknown --all-features -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test root --test endpoints -- -D warnings

root-doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked -p canic-management-protocol -p canic-management -p canic-root --features backup --no-deps
	RUSTDOCFLAGS="-D warnings" cargo doc --locked -p canic-management --features backup,sharding --target wasm32-unknown-unknown --no-deps

.PHONY: store-test store-readiness-test store-clippy publication-test
publication-test:
	bash scripts/dev/test-publication.sh

store-readiness-test:
	bash scripts/dev/test-store.sh readiness

store-test:
	bash scripts/dev/test-store.sh

store-clippy:
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-shard-hub --target wasm32-unknown-unknown -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-qualification --test store -- -D warnings

.PHONY: auth-test auth-clippy auth-doc auth-browser-prepare auth-browser-test auth-profile-test
auth-profile-test:
	bash scripts/dev/test-auth.sh --profile-browser

auth-browser-prepare:
	bash scripts/dev/prepare-auth-browser.sh
	npm --prefix crates/canic/browser ci --ignore-scripts --no-audit --no-fund

auth-browser-test:
	cargo run --locked -p canic --features auth-browser --example auth-codecs -- target/auth-browser/generated
	diff -ru crates/canic/browser/src/generated target/auth-browser/generated
	npm --prefix crates/canic/browser test

auth-test:
	bash scripts/dev/test-auth.sh

auth-clippy:
	cargo clippy --locked -p canic -p canic-root --features canic/auth-receiver-stable,canic/auth-issuer-stable,canic/auth-browser,canic-root/auth --all-targets -- -D warnings
	cargo clippy --locked -p canic -p canic-root --features canic/auth-enrollment,canic-root/auth --target wasm32-unknown-unknown -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-auth-witness --target wasm32-unknown-unknown -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-auth-witness --features profile --target wasm32-unknown-unknown -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-auth-witness --features damaged-checkpoint --target wasm32-unknown-unknown -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-qualification --features profile-state --test auth --test auth_browser -- -D warnings

auth-doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked -p canic --features auth-receiver-stable,auth-issuer-stable --target wasm32-unknown-unknown --no-deps
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --manifest-path qualification/Cargo.toml -p canic-auth-witness --no-deps

.PHONY: asset-library-test
asset-library-test:
	cargo run --locked --quiet -p canic-cli -- app build --config apps/examples/asset-library/canic.toml
	cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification --features profile-state --test blob captured_users_share_one_object_without_application_authority_leaking -- --exact --nocapture

.PHONY: blob-test blob-clippy blob-doc
blob-test:
	bash scripts/dev/test-blob.sh

blob-clippy:
	cargo clippy --locked -p canic --features blob --all-targets -- -D warnings
	cargo clippy --locked -p canic-root --features blob --target wasm32-unknown-unknown -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-asset-catalog --target wasm32-unknown-unknown --all-features -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-qualification --features profile-state --test blob -- -D warnings

blob-doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --locked -p canic --features blob --target wasm32-unknown-unknown --no-deps
	RUSTDOCFLAGS="-D warnings" cargo doc --locked -p canic-root -p canic-management --features canic-root/blob --target wasm32-unknown-unknown --no-deps
	RUSTDOCFLAGS="-D warnings" cargo doc --locked --manifest-path qualification/Cargo.toml -p canic-asset-catalog --no-deps

.PHONY: db-test db-clippy
db-test:
	cargo test --locked -p canic --features provisioning --lib
	bash scripts/dev/test-db.sh

db-clippy:
	cargo clippy --locked -p canic --features db-schema,provisioning --all-targets -- -D warnings
	cargo clippy --locked -p canic --features db-schema,provisioning --lib --target wasm32-unknown-unknown -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-db-witness --lib --target wasm32-unknown-unknown -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-db-witness --lib --target wasm32-unknown-unknown --features fault-injection -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-db-witness --lib --target wasm32-unknown-unknown --features incompatible-schema -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-db-witness --lib --target wasm32-unknown-unknown --features managed-fixture,fault-injection -- -D warnings
	cargo clippy --locked --manifest-path qualification/Cargo.toml -p canic-db-witness --test installed -- -D warnings

.PHONY: backup-signer-setup backup-signer-test
backup-signer-setup:
	umask 077 && cargo install --locked ic-host-sign --version 0.1.0 --features provider --root target/backup-sign-tools
	chmod 700 target/backup-sign-tools/bin/ic-sign

backup-signer-test:
	bash scripts/dev/test-backup-signer.sh
