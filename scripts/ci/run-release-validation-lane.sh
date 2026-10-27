#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VALIDATION_KIND="${1:-}"
BUMP_TYPE="${2:-}"

case "$VALIDATION_KIND/$BUMP_TYPE" in
    complete/ | complete/patch | complete/minor | complete/major | fast/patch) ;;
    *)
        echo "usage: $0 complete [patch|minor|major] | fast patch" >&2
        exit 2
        ;;
esac

cd "$ROOT"
make --no-print-directory ensure-clean
if [[ -n "$BUMP_TYPE" ]]; then
    bash scripts/ci/check-release-draft-ready.sh "$BUMP_TYPE" --check-remote
fi
validated_head="$(git rev-parse HEAD)"
if [[ -n "${RELEASE_SOURCE:-}" && "$RELEASE_SOURCE" != "$validated_head" ]]; then
    echo 'release qualification source differs from the delivery source' >&2
    exit 1
fi
# Qualification is fresh: a source-only receipt cannot bind effective environment,
# compiler overrides or the current security database. Exact-attempt recovery is
# owned by the release runner after successful qualification and preparation.
make --no-print-directory check-build-environment
make --no-print-directory tools-check
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

if [[ -n "$BUMP_TYPE" ]]; then
    CANIC_RELEASE_VALIDATED=1 \
    CANIC_RELEASE_VALIDATED_HEAD="$validated_head" \
    CANIC_RELEASE_VALIDATION_KIND="$VALIDATION_KIND" \
        scripts/ci/bump-version.sh "$BUMP_TYPE"
fi
