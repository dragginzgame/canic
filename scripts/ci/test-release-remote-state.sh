#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CHECK="$ROOT/scripts/ci/check-release-remote-state.sh"
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/canic-release-remote-state.XXXXXX")"
trap 'rm -rf "$FIXTURE"' EXIT

fail() {
    echo "release remote-state test failed: $1" >&2
    exit 1
}

REMOTE="$FIXTURE/origin.git"
LOCAL="$FIXTURE/local"
OTHER="$FIXTURE/other"
git init --quiet --bare "$REMOTE"
git init --quiet --initial-branch=main "$LOCAL"
git -C "$LOCAL" config user.email release-test@example.invalid
git -C "$LOCAL" config user.name "Release Test"
printf 'base\n' >"$LOCAL/state"
git -C "$LOCAL" add state
git -C "$LOCAL" commit --quiet -m base
git -C "$LOCAL" remote add origin "$REMOTE"
git -C "$LOCAL" push --quiet --set-upstream origin main
git -C "$REMOTE" symbolic-ref HEAD refs/heads/main

(
    cd "$LOCAL"
    bash "$CHECK" before-version 1.2.4 >/dev/null
) || fail "accepted fast-forward source was rejected"

git clone --quiet "$REMOTE" "$OTHER"
git -C "$OTHER" config user.email release-test@example.invalid
git -C "$OTHER" config user.name "Other Release Test"
printf 'remote advance\n' >>"$OTHER/state"
git -C "$OTHER" commit --quiet -am "remote advance"
git -C "$OTHER" push --quiet origin main
if (
    cd "$LOCAL"
    bash "$CHECK" before-version 1.2.4 >/dev/null 2>&1
); then
    fail "diverged source was accepted before version mutation"
fi

git -C "$LOCAL" merge --quiet --ff-only origin/main
git -C "$OTHER" tag -a v1.2.4 -m "occupied release tag"
git -C "$OTHER" push --quiet origin refs/tags/v1.2.4
if (
    cd "$LOCAL"
    bash "$CHECK" before-version 1.2.4 >/dev/null 2>&1
); then
    fail "occupied remote tag was accepted before version mutation"
fi

git -C "$LOCAL" tag -a v1.2.5 -m "Release 1.2.5"
(
    cd "$LOCAL"
    bash "$CHECK" before-push 1.2.5 >/dev/null
) || fail "absent remote release tag was rejected before push"
git -C "$LOCAL" push --quiet origin refs/tags/v1.2.5
(
    cd "$LOCAL"
    bash "$CHECK" before-push 1.2.5 >/dev/null
) || fail "matching remote release tag was rejected on push retry"

git -C "$LOCAL" tag -a v1.2.6 -m "local tag object"
git -C "$OTHER" tag -a v1.2.6 -m "conflicting remote tag object"
git -C "$OTHER" push --quiet origin refs/tags/v1.2.6
if (
    cd "$LOCAL"
    bash "$CHECK" before-push 1.2.6 >/dev/null 2>&1
); then
    fail "conflicting remote release tag object was accepted"
fi

echo "release remote-state test passed"
