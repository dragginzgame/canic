#!/usr/bin/env bash
# Fail fast in a clean checkout, without downloading a dependency or starting IC.
set -euo pipefail
root=$(git rev-parse --show-toplevel)
mkdir -p target
fixture=$(mktemp -d "$PWD/target/publication-preflight.XXXXXX")
git clone --quiet --local --shared "$root" "$fixture/checkout"
cd "$fixture/checkout"
export IC_TESTKIT_SERVER_DIRECTORY="$fixture/prepared"
mkdir "$fixture/bin"
cat > "$fixture/bin/cargo" <<'CARGO'
#!/usr/bin/env bash
printf '%s %s\n' "${CARGO_NET_OFFLINE-unset}" "$*" >> "$CARGO_TRACE"
echo 'fixture dependency source unavailable' >&2
exit 101
CARGO
chmod +x "$fixture/bin/cargo"
export PATH="$fixture/bin:$PATH" CARGO_TRACE="$fixture/cargo.trace"
check_fails() {
    local status=0
    bash "$root/scripts/ci/publication-preflight.sh" > "$fixture/preflight.log" 2>&1 || status=$?
    [[ $status != 0 ]]
    grep -F "$1" "$fixture/preflight.log"
}
check_fails "Missing or invalid prepared simulator at $IC_TESTKIT_SERVER_DIRECTORY"
touch "$fixture/file"
ln -s "$fixture/file" "$IC_TESTKIT_SERVER_DIRECTORY"
check_fails "Missing or invalid prepared simulator at $IC_TESTKIT_SERVER_DIRECTORY"
[[ ! -e "$CARGO_TRACE" ]]
mkdir "$fixture/empty-cache-test"
export IC_TESTKIT_SERVER_DIRECTORY="$fixture/empty-cache-test"
cp /usr/bin/true "$IC_TESTKIT_SERVER_DIRECTORY/pocket-ic"
check_fails 'fixture dependency source unavailable'
[[ $(cat "$CARGO_TRACE") == "${CARGO_NET_OFFLINE-unset} fetch --locked --manifest-path Cargo.toml" ]]
grep -F 'Fetching locked dependencies for Cargo.toml' "$fixture/preflight.log"
if grep -Fq 'Fetching locked dependencies for qualification/Cargo.toml' "$fixture/preflight.log"; then exit 1; fi
status=0
perl "$root/scripts/dev/blob-inputs.pl" > "$fixture/blob.json" 2> "$fixture/blob.log" || status=$?
[[ $status == 101 && ! -s "$fixture/blob.json" ]]
grep -F 'cargo metadata failed for qualification/Cargo.toml (exit 101)' "$fixture/blob.log"
