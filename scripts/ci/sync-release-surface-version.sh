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

EXPECTED_CLI_VERSION="CANIC_CLI_VERSION=\"\${CANIC_CLI_VERSION:-$NEW_VERSION}\""
if ! rg -q -F "$EXPECTED_CLI_VERSION" scripts/dev/install_dev.sh; then
  echo "install_dev.sh default CLI version did not sync to $NEW_VERSION" >&2
  exit 1
fi

echo "✅ Synced release surface version to $NEW_VERSION"
