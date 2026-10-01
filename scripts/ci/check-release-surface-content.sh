#!/usr/bin/env bash
set -euo pipefail

# Reproduce only Cargo's governed version transaction from the validated source.
# Comparing Cargo's own output checks every manifest field and the full lock
# graph without maintaining a second TOML parser or a path-only exception.
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
source="${1:?validated source required}"
version="${2:?release version required}"
release_date="${3:?release date required}"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/canic-release-content.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT

jq -se --arg source "$source" --arg version "$version" --arg date "$release_date" '
    length == 1 and (.[0] |
    keys == ["date", "gate", "schema", "source", "version"] and
    .schema == 1 and .source == $source and .version == $version and .date == $date and
    (.date | type == "string" and test("^[0-9]{4}-[0-9]{2}-[0-9]{2}$")) and
    (.gate == "complete" or .gate == "fast"))
' "$ROOT/release-validation.json" >/dev/null
git -C "$ROOT" archive "$source" | tar -xf - -C "$scratch"
(
    cd "$scratch"
    cargo set-version --workspace --offline "$version" >/dev/null
    cargo update --workspace --offline >/dev/null
)
while IFS= read -r -d '' manifest; do
    relative="${manifest#"$scratch/"}"
    cmp -s "$manifest" "$ROOT/$relative" || {
        echo "release content differs from the governed version change: $relative" >&2
        exit 1
    }
done < <(find "$scratch" -name Cargo.toml -type f -print0)
cmp -s "$scratch/Cargo.lock" "$ROOT/Cargo.lock" || {
    echo 'release lock graph differs from the governed version change' >&2
    exit 1
}
sed -E \
    "s#CANIC_CLI_VERSION=\"\\\$\\{CANIC_CLI_VERSION:-[0-9]+\\.[0-9]+\\.[0-9]+\\}\"#CANIC_CLI_VERSION=\"\\\${CANIC_CLI_VERSION:-$version}\"#" \
    "$scratch/scripts/dev/install_dev.sh" >"$scratch/expected-installer"
cmp -s "$scratch/expected-installer" "$ROOT/scripts/dev/install_dev.sh" || {
    echo 'release installer differs from the governed version change' >&2
    exit 1
}
echo 'release content matches the exact validated source version transaction'
