#!/usr/bin/env bash
# Exercise the real publication-stage entrypoint in an isolated evidence store.
set -euo pipefail
root=$(git rev-parse --show-toplevel)
mkdir -p target
fixture=$(mktemp -d "$PWD/target/publication-retry.XXXXXX")
cd "$fixture"
git init -q
mkdir -p qualification scripts/{ci,dev} target/effects server
cp "$root"/scripts/ci/{observe-publication-effect.pl,publication-effect-stage.sh} scripts/ci/
cp "$root/scripts/dev/test-effects.sh" scripts/dev/
cp "$root/Cargo.lock" .
cp "$root/qualification/Cargo.lock" qualification/
ln -s "$root/vendor" vendor
git add Cargo.lock qualification scripts
git -c user.name=Fixture -c user.email=fixture@localhost commit -qm inputs
cp /usr/bin/true server/pocket-ic
export IC_TESTKIT_SERVER_DIRECTORY="$PWD/server"
export CANIC_EFFECT_PROBE_WASM="$PWD/original.wasm" CANIC_EFFECT_CANDIDATE_WASM="$PWD/candidate.wasm"
touch original.wasm candidate.wasm
# The second executable models a transient failure before any upload.
cat > retry.c <<'C'
#include <stdio.h>
int main(void) {
    FILE *file = fopen("first-attempt", "wx");
    if (!file) return 0;
    fclose(file);
    return 42;
}
C
cc retry.c -o retry
run() {
    bash scripts/ci/publication-effect-stage.sh "$1" "$2" >"$1.log" 2>&1
}
run creation /usr/bin/true
grep -F 'stage creation: executed' creation.log
status=0
run maintenance "$PWD/retry" || status=$?
[[ $status == 42 ]]
run creation /usr/bin/true
grep -F 'stage creation: reused' creation.log
run maintenance "$PWD/retry"
grep -F 'stage maintenance: executed' maintenance.log
printf 'changed' > candidate.wasm
run creation /usr/bin/true
grep -F 'stage creation: executed' creation.log
proof=$(jq -er .execution_proof "$(awk '/stage creation: executed/ {print $NF}' creation.log)")
printf 'damaged\n' > "$proof"
run creation /usr/bin/true
grep -F 'stage creation: executed' creation.log
