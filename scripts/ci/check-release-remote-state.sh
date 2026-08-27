#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

fail() {
    echo "release remote-state check failed: $1" >&2
    exit 1
}

MODE="${1:-}"
VERSION="${2:-}"
case "$MODE" in
    before-version | before-push) ;;
    *) fail "expected before-version or before-push mode" ;;
esac
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] ||
    fail "expected an exact stable semantic version"

BRANCH="$(git symbolic-ref --quiet --short HEAD)" || fail "HEAD is detached"
REMOTE=origin
REMOTE_BRANCH="refs/remotes/$REMOTE/$BRANCH"
TAG="v$VERSION"

git fetch --quiet --no-tags "$REMOTE" \
    "+refs/heads/$BRANCH:$REMOTE_BRANCH" ||
    fail "could not refresh $REMOTE/$BRANCH"

REMOTE_HEAD="$(git rev-parse --verify "$REMOTE_BRANCH")" ||
    fail "could not resolve refreshed $REMOTE/$BRANCH"
LOCAL_HEAD="$(git rev-parse HEAD)" || fail "could not resolve local HEAD"
git merge-base --is-ancestor "$REMOTE_HEAD" "$LOCAL_HEAD" ||
    fail "$REMOTE/$BRANCH at $REMOTE_HEAD is not an ancestor of local HEAD $LOCAL_HEAD"

REMOTE_TAG_OBJECT=""
if REMOTE_TAG_LINE="$(git ls-remote --exit-code --refs "$REMOTE" "refs/tags/$TAG")"; then
    read -r REMOTE_TAG_OBJECT REMOTE_TAG_REF <<<"$REMOTE_TAG_LINE"
    [[ "$REMOTE_TAG_REF" == "refs/tags/$TAG" ]] ||
        fail "remote returned an unexpected ref while checking $TAG"
else
    status=$?
    [[ $status -eq 2 ]] || fail "could not inspect remote tag $TAG"
fi

if [[ "$MODE" == "before-version" ]]; then
    [[ -z "$REMOTE_TAG_OBJECT" ]] ||
        fail "$TAG already exists on $REMOTE at $REMOTE_TAG_OBJECT"
else
    LOCAL_TAG_OBJECT="$(git rev-parse --verify "refs/tags/$TAG")" ||
        fail "local release tag $TAG is missing"
    if [[ -n "$REMOTE_TAG_OBJECT" && "$REMOTE_TAG_OBJECT" != "$LOCAL_TAG_OBJECT" ]]; then
        fail "$TAG conflicts with remote object $REMOTE_TAG_OBJECT"
    fi
fi

echo "✅ Release remote state is ready: $REMOTE/$BRANCH at $REMOTE_HEAD; $TAG checked for $MODE"
