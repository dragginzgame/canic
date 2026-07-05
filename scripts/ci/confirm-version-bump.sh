#!/usr/bin/env bash
set -euo pipefail

BUMP_TYPE="${1:?usage: confirm-version-bump.sh <minor|major>}"

case "$BUMP_TYPE" in
    minor | major) ;;
    *)
        echo "unsupported guarded bump type: $BUMP_TYPE" >&2
        exit 2
        ;;
esac

CURRENT_VERSION="$(cargo get workspace.package.version)"

cat >&2 <<MSG
This will run the $BUMP_TYPE release gate and bump Canic from $CURRENT_VERSION.
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
