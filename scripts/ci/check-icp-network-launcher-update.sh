#!/usr/bin/env bash
set -euo pipefail

PINNED_TAG="${ICP_CLI_NETWORK_LAUNCHER_VERSION:-v13.0.0-2026-05-07-04-27}"
FIX_COMMIT="${ICP_CLI_NETWORK_LAUNCHER_FIX_COMMIT:-17524c56ef237c54b510a201ceb58da8d790d4d5}"
REPOSITORY="${ICP_CLI_NETWORK_LAUNCHER_REPOSITORY:-dfinity/icp-cli-network-launcher}"
API_URL="${GITHUB_API_URL:-https://api.github.com}"
TAGS_URL="${ICP_CLI_NETWORK_LAUNCHER_TAGS_URL:-$API_URL/repos/$REPOSITORY/tags?per_page=1}"

if [ -n "${ICP_CLI_NETWORK_LAUNCHER_TAGS_JSON:-}" ]; then
    tags_json="$ICP_CLI_NETWORK_LAUNCHER_TAGS_JSON"
else
    tags_json="$(curl -fsSL \
        -H "Accept: application/vnd.github+json" \
        "$TAGS_URL")"
fi

latest_tag="$(
    printf '%s\n' "$tags_json" \
        | sed -n 's/.*"name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
        | head -n 1
)"

if [ -z "$latest_tag" ]; then
    echo "Could not read latest $REPOSITORY tag from $TAGS_URL." >&2
    exit 1
fi

if [ "$latest_tag" = "$PINNED_TAG" ]; then
    echo "ICP network launcher remains pinned at latest observed tag $PINNED_TAG."
    exit 0
fi

cat >&2 <<EOF
::error title=ICP network launcher update candidate::Latest $REPOSITORY tag is $latest_tag; Canic pins $PINNED_TAG while waiting for upstream dfinity/ic commit $FIX_COMMIT to reach a launcher release.
Test local II/NNS delegation with $latest_tag. If the upstream fix is present, update icp.yaml.
EOF
exit 1
