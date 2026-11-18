#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p target/effects
case "${1:-all}" in
    all|publication) tests=(--test creation --test funding --test maintenance) ;;
    maintenance) tests=(--test maintenance) ;;
    *) printf 'Usage: %s [all|maintenance|publication]\n' "$0" >&2; exit 2 ;;
esac
for variant in original maintenance-candidate; do
    features=''
    if [ "$variant" = maintenance-candidate ]; then features=maintenance-candidate; fi
    records=$(mktemp target/effects/build.XXXXXX.jsonl)
    if ! cargo build --locked --manifest-path qualification/Cargo.toml \
        -p canic-effect-probe --target wasm32-unknown-unknown --release --features "$features" \
        --message-format=json-render-diagnostics > "$records"; then
        jq -r 'select(.reason == "compiler-message") | .message.rendered // empty' "$records" >&2
        exit 1
    fi
    wasm=$(jq -sr '[.[] | select(.reason == "compiler-artifact" and .target.name == "canic_effect_probe")
        | .filenames[] | select(endswith(".wasm"))] | unique |
        if length == 1 then .[0] else error("expected one probe Wasm artifact") end' "$records")
    cp -- "$wasm" "target/effects/$variant.wasm"
done
cargo test --locked --manifest-path qualification/Cargo.toml -p canic-effect-probe --lib
export CANIC_EFFECT_PROBE_WASM="$root/target/effects/original.wasm"
export CANIC_EFFECT_CANDIDATE_WASM="$root/target/effects/maintenance-candidate.wasm"
test_command=(cargo test --locked --manifest-path qualification/Cargo.toml -p canic-qualification)
if [[ ${1:-all} != publication || $(uname -s) != Linux ]]; then
    exec "${test_command[@]}" "${tests[@]}" -- --nocapture
fi
# A target runner or Cargo configuration may change how the native binary is
# launched. Keep Cargo in charge for these unqualified execution environments.
ordinary=false
dir=$PWD
while :; do
    if [[ -e "$dir/.cargo/config" || -e "$dir/.cargo/config.toml" ]]; then ordinary=true; fi
    [[ "$dir" != / ]] || break
    dir=$(dirname "$dir")
done
cargo_home=${CARGO_HOME:-$HOME/.cargo}
if [[ -e "$cargo_home/config" || -e "$cargo_home/config.toml" || -n ${CARGO_BUILD_TARGET:-} ]] ||
    compgen -e | grep -E '^CARGO_TARGET_.*_RUNNER$' > /dev/null; then ordinary=true; fi
if [[ $ordinary == true ]]; then
    exec "${test_command[@]}" "${tests[@]}" -- --nocapture
fi

# Let Cargo select and build every executable afresh. Reuse only the execution.
records=$(mktemp target/effects/tests.XXXXXX.jsonl)
"${test_command[@]}" "${tests[@]}" --no-run --message-format=json-render-diagnostics > "$records"
for name in creation funding maintenance; do
    executable=$(jq -ser --arg name "$name" '
        [.[] | select(.reason == "compiler-artifact" and .target.name == $name and
          .target.kind == ["test"]) | .executable | strings] | unique |
        if length == 1 then .[0] else error("expected one test executable for " + $name) end' "$records")
    bash scripts/ci/publication-effect-stage.sh "$name" "$executable"
done
