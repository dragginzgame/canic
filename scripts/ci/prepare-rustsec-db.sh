#!/usr/bin/env bash
set -euo pipefail

# Prepare one isolated committed database. Never run an audit or choose its policy.
if [[ $# != 3 ]]; then
    echo 'usage: prepare-rustsec-db.sh <online|local> <source> <new-destination>' >&2
    exit 2
fi
mode="$1"
source="$2"
destination="$3"
case "$source$destination" in
    *$'\n'*|*$'\r'*|*$'\t'*) echo 'source/destination must not contain control separators' >&2; exit 2 ;;
esac
[[ -n "$source" && -n "$destination" ]] || { echo 'source and destination are required' >&2; exit 2; }
export GIT_TERMINAL_PROMPT=0
# All Git operations belong to the selected source or new database, even when
# invoked from a hook or another repository's validation process.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR
unset GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_NAMESPACE
case "$mode" in
    online)
        # No embedded credentials, query tokens, fragments or alternate transports.
        [[ "$source" =~ ^https://[A-Za-z0-9.-]+(:[0-9]+)?/[^@?#[:space:]]+$ ]] || {
            echo 'online source must be an explicit HTTPS repository URL without credentials/query/fragment' >&2
            exit 2
        }
        export GIT_ALLOW_PROTOCOL=https ;;
    local)
        export GIT_ALLOW_PROTOCOL=file
        source="$(cd "$source" && pwd -P)" ;;
    *) echo 'select online or local preparation explicitly' >&2; exit 2 ;;
esac
[[ "$destination" == /* ]] || destination="$PWD/$destination"
# mkdir claims the path exclusively. Never reuse, remove or overwrite a caller path.
mkdir "$destination"
finish() {
    local status=$?
    if [[ "$status" != 0 ]]; then
        echo "RustSec preparation failed; evidence retained: $destination" >&2
    fi
}
trap finish EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
log="$destination/prepare.log"
(
    # Keep the outer failure diagnostic outside this block's log redirection.
    trap - EXIT
    printf 'mode: %s\nsource: %s\n' "$mode" "$source"
    if [[ "$mode" == local ]]; then
        # Observe before copying; dirty/untracked files are never the selected source.
        revision="$(git -C "$source" rev-parse --verify 'HEAD^{commit}')"
        [[ "$revision" =~ ^([0-9a-f]{40}|[0-9a-f]{64})$ ]] || exit 1
        git clone --local --no-hardlinks --dissociate --no-checkout --no-recurse-submodules \
            -- "$source" "$destination/db"
        git -C "$destination/db" checkout --detach "$revision" --
    else
        git -c http.lowSpeedLimit=1024 -c http.lowSpeedTime=30 \
            clone --depth 1 --single-branch --no-tags --no-recurse-submodules \
            -- "$source" "$destination/db"
        revision="$(git -C "$destination/db" rev-parse --verify 'HEAD^{commit}')"
        [[ "$revision" =~ ^([0-9a-f]{40}|[0-9a-f]{64})$ ]] || exit 1
    fi
    selected_revision="$(git -C "$destination/db" rev-parse --verify 'HEAD^{commit}')" || exit 1
    [[ "$selected_revision" == "$revision" ]] || exit 1
    # An isolated clone must not borrow source objects through an alternates file.
    [[ ! -s "$destination/db/.git/objects/info/alternates" ]] || exit 1
    printf 'revision: %s\n' "$revision"
    printf '%s\n' "$revision" > "$destination/revision"
) > "$log" 2>&1
cat "$destination/revision"
