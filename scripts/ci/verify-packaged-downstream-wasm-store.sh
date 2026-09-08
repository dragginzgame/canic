#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/canic-packaged-downstream-wasm-store.XXXXXX")"
HOST_CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
HOST_RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
PACKAGE_STAGING_ROOT="$ROOT/target/package"
GENERATED_TOOL_ROOT="$TMP_ROOT/tool-root-generated"
GENERATED_PACKAGE_ROOT="$GENERATED_TOOL_ROOT/package-root"
GENERATED_DOWNSTREAM_ROOT="$TMP_ROOT/downstream-generated"
GENERATED_TESTING_ROOT="$TMP_ROOT/testing-consumer-generated"
GENERATED_TARGET_DIR="$TMP_ROOT/cargo-target-generated"
PROOF_HOME="$TMP_ROOT/home"
PROOF_TMPDIR="$TMP_ROOT/tmp"
VERSION="$(
    cargo metadata --locked --no-deps --format-version=1 --manifest-path "$ROOT/Cargo.toml" |
        jq -r '.packages[] | select(.name == "canic") | .version'
)"

cleanup() {
    rm -rf "$TMP_ROOT"
}

trap cleanup EXIT

ensure_packaged_crate() {
    local crate_name="$1"
    local crate_archive="$PACKAGE_STAGING_ROOT/$crate_name-$VERSION.crate"
    rm -f "$crate_archive"
    case "$crate_name" in
        canic-control-plane)
            cargo package --locked -p "$crate_name" --allow-dirty --no-verify \
                --config "patch.crates-io.canic-core.path=\"$ROOT/crates/canic-core\"" >/dev/null
            ;;
        canic)
            cargo package --locked -p "$crate_name" --allow-dirty --no-verify \
                --config "patch.crates-io.canic-control-plane.path=\"$ROOT/crates/canic-control-plane\"" \
                --config "patch.crates-io.canic-core.path=\"$ROOT/crates/canic-core\"" \
                --config "patch.crates-io.canic-macros.path=\"$ROOT/crates/canic-macros\"" >/dev/null
            ;;
        canic-host)
            cargo package --locked -p "$crate_name" --allow-dirty --no-verify \
                --config "patch.crates-io.canic-control-plane.path=\"$ROOT/crates/canic-control-plane\"" \
                --config "patch.crates-io.canic-core.path=\"$ROOT/crates/canic-core\"" >/dev/null
            ;;
        *)
            cargo package --locked -p "$crate_name" --allow-dirty --no-verify >/dev/null
            ;;
    esac
}

prepare_lockfile() {
    local root="$1"
    local filter_platform="${2:-}"

    mkdir -p "$PROOF_HOME" "$PROOF_TMPDIR"
    (
        cd "$root"
        HOME="$PROOF_HOME" \
            CARGO_HOME="$HOST_CARGO_HOME" \
            RUSTUP_HOME="$HOST_RUSTUP_HOME" \
            TMPDIR="$PROOF_TMPDIR" \
            cargo +1.91.0 generate-lockfile --offline >/dev/null
        if [ -n "$filter_platform" ]; then
            HOME="$PROOF_HOME" \
                CARGO_HOME="$HOST_CARGO_HOME" \
                RUSTUP_HOME="$HOST_RUSTUP_HOME" \
                TMPDIR="$PROOF_TMPDIR" \
                cargo +1.91.0 metadata --offline --format-version=1 \
                    --filter-platform "$filter_platform" >/dev/null
        fi
    )
}

populate_isolated_package_root() {
    local package_root="$1"

    mkdir -p "$package_root"

    local crate_archive=""
    for crate_archive in \
        "$PACKAGE_STAGING_ROOT/canic-backup-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-control-plane-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-core-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-macros-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-host-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-$VERSION.crate"
    do
        [ -f "$crate_archive" ] || {
            echo "expected packaged crate archive at $crate_archive" >&2
            exit 1
        }
        tar -xzf "$crate_archive" -C "$package_root"
    done


}

prepare_tool_root() {
    local tool_root="$1"

    mkdir -p "$tool_root"

    cat > "$tool_root/Cargo.toml" <<EOF
[workspace]
members = ["package-root/canic-host-$VERSION"]
resolver = "2"

[patch.crates-io]
canic = { path = "package-root/canic-$VERSION" }
canic-backup = { path = "package-root/canic-backup-$VERSION" }
canic-control-plane = { path = "package-root/canic-control-plane-$VERSION" }
canic-core = { path = "package-root/canic-core-$VERSION" }
canic-host = { path = "package-root/canic-host-$VERSION" }
canic-macros = { path = "package-root/canic-macros-$VERSION" }
EOF
}

prepare_packaged_canic_patch_config() {
    local config_root="$1"
    local package_root="$2"

    mkdir -p "$config_root/.cargo"

    cat > "$config_root/.cargo/config.toml" <<EOF
[patch.crates-io]
canic = { path = "$package_root/canic-$VERSION" }
canic-control-plane = { path = "$package_root/canic-control-plane-$VERSION" }
canic-core = { path = "$package_root/canic-core-$VERSION" }
canic-macros = { path = "$package_root/canic-macros-$VERSION" }
EOF
}

assert_packaged_tool_root() {
    local tool_root="$1"
    local package_root="$2"

    if grep -R -Fq "$ROOT/crates" "$tool_root/Cargo.toml" "$package_root"; then
        echo "packaged downstream wasm_store proof must not use repository crate paths" >&2
        exit 1
    fi

    if grep -R -Fq 'target/debug/canic' "$tool_root/Cargo.toml" "$package_root"; then
        echo "packaged downstream wasm_store proof must not use target/debug/canic" >&2
        exit 1
    fi
}

prepare_downstream_root() {
    local downstream_root="$1"
    local package_root="$2"

    mkdir -p "$downstream_root/src" "$downstream_root/apps"
    prepare_packaged_canic_patch_config "$downstream_root" "$package_root"

    cp "$ROOT/apps/test/canic.toml" "$downstream_root/apps/canic.toml"

    cat > "$downstream_root/Cargo.toml" <<EOF
[package]
name = "canic-packaged-downstream-probe"
version = "0.0.0"
edition = "2024"
rust-version = "1.91.0"
publish = false
build = "build.rs"

[package.metadata.canic]
app = "packaged_probe"
role = "app"

[lib]
name = "canic_packaged_downstream_probe"
crate-type = ["cdylib"]

[dependencies]
candid = { version = "0.10", default-features = false }
canic = { path = "$package_root/canic-$VERSION", default-features = false, features = [] }
ic-cdk = "0.20"

[build-dependencies]
canic = { path = "$package_root/canic-$VERSION", default-features = false }
EOF

    cat > "$downstream_root/build.rs" <<'EOF'
fn main() {
    canic::build!("canic.toml");
}
EOF

    cat > "$downstream_root/canic.toml" <<'EOF'
[app]
name = "packaged_probe"

[auth.delegated_tokens]
enabled = false

[roles.root]
kind = "root"

[roles.app]
kind = "canister"
package = "."

[component_specs.app]
component_role = "app"
maximum_instances = 1
EOF

    cat > "$downstream_root/src/lib.rs" <<'EOF'
use candid::Principal;
use canic::Error;

async fn canic_setup() {}

async fn canic_install(_: Option<Vec<u8>>) {}

async fn canic_upgrade() {}

canic::start!();

#[canic::canic_query(public)]
fn packaged_probe(subject: Principal) -> Result<Principal, Error> {
    Ok(subject)
}

canic::finish!();
EOF
}

prepare_testing_consumer() {
    local consumer_root="$1"
    local package_root="$2"

    mkdir -p "$consumer_root/src"
    prepare_packaged_canic_patch_config "$consumer_root" "$package_root"

    cat > "$consumer_root/Cargo.toml" <<EOF
[package]
name = "canic-packaged-testing-consumer"
version = "0.0.0"
edition = "2024"
rust-version = "1.91.0"
publish = false

[dependencies]
thiserror = "=2.0.20"
EOF

    cat > "$consumer_root/src/lib.rs" <<'EOF'
use canic::testing::{
    ManagedAppFixture, ManagedAppQualificationError, ManagedAppQualificationInput,
    ManagedComponentGroupFixture, ManagedComponentGroupQualificationError,
    ManagedComponentGroupQualificationInput, ManagedComponentNode,
    ManagedRoleQualificationArtifact, StandaloneAppFixture, install_managed_app,
    install_managed_component_group, install_standalone_app,
};
use std::time::Duration;

pub fn compile_managed_consumer(
    input: ManagedAppQualificationInput<'_>,
) -> Result<ManagedAppFixture, ManagedAppQualificationError> {
    install_managed_app(input)
}

pub fn compile_standalone_consumer(
    wasm: Vec<u8>,
) -> Result<StandaloneAppFixture, ManagedAppQualificationError> {
    let fixture = install_standalone_app(wasm, 10_000_000_000_000);
    fixture.upgrade_same_release(Duration::from_secs(300))?;
    Ok(fixture)
}

pub fn compile_managed_component_group_consumer(
    input: ManagedComponentGroupQualificationInput<'_>,
) -> Result<ManagedComponentGroupFixture, ManagedComponentGroupQualificationError> {
    let fixture = install_managed_component_group(input)?;
    let _: Vec<ManagedComponentNode> = fixture.nodes();
    let _: Option<ManagedRoleQualificationArtifact> = None;
    Ok(fixture)
}
EOF
}

enable_and_resolve_packaged_testing_consumer() {
    local consumer_root="$1"
    local package_root="$2"
    local target_dir="$3-testing"
    local manifest="$consumer_root/Cargo.toml"

    grep -Fq 'thiserror = "=2.0.20"' "$manifest" || {
        echo "expected testing consumer to begin from the retained current 2.x lock graph" >&2
        exit 1
    }
    sed -i \
        "s|thiserror = \"=2.0.20\"|canic = { path = \"$package_root/canic-$VERSION\", default-features = false, features = [\"testing\"] }|" \
        "$manifest"
    if grep -Eq '^thiserror[[:space:]]*=' "$manifest"; then
        echo "packaged testing consumer must not override a transitive dependency" >&2
        exit 1
    fi

    mkdir -p "$PROOF_HOME" "$target_dir" "$PROOF_TMPDIR"
    (
        cd "$consumer_root"
        HOME="$PROOF_HOME" \
            CARGO_HOME="$HOST_CARGO_HOME" \
            CARGO_TARGET_DIR="$target_dir" \
            RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-D warnings" \
            RUSTUP_HOME="$HOST_RUSTUP_HOME" \
            TMPDIR="$PROOF_TMPDIR" \
            cargo +1.91.0 check --offline >/dev/null
    )
}

run_packaged_testing_consumer() {
    local consumer_root="$1"
    local target_dir="$2-testing"

    mkdir -p "$PROOF_HOME" "$target_dir" "$PROOF_TMPDIR"
    (
        cd "$consumer_root"
        HOME="$PROOF_HOME" \
            CARGO_HOME="$HOST_CARGO_HOME" \
            CARGO_TARGET_DIR="$target_dir" \
            RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-D warnings" \
            RUSTUP_HOME="$HOST_RUSTUP_HOME" \
            TMPDIR="$PROOF_TMPDIR" \
            cargo +1.91.0 check --offline --locked >/dev/null
    )
}

run_packaged_canister_probe() {
    local downstream_root="$1"
    local target_dir="$2-canister"
    local wasm_path="$target_dir/wasm32-unknown-unknown/debug/canic_packaged_downstream_probe.wasm"
    local local_did="$downstream_root/packaged-probe.local.did"
    local ic_did="$downstream_root/packaged-probe.ic.did"

    mkdir -p "$PROOF_HOME" "$target_dir" "$PROOF_TMPDIR"

    command -v candid-extractor >/dev/null || {
        echo "packaged downstream Canister proof requires candid-extractor" >&2
        exit 1
    }
    (
        cd "$downstream_root"
        CANIC_INTERNAL_CANDID_BUILD=1 \
        CANIC_ROLE_CONTRACT_VALIDATED=1 \
            ICP_ENVIRONMENT=local \
            HOME="$PROOF_HOME" \
            CARGO_HOME="$HOST_CARGO_HOME" \
            CARGO_TARGET_DIR="$target_dir" \
            RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-D warnings" \
            RUSTUP_HOME="$HOST_RUSTUP_HOME" \
            TMPDIR="$PROOF_TMPDIR" \
            cargo +1.91.0 build --offline --locked --target wasm32-unknown-unknown >/dev/null
    )
    candid-extractor "$wasm_path" >"$local_did"
    grep -Fq 'packaged_probe' "$local_did" || {
        echo "packaged downstream local Wasm omitted its typed Candid endpoint" >&2
        exit 1
    }

    (
        cd "$downstream_root"
        CANIC_ROLE_CONTRACT_VALIDATED=1 \
            ICP_ENVIRONMENT=ic \
            HOME="$PROOF_HOME" \
            CARGO_HOME="$HOST_CARGO_HOME" \
            CARGO_TARGET_DIR="$target_dir" \
            RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-D warnings" \
            RUSTUP_HOME="$HOST_RUSTUP_HOME" \
            TMPDIR="$PROOF_TMPDIR" \
            cargo +1.91.0 build --offline --locked --target wasm32-unknown-unknown >/dev/null
    )
    if candid-extractor "$wasm_path" >"$ic_did" 2>/dev/null &&
        grep -Fq 'packaged_probe' "$ic_did"; then
        echo "packaged downstream IC Wasm unexpectedly retained local Candid export" >&2
        exit 1
    fi
}

run_probe() {
    local tool_root="$1"
    local package_root="$2"
    local downstream_root="$3"
    local target_dir="$4"
    local role="${5:-wasm_store}"
    local config_path="$downstream_root/canic.toml"

    mkdir -p "$PROOF_HOME" "$target_dir" "$PROOF_TMPDIR"
    assert_packaged_tool_root "$tool_root" "$package_root"

    (
        cd "$tool_root"
        HOME="$PROOF_HOME" \
            CARGO_HOME="$HOST_CARGO_HOME" \
            CARGO_TARGET_DIR="$target_dir" \
            RUSTUP_HOME="$HOST_RUSTUP_HOME" \
            TMPDIR="$PROOF_TMPDIR" \
            cargo run --manifest-path "$tool_root/Cargo.toml" --offline --locked -q -p canic-host \
                --example build_artifact -- "$role" fast "$downstream_root" \
                "$downstream_root" "$config_path" >/dev/null
    )
}

assert_fleet_probe_outputs() {
    local downstream_root="$1"
    local package_root="$2"
    local role="$3"
    local feature="$4"
    local package="$5"
    local manifest="$downstream_root/.canic/generated/$package/Cargo.toml"
    local artifacts="$downstream_root/.icp/local/canisters/$role"
    for extension in wasm wasm.gz did; do
        test -s "$artifacts/$role.$extension"
    done
    cargo metadata --locked --offline --no-deps --format-version=1 --manifest-path "$manifest" |
        jq -e --arg path "$package_root/canic-$VERSION" --arg package "$package" \
            --arg version "$VERSION" --arg feature "$feature" '
            .packages[] | select(.name == $package) |
            .version == $version and .publish == [] and
            any(.dependencies[];
                .name == "canic" and .kind == null and .path == $path and
                .uses_default_features == false and .features == [$feature])
        ' >/dev/null
    if grep -Fq "$ROOT/crates" "$manifest"; then
        echo "generated Fleet packages must depend on isolated packaged source" >&2
        exit 1
    fi
    if [ "$role" != root ]; then
        cmp "$package_root/canic-$VERSION/candid/$role.did" "$artifacts/$role.did"
    fi
    echo "packaged generated $role artifact passed"
}

main() {
    local scope="${1:-all}"
    local role feature package
    case "$scope" in
        all|fleet|root) ;;
        *) echo "usage: $0 [all|fleet|root]" >&2; exit 2 ;;
    esac
    ensure_packaged_crate canic-backup
    ensure_packaged_crate canic-control-plane
    ensure_packaged_crate canic-core
    ensure_packaged_crate canic-macros
    ensure_packaged_crate canic-host
    ensure_packaged_crate canic

    populate_isolated_package_root "$GENERATED_PACKAGE_ROOT"
    prepare_tool_root "$GENERATED_TOOL_ROOT"
    prepare_downstream_root "$GENERATED_DOWNSTREAM_ROOT" "$GENERATED_PACKAGE_ROOT"
    prepare_lockfile "$GENERATED_TOOL_ROOT"
    prepare_lockfile "$GENERATED_DOWNSTREAM_ROOT" wasm32-unknown-unknown
    if [ "$scope" = root ]; then
        run_probe "$GENERATED_TOOL_ROOT" "$GENERATED_PACKAGE_ROOT" "$GENERATED_DOWNSTREAM_ROOT" "$GENERATED_TARGET_DIR" root
        assert_fleet_probe_outputs "$GENERATED_DOWNSTREAM_ROOT" "$GENERATED_PACKAGE_ROOT" root control-plane canic-fleet-root
        return
    fi
    if [ "$scope" = all ]; then
        prepare_testing_consumer "$GENERATED_TESTING_ROOT" "$GENERATED_PACKAGE_ROOT"
        prepare_lockfile "$GENERATED_TESTING_ROOT"
        enable_and_resolve_packaged_testing_consumer \
            "$GENERATED_TESTING_ROOT" \
            "$GENERATED_PACKAGE_ROOT" \
            "$GENERATED_TARGET_DIR"
        run_packaged_canister_probe "$GENERATED_DOWNSTREAM_ROOT" "$GENERATED_TARGET_DIR"
        run_packaged_testing_consumer "$GENERATED_TESTING_ROOT" "$GENERATED_TARGET_DIR"
    fi
    for role in fleet_coordinator root wasm_store; do
        run_probe "$GENERATED_TOOL_ROOT" "$GENERATED_PACKAGE_ROOT" "$GENERATED_DOWNSTREAM_ROOT" "$GENERATED_TARGET_DIR" "$role"
        case "$role" in
            fleet_coordinator) feature=fleet-coordinator-canister; package=canic-fleet-coordinator ;;
            root) feature=control-plane; package=canic-fleet-root ;;
            wasm_store) feature=wasm-store-canister; package=canic-fleet-wasm-store ;;
        esac
        assert_fleet_probe_outputs "$GENERATED_DOWNSTREAM_ROOT" "$GENERATED_PACKAGE_ROOT" "$role" "$feature" "$package"
    done
    echo "packaged generated Fleet artifact proof passed"
}

main "$@"
