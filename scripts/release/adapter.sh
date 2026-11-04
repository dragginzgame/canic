#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd -P)"
cd "$root"
bash scripts/release/selection.sh
case "${1:-}" in
    preflight)
        [[ "$(bash scripts/ci/read-workspace-version.sh)" == "${RELEASE_PREVIOUS:?}" ]]
        bash scripts/ci/check-release-source.sh
        bash scripts/ci/check-release-draft-ready.sh "${RELEASE_KIND:?}"
        make --no-print-directory prepare-validation SCOPE=release
        ;;
    consumer-locks)
        printf '%s\n' integrations/blob-service/consumer/Cargo.lock \
            integrations/blob-service/embedded-consumer/Cargo.lock
        ;;
    files) bash scripts/release/metadata.sh files ;;
    committed) bash scripts/ci/check-release-candidate.sh --commit "${RELEASE_COMMIT:?}" ;;
    tagged)
        bash scripts/ci/check-release-candidate.sh --commit "${RELEASE_COMMIT:?}"
        bash scripts/ci/check-release-tag.sh "$RELEASE_COMMIT" "${RELEASE_VERSION:?}" "$RELEASE_GROUP"
        ;;
    *) exit 2 ;;
esac
