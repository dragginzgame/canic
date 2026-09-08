#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MANIFEST="${1:-$ROOT/Cargo.toml}"

# Cargo resolves workspace inheritance, renames, optional dependencies and
# target-specific tables without compiling a crate or contacting the registry.
metadata="$(cargo metadata --manifest-path "$MANIFEST" --locked --offline --no-deps --format-version 1)"
violations="$(jq -r '
    .workspace_members as $ids
    | [.packages[] | select(.id as $id | $ids | index($id))] as $members
    | if ($members | length) == 0 then error("empty workspace metadata") else
        [ $members[] | select(.publish != []) as $owner
          | $owner.dependencies[] | select(.kind != "dev" and .path != null) as $dependency
          | $members[] | select(.publish == [])
          | select(.name == $dependency.name and .manifest_path == ($dependency.path + "/Cargo.toml"))
          | "\($owner.name): \($dependency.kind // "normal") dependency \($dependency.rename // $dependency.name) reaches unpublished workspace package \(.name)"
        ] | unique | .[]
      end
' <<<"$metadata")"

if [[ -n "$violations" ]]; then
    printf 'publish manifest boundary failed:\n%s\n' "$violations" >&2
    exit 1
fi
echo "publish manifest boundary passed (locked offline metadata; no compilation)"
