#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/canic-packaged-cli.XXXXXX")"
HOST_CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
HOST_RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
PACKAGE_STAGING_ROOT="$ROOT/target/package"
TOOL_ROOT="$TMP_ROOT/tool-root"
PACKAGE_ROOT="$TOOL_ROOT/package-root"
DOWNSTREAM_ROOT="$TOOL_ROOT/downstream-root"
PROOF_HOME="$TMP_ROOT/home"
PROOF_TARGET_DIR="$TMP_ROOT/cargo-target"
PROOF_TMPDIR="$TMP_ROOT/tmp"
INSTALLED_CLI="$TMP_ROOT/install/bin/canic"
VERSION="$(
    cargo metadata --locked --offline --no-deps --format-version=1 --manifest-path "$ROOT/Cargo.toml" |
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
        canic-cli)
            cargo package --locked -p "$crate_name" --allow-dirty --no-verify \
                --config "patch.crates-io.canic-backup.path=\"$ROOT/crates/canic-backup\"" \
                --config "patch.crates-io.canic-core.path=\"$ROOT/crates/canic-core\"" \
                --config "patch.crates-io.canic-host.path=\"$ROOT/crates/canic-host\"" >/dev/null
            ;;
        *)
            cargo package --locked -p "$crate_name" --allow-dirty --no-verify >/dev/null
            ;;
    esac
}

populate_isolated_package_root() {
    mkdir -p "$PACKAGE_ROOT"

    local crate_archive=""
    for crate_archive in \
        "$PACKAGE_STAGING_ROOT/canic-backup-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-control-plane-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-core-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-macros-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-host-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-cli-$VERSION.crate"
    do
        [ -f "$crate_archive" ] || {
            echo "expected packaged crate archive at $crate_archive" >&2
            exit 1
        }
        tar -xzf "$crate_archive" -C "$PACKAGE_ROOT"
    done
}

prepare_tool_root() {
    mkdir -p "$TOOL_ROOT"

    cat > "$TOOL_ROOT/Cargo.toml" <<EOF
[workspace]
members = ["package-root/canic-cli-$VERSION"]
resolver = "3"

[patch.crates-io]
canic = { path = "package-root/canic-$VERSION" }
canic-backup = { path = "package-root/canic-backup-$VERSION" }
canic-control-plane = { path = "package-root/canic-control-plane-$VERSION" }
canic-core = { path = "package-root/canic-core-$VERSION" }
canic-host = { path = "package-root/canic-host-$VERSION" }
canic-macros = { path = "package-root/canic-macros-$VERSION" }
EOF
}

assert_packaged_tool_root() {
    if grep -R -Fq "$ROOT/crates" "$TOOL_ROOT/Cargo.toml" "$PACKAGE_ROOT"; then
        echo "packaged downstream CLI proof must not use repository crate paths" >&2
        exit 1
    fi

    if grep -R -Fq 'target/debug/canic' "$TOOL_ROOT/Cargo.toml" "$PACKAGE_ROOT"; then
        echo "packaged downstream CLI proof must not use target/debug/canic" >&2
        exit 1
    fi
}

prepare_downstream_root() {
    mkdir -p \
        "$DOWNSTREAM_ROOT/.icp/local/canisters/app" \
        "$DOWNSTREAM_ROOT/.icp/local/canisters/root" \
        "$DOWNSTREAM_ROOT/apps/downstream/app/src"

    cat > "$DOWNSTREAM_ROOT/Cargo.toml" <<EOF
[workspace]
members = ["apps/downstream/app"]
resolver = "3"

[workspace.package]
version = "0.0.0"

[patch.crates-io]
canic = { path = "$PACKAGE_ROOT/canic-$VERSION" }
canic-core = { path = "$PACKAGE_ROOT/canic-core-$VERSION" }
canic-control-plane = { path = "$PACKAGE_ROOT/canic-control-plane-$VERSION" }
canic-macros = { path = "$PACKAGE_ROOT/canic-macros-$VERSION" }

[profile.fast]
inherits = "release"
opt-level = 2
debug = false
EOF

    cat > "$DOWNSTREAM_ROOT/apps/downstream/app/Cargo.toml" <<EOF
[package]
name = "downstream-app"
version = { workspace = true }
edition = "2024"

[package.metadata.canic]
app = "downstream"
role = "app"

[lib]
crate-type = ["cdylib"]

[dependencies]
canic = { version = "=$VERSION", default-features = false, features = [] }
candid = "0.10"
serde = "1"
ic-cdk = "0.20"

[build-dependencies]
canic = { version = "=$VERSION", default-features = false, features = [] }
EOF
    printf 'fn main() { canic::build!("../canic.toml"); }\n' >"$DOWNSTREAM_ROOT/apps/downstream/app/build.rs"
    cat >"$DOWNSTREAM_ROOT/apps/downstream/app/src/lib.rs" <<'EOF'
use canic::prelude::*;
canic::start!();
async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}
#[canic_query(requires(caller::is_controller()))]
async fn packaged_probe() -> Result<u64, canic::Error> { Ok(7) }
canic::finish!();
EOF
    printf 'canisters: []\n' >"$DOWNSTREAM_ROOT/icp.yaml"

    cat > "$DOWNSTREAM_ROOT/apps/downstream/canic.toml" <<'EOF'
[app]
name = "downstream"

[roles.root]
kind = "root"

[roles.app]
kind = "canister"
package = "app"

[component_specs.app]
component_role = "app"
maximum_instances = 1

[component_groups.qualification.components.default]
component_spec = "app"

[component_group_deployments.qualification]
component_group = "qualification"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
EOF

    printf '\x00asm\x01\x00\x00\x00' | gzip -n > "$DOWNSTREAM_ROOT/.icp/local/canisters/app/app.wasm.gz"
}

run_packaged_canic() {
    (
        cd "$DOWNSTREAM_ROOT"
        HOME="$PROOF_HOME" \
            CARGO_HOME="$HOST_CARGO_HOME" \
            CARGO_TARGET_DIR="$PROOF_TARGET_DIR" \
            RUSTUP_HOME="$HOST_RUSTUP_HOME" \
            TMPDIR="$PROOF_TMPDIR" \
            "$INSTALLED_CLI" "$@"
    )
}

run_probe() {
    mkdir -p "$PROOF_HOME" "$PROOF_TARGET_DIR" "$PROOF_TMPDIR"
    assert_packaged_tool_root

    run_packaged_canic app list > "$TMP_ROOT/app-list.out"
    run_packaged_canic app role list downstream > "$TMP_ROOT/role-list.out"
    run_packaged_canic app role inspect downstream app > "$TMP_ROOT/app-inspect.out"
    run_packaged_canic fleet ensure --help > "$TMP_ROOT/fleet-ensure-help.out"
}

assert_probe_outputs() {
    grep -q 'downstream' "$TMP_ROOT/app-list.out" || {
        echo "expected packaged canic CLI to list downstream App" >&2
        sed -n '1,120p' "$TMP_ROOT/app-list.out" >&2
        exit 1
    }
    grep -q '2 (root, app)' "$TMP_ROOT/app-list.out" || {
        echo "expected packaged canic CLI to summarize root and app canisters" >&2
        sed -n '1,120p' "$TMP_ROOT/app-list.out" >&2
        exit 1
    }
    grep -q 'downstream.root' "$TMP_ROOT/role-list.out" || {
        echo "expected packaged canic CLI to list downstream.root" >&2
        sed -n '1,160p' "$TMP_ROOT/role-list.out" >&2
        exit 1
    }
    grep -q 'downstream.app' "$TMP_ROOT/role-list.out" || {
        echo "expected packaged canic CLI to list downstream.app" >&2
        sed -n '1,160p' "$TMP_ROOT/role-list.out" >&2
        exit 1
    }
    grep -q 'state: attached' "$TMP_ROOT/app-inspect.out" || {
        echo "expected packaged canic CLI to inspect app as attached" >&2
        sed -n '1,160p' "$TMP_ROOT/app-inspect.out" >&2
        exit 1
    }
    grep -q 'deploy artifact: eligible' "$TMP_ROOT/app-inspect.out" || {
        echo "expected packaged canic CLI to inspect app as build-eligible" >&2
        sed -n '1,160p' "$TMP_ROOT/app-inspect.out" >&2
        exit 1
    }
}

main() {
    ensure_packaged_crate canic-backup
    ensure_packaged_crate canic-core
    ensure_packaged_crate canic-control-plane
    ensure_packaged_crate canic-macros
    ensure_packaged_crate canic
    ensure_packaged_crate canic-host
    ensure_packaged_crate canic-cli
    populate_isolated_package_root

    prepare_tool_root
    prepare_downstream_root
    mkdir -p "$PROOF_HOME" "$PROOF_TARGET_DIR" "$PROOF_TMPDIR"
    # Install only extracted package contents; the consumer never executes a
    # workspace-built Canic binary or resolves Canic from repository paths.
    cargo generate-lockfile --offline --manifest-path "$TOOL_ROOT/Cargo.toml"
    CARGO_TARGET_DIR="$PROOF_TARGET_DIR" CARGO_PROFILE_DEV_DEBUG=1 \
        cargo install --locked --offline --debug --path "$PACKAGE_ROOT/canic-cli-$VERSION" \
        --root "$TMP_ROOT/install" --bin canic
    cargo generate-lockfile --offline --manifest-path "$DOWNSTREAM_ROOT/Cargo.toml"
    metadata="$(cargo metadata --locked --offline --format-version 1 --manifest-path "$DOWNSTREAM_ROOT/Cargo.toml")"
    jq -e --arg root "$PACKAGE_ROOT/" '
        [.packages[] | select(.name == "canic" or (.name | startswith("canic-")))]
        | length > 0 and all(.[]; .manifest_path | startswith($root))
    ' <<<"$metadata" >/dev/null
    run_probe
    assert_probe_outputs

    CANIC_PACKAGED_CLI="$INSTALLED_CLI" CANIC_PACKAGED_WORKSPACE="$DOWNSTREAM_ROOT" \
        CANIC_PACKAGED_CLI_SHA256="$(sha256sum "$INSTALLED_CLI" | cut -d ' ' -f1)" \
        CANIC_PACKAGED_TARGET="$PROOF_TARGET_DIR" \
        bash "$ROOT/scripts/ci/run-workspace-tests.sh" targeted-pocketic \
        pic::fleet_registry::baseline::tests::packaged_consumer::installed_package_build_deploy_recover_and_replay

    echo "packaged downstream CLI probe passed"
}

main "$@"
