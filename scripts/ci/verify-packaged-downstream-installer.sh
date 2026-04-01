#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/canic-packaged-installer.XXXXXX")"
PACKAGE_STAGING_ROOT="$ROOT/target/package"
TOOL_ROOT="$TMP_ROOT/tool-root"
PACKAGE_ROOT="$TOOL_ROOT/package-root"
DOWNSTREAM_ROOT="$TOOL_ROOT/downstream-root"
VERSION="$(
    cargo metadata --no-deps --format-version=1 --manifest-path "$ROOT/Cargo.toml" |
        python3 -c '
import json, sys

data = json.load(sys.stdin)
root = next(pkg for pkg in data["packages"] if pkg["name"] == "canic")
print(root["version"])
'
)"

cleanup() {
    rm -rf "$TMP_ROOT"
}

trap cleanup EXIT

ensure_packaged_crate() {
    local crate_name="$1"
    local crate_dir="$PACKAGE_STAGING_ROOT/$crate_name-$VERSION"
    if [ -d "$crate_dir" ]; then
        return
    fi

    cargo package -p "$crate_name" --allow-dirty --no-verify >/dev/null
}

populate_isolated_package_root() {
    mkdir -p "$PACKAGE_ROOT"

    local crate_dir=""
    for crate_dir in \
        "$PACKAGE_STAGING_ROOT/canic-cdk-$VERSION" \
        "$PACKAGE_STAGING_ROOT/canic-core-$VERSION" \
        "$PACKAGE_STAGING_ROOT/canic-dsl-macros-$VERSION" \
        "$PACKAGE_STAGING_ROOT/canic-memory-$VERSION" \
        "$PACKAGE_STAGING_ROOT/canic-types-$VERSION" \
        "$PACKAGE_STAGING_ROOT/canic-$VERSION" \
        "$PACKAGE_STAGING_ROOT/canic-installer-$VERSION"
    do
        [ -d "$crate_dir" ] || {
            echo "expected packaged crate directory at $crate_dir" >&2
            exit 1
        }
        cp -R "$crate_dir" "$PACKAGE_ROOT/"
    done
}

prepare_tool_root() {
    mkdir -p "$TOOL_ROOT"

    cat > "$TOOL_ROOT/Cargo.toml" <<EOF
[workspace]
members = ["package-root/canic-installer-$VERSION"]
resolver = "2"

[patch.crates-io]
canic = { path = "package-root/canic-$VERSION" }
canic-cdk = { path = "package-root/canic-cdk-$VERSION" }
canic-core = { path = "package-root/canic-core-$VERSION" }
canic-dsl-macros = { path = "package-root/canic-dsl-macros-$VERSION" }
canic-memory = { path = "package-root/canic-memory-$VERSION" }
canic-types = { path = "package-root/canic-types-$VERSION" }
EOF
}

prepare_downstream_root() {
    mkdir -p \
        "$DOWNSTREAM_ROOT/.dfx/local/canisters/app" \
        "$DOWNSTREAM_ROOT/.dfx/local/canisters/root" \
        "$DOWNSTREAM_ROOT/canisters/root"

    cat > "$DOWNSTREAM_ROOT/Cargo.toml" <<'EOF'
[workspace]
members = []
resolver = "2"

[workspace.package]
version = "0.0.0"
EOF

    cat > "$DOWNSTREAM_ROOT/canisters/root/Cargo.toml" <<'EOF'
[package]
name = "downstream-root"
version = { workspace = true }
edition = "2024"
EOF

    cat > "$DOWNSTREAM_ROOT/canisters/canic.toml" <<'EOF'
controllers = []
app_directory = []

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
subnet_directory = ["app"]
pool.minimum_size = 1

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "singleton"
EOF

    python3 - <<'PY' "$DOWNSTREAM_ROOT/.dfx/local/canisters/app/app.wasm.gz"
import gzip
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
path.parent.mkdir(parents=True, exist_ok=True)
with gzip.open(path, "wb") as f:
    f.write(b"\x00asm\x01\x00\x00\x00")
PY
}

run_probe() {
    (
        cd "$TOOL_ROOT"
        CANIC_WORKSPACE_ROOT="$DOWNSTREAM_ROOT" \
            cargo run --offline -q -p canic-installer --bin canic-emit-root-release-set-manifest >/dev/null
    )
}

assert_probe_outputs() {
    local manifest_path="$DOWNSTREAM_ROOT/.dfx/local/canisters/root/root.release-set.json"

    [ -s "$manifest_path" ] || {
        echo "expected emitted release-set manifest at $manifest_path" >&2
        exit 1
    }

    python3 - <<'PY' "$manifest_path"
import json
import pathlib
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text())
entries = manifest.get("entries", [])

assert manifest.get("release_version") == "0.0.0", manifest
assert len(entries) == 1, entries
assert entries[0]["role"] == "app", entries
assert entries[0]["artifact_relative_path"] == ".dfx/local/canisters/app/app.wasm.gz", entries
PY
}

main() {
    ensure_packaged_crate canic-cdk
    ensure_packaged_crate canic-core
    ensure_packaged_crate canic-dsl-macros
    ensure_packaged_crate canic-memory
    ensure_packaged_crate canic-types
    ensure_packaged_crate canic
    ensure_packaged_crate canic-installer
    populate_isolated_package_root

    prepare_tool_root
    prepare_downstream_root
    run_probe
    assert_probe_outputs

    echo "packaged downstream installer probe passed"
}

main "$@"
