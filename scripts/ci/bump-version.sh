#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
cd "$root"
bash scripts/release/selection.sh
kind="${1:-patch}"
[[ "${CANIC_RELEASE_VALIDATED:-}" == 1 && "${CANIC_RELEASE_VALIDATED_HEAD:-}" == "$(git rev-parse HEAD)" ]] || {
    echo 'Refusing to bump without fresh governed validation of this source' >&2; exit 1;
}
export CANIC_RELEASE_VALIDATION_KIND="${CANIC_RELEASE_VALIDATION_KIND:-complete}"
case "$CANIC_RELEASE_VALIDATION_KIND/$kind" in complete/patch|complete/minor|complete/major|plan/patch|plan/minor|plan/major|fast/patch|maintainer-waived/minor) ;; *) exit 2 ;; esac
if [[ "$CANIC_RELEASE_VALIDATION_KIND" == plan ]]; then
    perl scripts/release/validation-evidence.pl verify \
        "target/release-validation/$RELEASE_GROUP/$CANIC_RELEASE_VALIDATED_HEAD.json"
fi
if [[ -z "${RELEASE_VERSION:-}" ]]; then
    make --no-print-directory ensure-clean
fi
export RELEASE_PREVIOUS RELEASE_VERSION RELEASE_DATE
RELEASE_PREVIOUS="$(bash scripts/ci/read-workspace-version.sh)"
planned="$(bash scripts/ci/next-release-version.sh "$RELEASE_PREVIOUS" "$kind")"
[[ -z "${RELEASE_VERSION:-}" || "$RELEASE_VERSION" == "$planned" ]] || exit 1
if [[ -z "${RELEASE_VERSION:-}" ]]; then
    bash scripts/ci/check-release-remote-state.sh before-version "$planned"
fi
RELEASE_VERSION="$planned"
if [[ "$CANIC_RELEASE_VALIDATION_KIND" == maintainer-waived ]]; then
    bash scripts/release/check-maintainer-exception.sh "$CANIC_RELEASE_VALIDATED_HEAD" "$RELEASE_VERSION"
fi
RELEASE_DATE="${CANIC_RELEASE_DATE:-${RELEASE_DATE:-$(date -u +%F)}}"
bash scripts/ci/check-release-draft-ready.sh "$kind"
tag_status=0
git show-ref --verify --quiet "refs/tags/$RELEASE_GROUP-v$RELEASE_VERSION" || tag_status=$?
[[ "$tag_status" == 1 ]] || { echo 'Selected release tag is occupied or unreadable' >&2; exit 1; }
bash scripts/release/metadata.sh prepare
printf 'Prepared %s %s -> %s; publication remains separate\n' "$RELEASE_GROUP" "$RELEASE_PREVIOUS" "$RELEASE_VERSION"
