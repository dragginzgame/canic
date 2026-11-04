#!/usr/bin/env bash
set -euo pipefail
# This explicit exception applies only to the maintainer-selected 0.115.0 batch.
# Read authority from the immutable source being rendered, never ambient prose.
source="${1:?source commit}" version="${2:?release version}"
[[ "$source" =~ ^[0-9a-f]{40}$ && "$version" == 0.115.0 ]]
git show "$source:ci/release-exception-0.115.json" |
    jq -e --arg group "${RELEASE_GROUP:?}" --arg version "$version" '
        .schema == 1 and .version == $version and .gate == "maintainer-waived"
        and (.groups | index($group) != null)
    ' > /dev/null
printf 'Full release gate explicitly waived for %s %s; focused evidence only.\n' "$RELEASE_GROUP" "$version"
