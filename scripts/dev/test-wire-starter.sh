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
# Link a real retained Once and prove its private update stays out of the client.
sed -i.bak '/^\[dependencies\]$/a\
ic-timers = "0.46.13"
' starter-app/Cargo.toml
sed -i.bak 's/"readiness"/"readiness", "lifecycle"/' starter-app/Cargo.toml
echo 'mod lifecycle;' >>starter-app/src/lib.rs
cat >starter-app/src/lifecycle.rs <<'RS'
canic::mount_lifecycle!(install = timers, reopen = timers);
fn timers() {
    ic_timers::initialize().unwrap();
    ic_timers::bind_once(["sample", "app", "once"], ic_timers::Lifetime::Retained, async || {
        ic_timers::OnceResult { observation: ic_timers::Observation::NoWork, next: ic_timers::OnceNext::Stop }
    }).unwrap();
}
RS
cargo metadata --offline --format-version 1 >/dev/null
check
cli app export-wire --out generated
cli artifact endpoints artifacts/app.wasm --json | jq -e '
  any(.endpoints[]; .name == "__ic_scheduling_once" and .mode == "update" and .argument == null) and
  any(.endpoints[]; .name == "greet" and .argument == "String" and .reply == "Greeting")
' >/dev/null
grep -F 'transport.query(canisterId, "greet"' generated/service-0.ts >/dev/null
if grep -Fq '__ic_scheduling_once' generated/*.ts; then
    echo "Timer executor leaked into the generated client" >&2
    exit 1
fi
# The mandatory reader has a typed reply in the starter baseline.
jq -e '.app.logs.mode == "update" and .app.metrics.mode == "query" and (.app.metrics.reply.schema | length > 1)' "$baseline" >/dev/null
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
