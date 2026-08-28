#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VALIDATION_KIND="${1:-}"
BUMP_TYPE="${2:-}"

case "$VALIDATION_KIND/$BUMP_TYPE" in
    complete/patch | complete/minor | complete/major | fast/patch) ;;
    *)
        echo "usage: $0 <complete|fast> <patch|minor|major>" >&2
        exit 2
        ;;
esac

cd "$ROOT"
make --no-print-directory ensure-clean
validated_head="$(git rev-parse HEAD)"

case "$VALIDATION_KIND" in
    complete)
        make --no-print-directory validate
        ;;
    fast)
        bash scripts/ci/check-fast-patch-eligibility.sh
        ;;
esac

make --no-print-directory ensure-clean
current_head="$(git rev-parse HEAD)"
if [[ "$validated_head" != "$current_head" ]]; then
    echo "validated source changed during the release gate" >&2
    exit 1
fi

CANIC_RELEASE_VALIDATED=1 \
CANIC_RELEASE_VALIDATED_HEAD="$validated_head" \
CANIC_RELEASE_VALIDATION_KIND="$VALIDATION_KIND" \
    scripts/ci/bump-version.sh "$BUMP_TYPE"
