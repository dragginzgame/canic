#!/usr/bin/env bash
# Materialize committed supplier bytes only; never package a sibling working tree.
set -euo pipefail
cd "$(dirname "$0")/../.."
root="$PWD"
source_repo="${1:-../ic-auth}"
revision=d5382b197ec794b9f30943a3beb978cbaffe8321
out="$root/target/auth-browser"
mkdir -p "$out"
scratch=$(mktemp -d "$out/source.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
git -C "$source_repo" archive "$revision" packages/client | tar -x -C "$scratch"
(
    cd "$scratch/packages/client"
    npm pack --ignore-scripts --pack-destination "$scratch" > "$scratch/package-name"
)
mv "$scratch/$(cat "$scratch/package-name")" "$out/ic-auth-client.tgz"
printf '%s\n' "$revision" > "$out/source-revision"
shasum -a 256 "$out/ic-auth-client.tgz"
