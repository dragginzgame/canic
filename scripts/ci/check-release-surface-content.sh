#!/usr/bin/env bash
set -euo pipefail

# shellcheck source=scripts/ci/require-jq.sh
source "$(dirname "${BASH_SOURCE[0]}")/../ci/require-jq.sh"
require_jq

# Reproduce only Cargo's governed version transaction from the validated source.
# Comparing Cargo's own output checks every manifest field and the full lock
# graph without maintaining a second TOML parser or a path-only exception.
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
source="${1:?validated source required}"
version="${2:?release version required}"
release_date="${3:?release date required}"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/canic-release-content.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
actual_root="$ROOT"
if [[ -n "${4:-}" ]]; then
    [[ "$4" =~ ^([0-9a-f]{40}|[0-9a-f]{64})$ ]] || exit 2
    actual_root="$scratch/committed"
    mkdir "$actual_root"
    git -C "$ROOT" archive "$4" | tar -xf - -C "$actual_root"
fi

# jq expressions use literal variable names.
# shellcheck disable=SC2016
"$JQ_BIN" -se --arg source "$source" --arg version "$version" --arg date "$release_date" '
    length == 1 and (.[0] |
    keys == ["date", "gate", "schema", "source", "version"] and
    .schema == 1 and .source == $source and .version == $version and .date == $date and
    (.date | type == "string" and test("^[0-9]{4}-[0-9]{2}-[0-9]{2}$")) and
    (.gate == "complete" or .gate == "fast"))
' "$actual_root/release-validation.json" >/dev/null
mkdir "$scratch/expected"
git -C "$ROOT" archive "$source" | tar -xf - -C "$scratch/expected"
consumer_locks=()
consumer_lock_paths="$(bash "$ROOT/scripts/release/adapter.sh" consumer-locks)"
[[ -n "$consumer_lock_paths" ]]
while IFS= read -r path; do consumer_locks[${#consumer_locks[@]}]="$path"; done <<< "$consumer_lock_paths"
(
    cd "$scratch/expected"
    cargo metadata --locked --offline --no-deps --format-version 1 > "$scratch/expected/metadata.json"
    cp -p Cargo.lock "$scratch/expected/source.lock"
    previous="$(bash "$ROOT/scripts/ci/read-cargo-workspace-version.sh" "$scratch/expected/Cargo.toml")"
    for index in "${!consumer_locks[@]}"; do
        lock="${consumer_locks[$index]}"
        cargo metadata --locked --offline --format-version 1 --manifest-path "${lock%/Cargo.lock}/Cargo.toml" > "$scratch/consumer-$index.json"
        cp -p "$lock" "$scratch/consumer-$index.lock"
    done
    cargo set-version --workspace --offline "$version" >/dev/null
    bash "$ROOT/scripts/release/rewrite-owned-lock.sh" "$scratch/expected/metadata.json" "$scratch/expected/source.lock" "$previous" "$version" > Cargo.lock
    for index in "${!consumer_locks[@]}"; do
        lock="${consumer_locks[$index]}"
        bash "$ROOT/scripts/release/rewrite-owned-lock.sh" "$scratch/expected/metadata.json" \
            "$scratch/consumer-$index.lock" "$previous" "$version" "$scratch/consumer-$index.json" > "$lock"
        cargo metadata --locked --offline --no-deps --format-version 1 --manifest-path "${lock%/Cargo.lock}/Cargo.toml" >/dev/null
    done
)
while IFS= read -r -d '' manifest; do
    relative="${manifest#"$scratch/expected/"}"
    cmp -s "$manifest" "$actual_root/$relative" || {
        echo "release content differs from the governed version change: $relative" >&2
        exit 1
    }
done < <(find "$scratch/expected" -name Cargo.toml -type f -print0)
cmp -s "$scratch/expected/Cargo.lock" "$actual_root/Cargo.lock" || {
    echo 'release lock graph differs from the governed version change' >&2
    exit 1
}
for lock in "${consumer_locks[@]}"; do
    cmp -s "$scratch/expected/$lock" "$actual_root/$lock" || {
        echo "release consumer lock graph differs from the governed version change: $lock" >&2
        exit 1
    }
done
sed -E \
    "s#CANIC_CLI_VERSION=\"\\\$\\{CANIC_CLI_VERSION:-[0-9]+\\.[0-9]+\\.[0-9]+\\}\"#CANIC_CLI_VERSION=\"\\\${CANIC_CLI_VERSION:-$version}\"#" \
    "$scratch/expected/scripts/dev/install_dev.sh" >"$scratch/expected-installer"
cmp -s "$scratch/expected-installer" "$actual_root/scripts/dev/install_dev.sh" || {
    echo 'release installer differs from the governed version change' >&2
    exit 1
}
echo 'release content matches the exact validated source version transaction'
