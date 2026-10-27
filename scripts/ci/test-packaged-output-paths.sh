#!/usr/bin/env bash
set -euo pipefail

# Exercise each proof's actual package producer/discovery with tiny local crates.
ROOT="$(cd "$(dirname "$0")/../.." && pwd -P)"
export PATH="$ROOT/.tools/host/bin:$PATH"
export CARGO_NET_OFFLINE=true
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/canic-package-output.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf -- "$FIXTURE"; else printf "Package fixture retained: %s\n" "$FIXTURE" >&2; fi' EXIT
mkdir -p "$FIXTURE/scripts/ci" "$FIXTURE/.cargo"
cp "$ROOT/scripts/ci/require-jq.sh" "$FIXTURE/scripts/ci/"
cp "$ROOT/.cargo/config.toml" "$FIXTURE/.cargo/"
cp "$ROOT/rust-toolchain.toml" "$FIXTURE/"
cat > "$FIXTURE/Cargo.toml" <<'TOML'
[workspace]
members = ["crates/canic", "crates/canic-contracts"]
resolver = "3"
[workspace.package]
rust-version = "1.90"
TOML
for package in canic canic-contracts; do
    mkdir -p "$FIXTURE/crates/$package/src"
    printf '[package]\nname = "%s"\nversion = "0.1.0"\nedition = "2024"\n' "$package" > "$FIXTURE/crates/$package/Cargo.toml"
    printf '// Package path fixture.\n' > "$FIXTURE/crates/$package/src/lib.rs"
done
(cd "$FIXTURE" && cargo generate-lockfile --offline)
for script in verify-packaged-downstream-cli.sh verify-packaged-downstream-wasm-store.sh; do
    # Keep the maintained initialization and functions; replace only orchestration.
    sed '$d' "$ROOT/scripts/ci/$script" > "$FIXTURE/scripts/ci/$script"
    cat >> "$FIXTURE/scripts/ci/$script" <<'SH'
ensure_packaged_crate canic-contracts
[[ "$PACKAGE_STAGING_ROOT" == "$EXPECTED_OUTPUT/package" ]]
test -s "$PACKAGE_STAGING_ROOT/canic-contracts-0.1.0.crate"
SH
    for selection in default relative absolute; do
        (
            unset CARGO_TARGET_DIR
            case "$selection" in
                default) EXPECTED_OUTPUT="$FIXTURE/target/cargo" ;;
                relative) export CARGO_TARGET_DIR='custom target'; EXPECTED_OUTPUT="$FIXTURE/custom target" ;;
                absolute) export CARGO_TARGET_DIR="$FIXTURE/external target"; EXPECTED_OUTPUT="$CARGO_TARGET_DIR" ;;
            esac
            export EXPECTED_OUTPUT
            # Start outside the workspace, so metadata and packaging must share cwd.
            cd /tmp
            bash "$FIXTURE/scripts/ci/$script"
        )
    done
done
echo 'Packaged proof output discovery passes for default, relative and absolute Cargo targets'
