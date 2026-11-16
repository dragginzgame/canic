#!/usr/bin/env bash
# Explicit download only; tests use the supplier's offline asset admission.
set -euo pipefail
umask 077
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/qualification
attempt=$(mktemp -d target/qualification/setup.XXXXXX)
cargo build --locked --manifest-path qualification/Cargo.toml -p canic-qualification \
    --example prepare_server --message-format=json-render-diagnostics > "$attempt/build.jsonl"
prepare=$(jq -ers '[.[] | select(.reason == "compiler-artifact" and .target.name == "prepare_server") | .executable | select(. != null)] | unique | if length == 1 then .[0] else error("expected one preparation executable") end' "$attempt/build.jsonl")
if "$prepare" --check > "$attempt/check.stdout" 2> "$attempt/check.stderr"; then
    cat "$attempt/check.stdout"
    exit 0
fi
"$prepare" --asset > "$attempt/asset.txt"
url=$(sed -n '1p' "$attempt/asset.txt")
checksum=$(sed -n '2p' "$attempt/asset.txt")
source vendor/toko-tooling/scripts/dev/tool-store.sh
tool_store_download "$url" "$attempt/archive.gz" "$checksum"
# Supplier preparation refuses changed existing bytes; it never overwrites them.
"$prepare" "$attempt/archive.gz" "$(command -v gzip)"
