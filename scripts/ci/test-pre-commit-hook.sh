#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HOOK="$ROOT/.githooks/pre-commit"
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/canic-pre-commit-hook.XXXXXX")"
CASE_NUMBER=0
trap 'rm -rf "$FIXTURE"' EXIT

fail() {
    echo "pre-commit hook test failed: $1" >&2
    exit 1
}

new_case() {
    CASE_NUMBER=$((CASE_NUMBER + 1))
    CASE_ROOT="$FIXTURE/case-$CASE_NUMBER"
    mkdir -p "$CASE_ROOT/.githooks" "$CASE_ROOT/bin"
    git -C "$CASE_ROOT" init --quiet --initial-branch=main
    git -C "$CASE_ROOT" config user.email pre-commit-test@example.invalid
    git -C "$CASE_ROOT" config user.name "Pre-commit Test"
    cp "$HOOK" "$CASE_ROOT/.githooks/pre-commit"
    chmod +x "$CASE_ROOT/.githooks/pre-commit"
    printf 'base\n' >"$CASE_ROOT/target.rs"
    printf 'base\n' >"$CASE_ROOT/unrelated.txt"
    git -C "$CASE_ROOT" add target.rs unrelated.txt
    git -C "$CASE_ROOT" commit --quiet -m base
    printf '%s\n' \
        '#!/usr/bin/env bash' \
        'set -euo pipefail' \
        '[[ "$*" = "fmt" ]] || exit 97' \
        '[[ -z "${FAKE_FORMAT_TARGET:-}" ]] || printf "formatted\n" >"$FAKE_FORMAT_TARGET"' \
        >"$CASE_ROOT/bin/make"
    chmod +x "$CASE_ROOT/bin/make"
}

run_hook() {
    (
        cd "$CASE_ROOT"
        PATH="$CASE_ROOT/bin:$PATH" bash .githooks/pre-commit
    )
}

new_case
printf 'staged\n' >"$CASE_ROOT/target.rs"
git -C "$CASE_ROOT" add target.rs
printf 'unstaged\n' >"$CASE_ROOT/target.rs"
if run_hook >/dev/null 2>&1; then
    fail "partially staged input was accepted"
fi
[[ "$(git -C "$CASE_ROOT" show :target.rs)" = "staged" ]] ||
    fail "partial-stage rejection changed the index"
[[ "$(<"$CASE_ROOT/target.rs")" = "unstaged" ]] ||
    fail "partial-stage rejection changed the working copy"

new_case
printf 'unformatted\n' >"$CASE_ROOT/target.rs"
git -C "$CASE_ROOT" add target.rs
FAKE_FORMAT_TARGET="$CASE_ROOT/target.rs" run_hook >/dev/null
[[ "$(git -C "$CASE_ROOT" show :target.rs)" = "formatted" ]] ||
    fail "formatted staged content was not refreshed"
[[ "$(<"$CASE_ROOT/target.rs")" = "formatted" ]] ||
    fail "formatter output was not retained in the working copy"

new_case
printf 'unformatted\n' >"$CASE_ROOT/target.rs"
git -C "$CASE_ROOT" add target.rs
printf 'local-only\n' >"$CASE_ROOT/unrelated.txt"
FAKE_FORMAT_TARGET="$CASE_ROOT/target.rs" run_hook >/dev/null
[[ "$(git -C "$CASE_ROOT" show :target.rs)" = "formatted" ]] ||
    fail "formatted staged content was not refreshed with unrelated local edits"
[[ "$(git -C "$CASE_ROOT" show :unrelated.txt)" = "base" ]] ||
    fail "unrelated local edits entered the index"
[[ "$(<"$CASE_ROOT/unrelated.txt")" = "local-only" ]] ||
    fail "unrelated local edits were not preserved"

echo "pre-commit hook tests passed"
