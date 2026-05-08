#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/canic-packaged-cli.XXXXXX")"
PACKAGE_STAGING_ROOT="$ROOT/target/package"
TOOL_ROOT="$TMP_ROOT/tool-root"
PACKAGE_ROOT="$TOOL_ROOT/package-root"
DOWNSTREAM_ROOT="$TOOL_ROOT/downstream-root"
VERSION="$(
    cargo metadata --no-deps --format-version=1 --manifest-path "$ROOT/Cargo.toml" |
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
    if [ "$crate_name" = "canic-cli" ]; then
        cargo package -p "$crate_name" --allow-dirty --no-verify \
            --config "patch.crates-io.canic-host.path=\"$ROOT/crates/canic-host\"" >/dev/null
    else
        cargo package -p "$crate_name" --allow-dirty --no-verify >/dev/null
    fi
}

populate_isolated_package_root() {
    mkdir -p "$PACKAGE_ROOT"

    local crate_archive=""
    for crate_archive in \
        "$PACKAGE_STAGING_ROOT/canic-cdk-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-backup-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-core-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-macros-$VERSION.crate" \
        "$PACKAGE_STAGING_ROOT/canic-memory-$VERSION.crate" \
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
resolver = "2"

[patch.crates-io]
canic = { path = "package-root/canic-$VERSION" }
canic-backup = { path = "package-root/canic-backup-$VERSION" }
canic-cdk = { path = "package-root/canic-cdk-$VERSION" }
canic-core = { path = "package-root/canic-core-$VERSION" }
canic-host = { path = "package-root/canic-host-$VERSION" }
canic-macros = { path = "package-root/canic-macros-$VERSION" }
canic-memory = { path = "package-root/canic-memory-$VERSION" }
EOF
}

prepare_downstream_root() {
    mkdir -p \
        "$DOWNSTREAM_ROOT/.dfx/local/canisters/app" \
        "$DOWNSTREAM_ROOT/.dfx/local/canisters/root" \
        "$DOWNSTREAM_ROOT/fleets/root"

    cat > "$DOWNSTREAM_ROOT/Cargo.toml" <<'EOF'
[workspace]
members = []
resolver = "2"

[workspace.package]
version = "0.0.0"
EOF

    cat > "$DOWNSTREAM_ROOT/fleets/root/Cargo.toml" <<'EOF'
[package]
name = "downstream-root"
version = { workspace = true }
edition = "2024"
EOF

    cat > "$DOWNSTREAM_ROOT/fleets/canic.toml" <<'EOF'
controllers = []
app_index = []

[fleet]
name = "downstream"

[app]
init_mode = "enabled"
[app.whitelist]

[auth.delegated_tokens]
enabled = false
ecdsa_key_name = "test_key_1"

[standards]
icrc21 = false

[subnets.prime]
auto_create = ["app"]
subnet_index = ["app"]
pool.minimum_size = 1

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "singleton"
EOF

    printf '\x00asm\x01\x00\x00\x00' | gzip -n > "$DOWNSTREAM_ROOT/.dfx/local/canisters/app/app.wasm.gz"
}

run_probe() {
    (
        cd "$TOOL_ROOT"
        CANIC_WORKSPACE_ROOT="$DOWNSTREAM_ROOT" \
            cargo run --offline -q -p canic-cli --bin canic -- fleet list > "$TMP_ROOT/fleet-list.out"
    )
}

assert_probe_outputs() {
    grep -q 'downstream' "$TMP_ROOT/fleet-list.out" || {
        echo "expected packaged canic CLI to list downstream fleet" >&2
        exit 1
    }
    grep -q '2 (root, app)' "$TMP_ROOT/fleet-list.out" || {
        echo "expected packaged canic CLI to summarize root and app canisters" >&2
        exit 1
    }
}

main() {
    ensure_packaged_crate canic-cdk
    ensure_packaged_crate canic-backup
    ensure_packaged_crate canic-core
    ensure_packaged_crate canic-macros
    ensure_packaged_crate canic-memory
    ensure_packaged_crate canic
    ensure_packaged_crate canic-host
    ensure_packaged_crate canic-cli
    populate_isolated_package_root

    prepare_tool_root
    prepare_downstream_root
    run_probe
    assert_probe_outputs

    echo "packaged downstream CLI probe passed"
}

main "$@"
