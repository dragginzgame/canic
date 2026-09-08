#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/canic-publish-manifest.XXXXXX")"
trap 'rm -rf "$FIXTURE"' EXIT
mkdir -p "$FIXTURE/public/src" "$FIXTURE/private/src" "$FIXTURE/bin"
touch "$FIXTURE/public/src/lib.rs" "$FIXTURE/private/src/lib.rs"
cat >"$FIXTURE/Cargo.toml" <<'TOML'
[workspace]
members = ["public", "private"]
resolver = "3"
[workspace.dependencies]
renamed = { package = "private-fixture", path = "private" }
TOML
cat >"$FIXTURE/private/Cargo.toml" <<'TOML'
[package]
name = "private-fixture"
version = "0.1.0"
edition = "2024"
publish = false
TOML
export MANIFEST_TEST_CARGO
MANIFEST_TEST_CARGO="$(command -v cargo)"
cat >"$FIXTURE/bin/cargo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" == metadata ]] || { echo 'manifest guard attempted compilation' >&2; exit 99; }
exec "$MANIFEST_TEST_CARGO" "$@"
SH
chmod +x "$FIXTURE/bin/cargo"

check_case() {
    local section="$1" expected="$2"
    cat >"$FIXTURE/public/Cargo.toml" <<'TOML'
[package]
name = "public-fixture"
version = "0.1.0"
edition = "2024"
TOML
    printf '%s\n' "$section" >>"$FIXTURE/public/Cargo.toml"
    local status=0
    PATH="$FIXTURE/bin:$PATH" bash "$ROOT/scripts/ci/check-publish-manifest-boundary.sh" \
        "$FIXTURE/Cargo.toml" >"$FIXTURE/result.log" 2>&1 || status=$?
    if [[ "$status" -ne "$expected" ]]; then
        cat "$FIXTURE/result.log" >&2
        echo "manifest boundary: expected $expected, got $status for $section" >&2
        exit 1
    fi
}

check_case '' 0
check_case $'[dev-dependencies]\nrenamed = { workspace = true }' 0
check_case $'[dependencies]\nprivate-fixture = "0.1.0"' 0
check_case $'[dependencies]\nrenamed = { workspace = true }' 1
check_case $'[dependencies]\nrenamed = { workspace = true, optional = true }' 1
check_case $'[build-dependencies]\nrenamed = { workspace = true }' 1
check_case $'[target.\'cfg(target_arch = "wasm32")\'.dependencies]\nrenamed = { workspace = true }' 1
check_case $'[target.\'cfg(target_os = "none")\'.build-dependencies]\nrenamed = { workspace = true }' 1
sed 's/publish = false/publish = true/' "$FIXTURE/private/Cargo.toml" >"$FIXTURE/private/published.toml"
mv "$FIXTURE/private/published.toml" "$FIXTURE/private/Cargo.toml"
check_case $'[dependencies]\nrenamed = { workspace = true }' 0
echo "publish manifest boundary fixtures passed"
