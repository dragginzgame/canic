#!/usr/bin/env bash
# Run in the final delivery checkout. Testkit owns asset integrity and preparation.
set -euo pipefail
root=$(git rev-parse --show-toplevel)
source=$(git rev-parse --verify HEAD)
changes=$(git status --porcelain --untracked-files=normal)
[[ $(pwd -P) == "$root" && -z "$changes" ]] || {
    echo 'Publication admission requires a clean release checkout at its repository root.' >&2
    exit 1
}
directory=${IC_TESTKIT_SERVER_DIRECTORY:-$root/target/qualification/server/prepared}
binary=$directory/pocket-ic
if [[ "$directory" != /* || ! -d "$directory" || -L "$directory" ||
      ! -f "$binary" || ! -x "$binary" || -L "$binary" ]]; then
    printf 'Missing or invalid prepared simulator at %s. Run make qualification-setup in the release checkout.\n' "$directory" >&2
    exit 1
fi
for lock in Cargo.lock qualification/Cargo.lock; do
    object=$(git rev-parse --verify "$source:$lock")
    git diff --quiet "$source" -- "$lock"
    printf 'Publication input %s %s %s\n' "$source" "$lock" "$object"
done
for manifest in Cargo.toml qualification/Cargo.toml; do
    printf 'Fetching locked dependencies for %s\n' "$manifest"
    cargo fetch --locked --manifest-path "$manifest"
done
cargo run --quiet --locked --manifest-path qualification/Cargo.toml -p canic-qualification --example prepare_server -- --check || {
    status=$?
    printf 'Testkit could not admit the simulator at %s\n' "$directory" >&2
    exit "$status"
}
