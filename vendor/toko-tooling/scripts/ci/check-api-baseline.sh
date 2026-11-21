#!/usr/bin/env bash
# Compare a consumer-generated API inventory with its reviewed directory.
set -euo pipefail

if [[ $# -lt 3 || $2 != -- || ! -d $1 || -L $1 ]]; then
    echo 'usage: check-api-baseline.sh BASELINE_DIRECTORY -- GENERATOR [ARG ...]' >&2
    exit 2
fi
baseline=$1
shift 2

mkdir -p target/tmp
candidate=$(mktemp -d "$PWD/target/tmp/api-baseline.XXXXXXXX")
# Keep generator stdout available to the consumer, including for other contracts.
printf 'API baseline candidate: %s\n' "$candidate" >&2
"$@" "$candidate"
if [[ ! -d $candidate || -L $candidate ]]; then
    echo 'Generator removed or replaced the candidate directory.' >&2
    exit 1
fi
git diff --no-index --no-renames --no-ext-diff --no-textconv --exit-code -- "$baseline" "$candidate" >&2
rm -r -- "$candidate"
