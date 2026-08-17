#!/usr/bin/env bash
set -euo pipefail

BUMP_TYPE="${1:?usage: confirm-version-bump.sh <minor|major>}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION_READER="$ROOT/scripts/ci/read-workspace-version.sh"

case "$BUMP_TYPE" in
    minor | major) ;;
    *)
        echo "unsupported guarded bump type: $BUMP_TYPE" >&2
        exit 2
        ;;
esac

CURRENT_VERSION="$(bash "$VERSION_READER")"

cat >&2 <<MSG
This will run make validate and bump Canic from $CURRENT_VERSION ($BUMP_TYPE).
Type '$BUMP_TYPE' to continue:
MSG

if ! read -r confirmation; then
    echo "Aborted $BUMP_TYPE version bump." >&2
    exit 1
fi

if [ "$confirmation" != "$BUMP_TYPE" ]; then
    echo "Aborted $BUMP_TYPE version bump." >&2
    exit 1
fi
