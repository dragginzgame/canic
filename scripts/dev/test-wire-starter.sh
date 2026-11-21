#!/usr/bin/env bash
# Exercise the downstream command in a fresh source workspace with zero fleets.
set -euo pipefail
repo=$(pwd -P)
mkdir -p target/tmp
scratch=$(mktemp -d "$repo/target/tmp/wire-starter.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo/target}"
cli() { cargo run --quiet --locked --manifest-path "$repo/Cargo.toml" -p canic-cli -- "$@"; }
cli app init sample --source "$scratch/app" --canic-source "$repo"
mkdir "$scratch/candidates"
export TMPDIR="$scratch/candidates"
cd "$scratch/app"
check() { cli app check-wire; }
candidate() {
    if check >"$scratch/result" 2>&1; then
        echo "Expected Wire contract drift" >&2
        exit 1
    fi
    cat "$scratch/result" >&2
    sed -n 's/^ *Review generated candidate: //p' "$scratch/result"
}
baseline=wire-baseline/sample/contracts.json
check
# An optional query mounted in a dependency must appear in both the Wasm and manifest.
sed -i.bak '/^package = "starter-app"$/a\
features = ["canic/endpoint-metrics"]
' canic.toml
generated=$(candidate)
jq -e '.app.metrics.mode == "query" and (.app.metrics.reply.schema | length > 1)' "$generated" >/dev/null
cp "$generated" "$baseline"
check
# Field-only change in a disposable actor. Every export name, mode and ID must survive.
sed -i.bak 's/pub message: String/pub message: String, pub revision: u32/;s/message: format!/revision: 1, message: format!/' starter-app/src/lib.rs
generated=$(candidate)
jq -e --slurp '
  (map(.app | map_values({mode,id})) | .[0] == .[1]) and
  (.[0].app.greet.reply.schema != .[1].app.greet.reply.schema) and
  (.[0].app.metrics == .[1].app.metrics)
' "$baseline" "$generated" >/dev/null
# An installed-only export missing from native inspection must fail before baseline comparison.
cat >>starter-app/src/lib.rs <<'RS'

#[cfg(target_arch = "wasm32")]
#[canic::query]
fn wasm_only() -> u32 { 1 }
RS
if check >"$scratch/result" 2>&1; then
    echo "Expected Wasm export mismatch" >&2
    exit 1
fi
grep -F "disagree with Wasm exports" "$scratch/result"
test ! -e .canic
