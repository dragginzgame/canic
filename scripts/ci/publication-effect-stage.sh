#!/usr/bin/env bash
set -euo pipefail
name=$1
executable=$2
source=$(git rev-parse --verify HEAD)
directory=${IC_TESTKIT_SERVER_DIRECTORY:-$PWD/target/qualification/server/prepared}
# Feature selection and package roster are part of the publication contract.
# The child sees only named non-secret inputs. Both Wasms and the executable
# were freshly selected from Cargo artifact messages before entering this stage.
environment=$(mktemp target/effects/environment.XXXXXX)
jq -cnS --arg source "$source" --arg exe "$executable" --arg directory "$directory" \
    --arg selection "${CARGO_PUBLISH_SELECTION:-all}" --arg features "${CARGO_PUBLISH_FEATURES:-}" \
    --arg all "${CARGO_PUBLISH_ALL_FEATURES:-0}" --arg defaults "${CARGO_PUBLISH_NO_DEFAULT_FEATURES:-0}" \
    '{PATH:"/usr/bin:/bin",LC_ALL:"C",CANIC_RELEASE_SOURCE:$source,CANIC_TEST_EXECUTABLE:$exe,
      CANIC_EFFECT_PROBE_WASM:env.CANIC_EFFECT_PROBE_WASM,CANIC_EFFECT_CANDIDATE_WASM:env.CANIC_EFFECT_CANDIDATE_WASM,
      IC_TESTKIT_SERVER_DIRECTORY:$directory,CANIC_PACKAGE_SELECTION:$selection,CANIC_PACKAGE_FEATURES:$features,
      CANIC_ALL_FEATURES:$all,CANIC_NO_DEFAULT_FEATURES:$defaults}' > "$environment"
perl vendor/toko-tooling/scripts/ci/run-validation-stage.pl --stage "$name" --group product \
    --contract canic-installed-effects/1 --source "$source" --state target/publication/effect-stages \
    --observe scripts/ci/observe-publication-effect.pl --environment "$environment" --max-bytes 536870912 \
    -- "$executable" --nocapture
