#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd -P)"
scratch_parent="${TMPDIR:-$ROOT/target/tmp}"
mkdir -p "$scratch_parent"
fixture="$(mktemp -d "$scratch_parent/canic-integration-workspace.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf -- "$fixture"; else echo "Integration fixture retained: $fixture" >&2; fi' EXIT
repository="$fixture/repository with spaces"
mkdir -p "$repository/scripts/dev" "$repository/source"
cp "$ROOT/scripts/dev/prepare-integration-workspace.pl" "$repository/scripts/dev/"
git init -q "$repository"
printf 'target/\n.canic/\n' > "$repository/.gitignore"
printf 'original\n' > "$repository/source/input"
mkdir -p "$repository/source/target" "$repository/.canic"
printf 'compiler output\n' > "$repository/source/target/ignored"
printf 'private state\n' > "$repository/.canic/ignored"
script="$repository/scripts/dev/prepare-integration-workspace.pl"
workspace="$(perl "$script" --path)"
[[ "$workspace" == "$repository/target/integrations/workspace" && ! -e "$repository/target" ]]
[[ "$(perl "$script" --prepare)" == "$workspace" ]]
cmp "$repository/source/input" "$workspace/source/input"
[[ ! -e "$workspace/.git" && ! -e "$workspace/.canic" && ! -e "$workspace/source/target" ]]

# Source refreshes preserve unchanged input identity and reusable build output.
touch -t 200001010000 "$workspace/source/input"
mkdir -p "$workspace/source/target"
printf 'reuse\n' > "$workspace/source/target/cache"
perl "$script" --prepare > /dev/null
[[ "$workspace/source/input" -ot "$repository/source/input" ]]
printf 'edited working tree\n' > "$repository/source/input"
perl "$script" --prepare > /dev/null
cmp "$repository/source/input" "$workspace/source/input"
[[ -f "$workspace/source/target/cache" ]]
rm "$repository/source/input"
perl "$script" --prepare > /dev/null
[[ ! -e "$workspace/source/input" && -f "$workspace/source/target/cache" ]]

# Commands see the prepared source, and their failure status reaches the caller.
printf 'current\n' > "$repository/source/input"
perl "$script" --run bash -c 'cmp "$1/source/input" "$CANIC_INTEGRATION_WORKSPACE/source/input"' _ "$repository"
if perl "$script" --run bash -c 'exit 23'; then
    exit 1
else
    [[ $? == 23 ]]
fi

# Never follow source/output aliases or overwrite an unowned directory.
ln -s "$repository/.canic/ignored" "$repository/source/alias"
if perl "$script" --prepare > "$fixture/source-alias.log" 2>&1; then exit 1; fi
rm "$repository/source/alias"
mv "$workspace/source/input" "$fixture/input"
ln -s "$fixture/input" "$workspace/source/input"
if perl "$script" --prepare > "$fixture/output-alias.log" 2>&1; then exit 1; fi
rm "$workspace/source/input"
mv "$fixture/input" "$workspace/source/input"
rm "$repository/target/integrations/source-files"
if perl "$script" --prepare > "$fixture/unowned.log" 2>&1; then exit 1; fi
echo 'Disposable integration source, cache reuse, child status and path custody passed'
