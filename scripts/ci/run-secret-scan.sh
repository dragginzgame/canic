#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=/dev/null
source "$ROOT_DIR/tool-versions.env"

GITLEAKS_BIN="${GITLEAKS_BIN:-gitleaks}"
TMP_DIR=""

if [ "$#" -ne 0 ]; then
    echo "usage: run-secret-scan.sh" >&2
    exit 1
fi

fail() {
    echo "secret scan failed: $1" >&2
    exit 1
}

if [ -n "${GITLEAKS_CONFIG:-}" ] || [ -n "${GITLEAKS_CONFIG_TOML:-}" ]; then
    fail "Gitleaks configuration overrides are not allowed; built-in rules are required"
fi
[ ! -e "$ROOT_DIR/.gitleaks.toml" ] ||
    fail "repository .gitleaks.toml overrides are not allowed; built-in rules are required"

case "$GITLEAKS_BIN" in
*/*)
    [ -x "$GITLEAKS_BIN" ] || fail "gitleaks binary is not executable: $GITLEAKS_BIN"
    ;;
*)
    command -v "$GITLEAKS_BIN" >/dev/null 2>&1 ||
        fail "gitleaks is unavailable; run make install-dev"
    ;;
esac

if ! version_output="$("$GITLEAKS_BIN" version 2>&1)"; then
    fail "unable to read the gitleaks version"
fi
if [ "$version_output" != "$CANIC_GITLEAKS_VERSION" ]; then
    fail "gitleaks version mismatch; expected $CANIC_GITLEAKS_VERSION, got $version_output"
fi

git -C "$ROOT_DIR" rev-parse --is-inside-work-tree >/dev/null 2>&1 ||
    fail "repository history is unavailable"
if ! shallow_repository="$(git -C "$ROOT_DIR" rev-parse --is-shallow-repository 2>/dev/null)"; then
    fail "unable to determine whether repository history is complete"
fi
[ "$shallow_repository" = "false" ] ||
    fail "complete repository history is unavailable in a shallow clone"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
report="$TMP_DIR/gitleaks.json"

if ! "$GITLEAKS_BIN" git \
    --redact=100 \
    --no-banner \
    --no-color \
    --log-level error \
    --gitleaks-ignore-path "$ROOT_DIR/.gitleaksignore" \
    --report-format json \
    --report-path "$report" \
    "$ROOT_DIR"; then
    finding_count="$(rg -c '"RuleID"' "$report" 2>/dev/null || true)"
    if [[ "$finding_count" =~ ^[1-9][0-9]*$ ]]; then
        fail "gitleaks found $finding_count candidate leak(s); findings were redacted and not retained"
    fi
    fail "gitleaks did not complete and produced no redacted findings"
fi

source_commit="$(git -C "$ROOT_DIR" rev-parse HEAD)"
printf 'secret scan passed: gitleaks %s; mode=full-history; rules=builtin; commit=%s; findings=0\n' \
    "$CANIC_GITLEAKS_VERSION" "$source_commit"
