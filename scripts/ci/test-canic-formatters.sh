#!/usr/bin/env bash
set -euo pipefail
unset MAKEFLAGS MFLAGS MAKEOVERRIDES GNUMAKEFLAGS MAKEFILES
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-formatters.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf -- "$fixture"; else printf "Formatter fixture retained: %s\n" "$fixture" >&2; fi' EXIT
mkdir -p "$fixture/make" "$fixture/ci" "$fixture/scripts/ci" "$fixture/.tools/rust/common/bin"
cp "$ROOT/Makefile" "$ROOT/tool-versions.env" "$fixture/"
cp "$ROOT/make/tools.mk" "$ROOT/make/execution.mk" "$ROOT/make/release.mk" "$ROOT/make/consumer.mk" "$ROOT/make/rust-format.mk" "$fixture/make/"
cp "$ROOT/ci/tool-versions.env" "$ROOT/ci/ic-tools.tsv" "$fixture/ci/"
bash "$ROOT/scripts/ci/prepare-script-fixture.sh" "$ROOT" "$fixture" \
    scripts/ci/check-make-execution.sh scripts/ci/check-format-tools.sh \
    scripts/ci/run-rust-formatting.pl scripts/ci/ic-tool-pins.sh \
    scripts/ci/tool-catalog-records.pl
# Observe every actual formatter invocation in this routing fixture.
printf '\nFORMAT_RUST_ARGS += --no-cache\n' >> "$fixture/Makefile"
mkdir -p "$fixture/integrations/blob-service/consumer" "$fixture/integrations/blob-service/embedded-consumer"
cat > "$fixture/.tools/rust/common/bin/cargo" <<'CARGO'
#!/usr/bin/env bash
set -euo pipefail
[[ "$CARGO_NET_OFFLINE" == true && "$RUSTUP_AUTO_INSTALL" == 0 ]]
case "$*" in
    'sort --version') echo 'cargo-sort 2.1.4' ;;
    'fmt --version') echo rustfmt ;;
    'sort-derives --version') echo 'cargo-sort-derives 0.14.1' ;;
    *) printf '%s\t%s\n' "$PWD" "$*" >> "$CANIC_FORMAT_EVENTS"
       [[ "$*" != "${CANIC_FORMAT_FAIL:-}" ]] || exit 23 ;;
esac
CARGO
chmod +x "$fixture/.tools/rust/common/bin/cargo"
export CANIC_FORMAT_EVENTS="$fixture/events" RUNNER_TEMP="$fixture"
for target in fmt fmt-check; do
    : > "$CANIC_FORMAT_EVENTS"
    make --no-print-directory -C "$fixture" "$target" > "$fixture/output" 2>&1
    for workspace in "$fixture" \
        "$fixture/integrations/blob-service/consumer" "$fixture/integrations/blob-service/embedded-consumer"; do
        awk -F '\t' -v root="$workspace" '$1 == root && $2 ~ /^fmt --all/ { found = 1 } END { exit !found }' "$CANIC_FORMAT_EVENTS"
    done
    awk -F '\t' '$2 ~ /^sort --workspace/ && $2 !~ /--grouped/ { exit 1 }' "$CANIC_FORMAT_EVENTS"
    if [[ "$target" == fmt-check ]]; then
        awk -F '\t' '$2 !~ /--check/ { exit 1 }' "$CANIC_FORMAT_EVENTS"
    else
        if grep -q -- --check "$CANIC_FORMAT_EVENTS"; then exit 1; fi
    fi
done
: > "$CANIC_FORMAT_EVENTS"
if CANIC_FORMAT_FAIL='sort-derives --check' make --no-print-directory -C "$fixture" fmt-check > "$fixture/output" 2>&1; then exit 1; fi
[[ -n "$(find "$fixture" -maxdepth 1 -name 'formatting.*' -print)" ]]
if grep -q 'fmt --all' "$CANIC_FORMAT_EVENTS"; then exit 1; fi
for mode in -n -i -q -t; do
    : > "$CANIC_FORMAT_EVENTS"
    if make --no-print-directory -C "$fixture" "$mode" fmt MAKEFLAGS= > "$fixture/output" 2>&1; then exit 1; fi
    [[ ! -s "$CANIC_FORMAT_EVENTS" ]]
done
# Exercise actual Cargo sort: grouped writes are stable and checks reject
# an unsorted dependency pair without mutating the inspected manifest.
mkdir -p "$fixture/grouped"
cat > "$fixture/grouped/Cargo.toml" <<'TOML'
[workspace]
members = []

[workspace.dependencies]
# Production dependencies
alpha = "1"
beta = "1"

# Tests and tooling only
aardvark = "1"
TOML
cargo sort --workspace --grouped "$fixture/grouped" > "$fixture/grouped.log" 2>&1
cp "$fixture/grouped/Cargo.toml" "$fixture/grouped-first"
cargo sort --workspace --grouped "$fixture/grouped" >> "$fixture/grouped.log" 2>&1
cmp "$fixture/grouped-first" "$fixture/grouped/Cargo.toml"
grep -Fxq '# Production dependencies' "$fixture/grouped/Cargo.toml"
grep -Fxq '# Tests and tooling only' "$fixture/grouped/Cargo.toml"
perl "$ROOT/scripts/ci/prepare-cargo-formatting-fixture.pl" \
    "$fixture/grouped/Cargo.toml" workspace.dependencies alpha beta > "$fixture/unsorted"
cp "$fixture/unsorted" "$fixture/grouped/Cargo.toml"
if cargo sort --workspace --grouped --check "$fixture/grouped" >> "$fixture/grouped.log" 2>&1; then exit 1; fi
cmp "$fixture/unsorted" "$fixture/grouped/Cargo.toml"
echo 'Canic formatter roots, check-only mode, failure logs and Make admission passed'
