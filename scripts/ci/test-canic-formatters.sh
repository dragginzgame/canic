#!/usr/bin/env bash
set -euo pipefail
unset MAKEFLAGS MFLAGS MAKEOVERRIDES GNUMAKEFLAGS MAKEFILES
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-formatters.XXXXXX")"
trap 'if [[ $? == 0 ]]; then rm -rf -- "$fixture"; else printf "Formatter fixture retained: %s\n" "$fixture" >&2; fi' EXIT
mkdir -p "$fixture/make" "$fixture/ci" "$fixture/scripts/ci" "$fixture/.tools/rust/bin"
cp "$ROOT/Makefile" "$ROOT/tool-versions.env" "$fixture/"
cp "$ROOT/make/tools.mk" "$ROOT/make/execution.mk" "$ROOT/make/release.mk" "$fixture/make/"
cp "$ROOT/ci/tool-versions.env" "$ROOT/ci/ic-tools.tsv" "$fixture/ci/"
cp "$ROOT/scripts/ci/check-make-execution.sh" "$ROOT/scripts/ci/check-format-tools.sh" \
    "$ROOT/scripts/ci/run-formatting.sh" "$ROOT/scripts/ci/run-canic-formatters.sh" \
    "$ROOT/scripts/ci/ic-tool-pins.sh" "$fixture/scripts/ci/"
mkdir -p "$fixture/integrations/blob-service/consumer" "$fixture/integrations/blob-service/embedded-consumer"
cat > "$fixture/.tools/rust/bin/cargo" <<'CARGO'
#!/usr/bin/env bash
set -euo pipefail
[[ "$CARGO_NET_OFFLINE" == true && "$RUSTUP_AUTO_INSTALL" == 0 ]]
case "$*" in
    'sort --version') echo 'cargo-sort 2.1.4' ;;
    'fmt --version') echo rustfmt ;;
    *) printf '%s\t%s\n' "$PWD" "$*" >> "$CANIC_FORMAT_EVENTS"
       [[ "$*" != "${CANIC_FORMAT_FAIL:-}" ]] || exit 23 ;;
esac
CARGO
chmod +x "$fixture/.tools/rust/bin/cargo"
export CANIC_FORMAT_EVENTS="$fixture/events" RUNNER_TEMP="$fixture"
for target in fmt fmt-check; do
    : > "$CANIC_FORMAT_EVENTS"
    make --no-print-directory -C "$fixture" "$target" > "$fixture/output" 2>&1
    [[ "$(wc -l < "$fixture/output")" == 1 ]]
    for workspace in "$fixture" \
        "$fixture/integrations/blob-service/consumer" "$fixture/integrations/blob-service/embedded-consumer"; do
        awk -F '\t' -v root="$workspace" '$1 == root && $2 ~ /^fmt --all/ { found = 1 } END { exit !found }' "$CANIC_FORMAT_EVENTS"
    done
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
echo 'Canic formatter roots, check-only mode, failure logs and Make admission passed'
