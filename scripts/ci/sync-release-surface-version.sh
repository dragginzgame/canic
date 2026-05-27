#!/usr/bin/env bash
set -euo pipefail

NEW_VERSION="${1:?usage: sync-release-surface-version.sh <x.y.z>}"

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

if [[ ! "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]; then
  echo "invalid semantic version: $NEW_VERSION" >&2
  exit 2
fi

sed -E -i \
  "s#CANIC_CLI_VERSION=\"\\\$\\{CANIC_CLI_VERSION:-[0-9]+\\.[0-9]+\\.[0-9]+\\}\"#CANIC_CLI_VERSION=\"\\\${CANIC_CLI_VERSION:-$NEW_VERSION}\"#" \
  scripts/dev/install_dev.sh

sed -E -i \
  "s#https://raw.githubusercontent.com/dragginzgame/canic/v[0-9]+\\.[0-9]+\\.[0-9]+/scripts/dev/install_dev.sh#https://raw.githubusercontent.com/dragginzgame/canic/v$NEW_VERSION/scripts/dev/install_dev.sh#g" \
  README.md \
  crates/canic-host/README.md

EXPECTED_CLI_VERSION="CANIC_CLI_VERSION=\"\${CANIC_CLI_VERSION:-$NEW_VERSION}\""
if ! rg -q -F "$EXPECTED_CLI_VERSION" scripts/dev/install_dev.sh; then
  echo "install_dev.sh default CLI version did not sync to $NEW_VERSION" >&2
  exit 1
fi

EXPECTED_INSTALL_URL="https://raw.githubusercontent.com/dragginzgame/canic/v$NEW_VERSION/scripts/dev/install_dev.sh"
for file in crates/canic-host/README.md; do
  if ! rg -q -F "$EXPECTED_INSTALL_URL" "$file"; then
    echo "$file install URL did not sync to v$NEW_VERSION" >&2
    exit 1
  fi
done

STALE_INSTALL_URLS=$(
  rg 'https://raw\.githubusercontent\.com/dragginzgame/canic/v[0-9]+\.[0-9]+\.[0-9]+/scripts/dev/install_dev\.sh' \
    README.md crates/canic-host/README.md |
    rg -v -F "$EXPECTED_INSTALL_URL" || true
)
if [[ -n "$STALE_INSTALL_URLS" ]]; then
  echo "install_dev.sh URLs did not all sync to v$NEW_VERSION" >&2
  printf '%s\n' "$STALE_INSTALL_URLS" >&2
  exit 1
fi

echo "✅ Synced release surface version to $NEW_VERSION"
