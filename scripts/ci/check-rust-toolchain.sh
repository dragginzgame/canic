#!/usr/bin/env bash
set -euo pipefail

# Read-only compiler admission. Explicit setup owns installation; the manifest
# and pinned toolchain own the policy, including independent managed consumers.
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
export PATH="$root/.tools/host/bin:$PATH"
export RUSTUP_AUTO_INSTALL=0
cd "$root"
minimum="$(bash scripts/ci/read-cargo-workspace-rust-version.sh Cargo.toml)"
policy="$(yq -p toml -o json '.' rust-toolchain.toml)"
channel="$(jq -ers 'if length == 1 then .[0].toolchain.channel else error("ambiguous toolchain policy") end | select(type == "string")' <<<"$policy")"
[[ "$channel" == "$minimum" ]] || { echo 'Pinned compiler and supported Rust floor differ' >&2; exit 1; }
for consumer in consumer embedded-consumer; do
    selected="$(bash scripts/ci/read-cargo-workspace-rust-version.sh "integrations/blob-service/$consumer/Cargo.toml")"
    [[ "$selected" == "$minimum" ]] || { echo "Managed $consumer compiler floor differs" >&2; exit 1; }
done
compiler="$("${RUSTC-rustc}" --version --verbose)"
cargo_identity="$(cargo --version --verbose)"
[[ "$(awk '$1 == "release:" { print $2 }' <<<"$compiler")" == "$minimum" ]] || {
    printf 'Selected rustc does not match required Rust %s\n%s\n' "$minimum" "$compiler" >&2; exit 1;
}
[[ "$(awk 'NR == 1 { print $2 }' <<<"$cargo_identity")" == "$minimum" ]] || {
    printf 'Selected Cargo does not match required Rust %s\n%s\n' "$minimum" "$cargo_identity" >&2; exit 1;
}
metadata="$(cargo metadata --locked --offline --no-deps --format-version 1)"
jq -e --arg minimum "$minimum" '
    . as $metadata |
    [.packages[] | select(.id as $id | $metadata.workspace_members | index($id))] |
    length > 0 and all(.[]; .rust_version == $minimum)
' <<<"$metadata" > /dev/null || { echo 'Workspace member compiler declarations differ from the supported floor' >&2; exit 1; }
printf 'Rust compiler policy admitted: %s\n%s\n%s\n' "$minimum" "$compiler" "$cargo_identity"
