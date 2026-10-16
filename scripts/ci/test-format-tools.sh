#!/usr/bin/env bash
# Shared companions: scripts/ci/check-format-tools.sh scripts/ci/fixture-lifecycle.sh
set -euo pipefail
ROOT="$0"
[[ "$ROOT" == /* ]] || ROOT="$PWD/$ROOT"
ROOT="$(cd -P "${ROOT%/*}/../.." && printf '%s/.' "$PWD")"
ROOT="${ROOT%/.}"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/format-tools-test.XXXXXX")"
fixture="$(CDPATH="" cd -P -- "$fixture" && printf '%s/.' "$PWD")"
fixture="${fixture%/.}"
# shellcheck source=scripts/ci/fixture-lifecycle.sh
source "$ROOT/scripts/ci/fixture-lifecycle.sh"
fixture_complete=false
trap 'fixture_finish "$?" "$fixture" "Formatter fixtures retained"' EXIT
export FORMAT_TOOLS_FIXTURE="$fixture"
mkdir "$fixture/bin"
cat > "$fixture/bin/cargo" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
[[ "$CARGO_NET_OFFLINE" == true && "$RUSTUP_AUTO_INSTALL" == 0 ]] || exit 1
[[ "${RUSTUP_TOOLCHAIN:-}" == consumer-toolchain ]] || exit 1
printf '%s\n' "$*" >> "$FORMAT_TOOLS_FIXTURE/calls"
case "$*" in
    'sort --version') echo "cargo-sort ${TEST_SORT_VERSION:-2.1.4}"; exit "${TEST_SORT_STATUS:-0}" ;;
    'fmt --version') echo rustfmt; exit "${TEST_FMT_STATUS:-0}" ;;
    *) exit 99 ;;
esac
SCRIPT
chmod +x "$fixture/bin/cargo"
export PATH="$fixture/bin:$PATH" RUSTUP_TOOLCHAIN=consumer-toolchain
check() { bash "$ROOT/scripts/ci/check-format-tools.sh" "$@"; }
refuse() {
    : > "$fixture/calls"
    if check "$@" > "$fixture/refusal.log" 2>&1; then
        echo 'formatter prerequisite check unexpectedly succeeded' >&2; exit 1
    fi
}
check 2.1.4
printf 'sort --version\nfmt --version\n' > "$fixture/expected"
cmp "$fixture/expected" "$fixture/calls"
# A selected executable can contain spaces; no splitting into command arguments.
cp "$fixture/bin/cargo" "$fixture/selected cargo"
: > "$fixture/calls"
check 2.1.4 "$fixture/selected cargo"
cmp "$fixture/expected" "$fixture/calls"
TEST_SORT_VERSION=2.1.40 refuse 2.1.4
[[ "$(cat "$fixture/calls")" == 'sort --version' ]] || exit 1
# Matching stdout with a failed status must not admit a broken tool.
TEST_SORT_STATUS=9 refuse 2.1.4
[[ "$(cat "$fixture/calls")" == 'sort --version' ]] || exit 1
TEST_FMT_STATUS=9 refuse 2.1.4
cmp "$fixture/expected" "$fixture/calls"
refuse 2.1.4 "$fixture/missing cargo"
[[ ! -s "$fixture/calls" ]] || exit 1
refuse latest
[[ ! -s "$fixture/calls" ]] || exit 1
refuse
[[ ! -s "$fixture/calls" ]] || exit 1
echo 'Formatter prerequisite checks passed'
fixture_complete=true
