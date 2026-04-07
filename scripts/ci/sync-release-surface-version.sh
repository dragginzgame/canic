#!/usr/bin/env bash
set -euo pipefail

NEW_VERSION="${1:?usage: sync-release-surface-version.sh <x.y.z>}"

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

sed -E -i \
  "s#CANIC_INSTALLER_VERSION=\"\\\$\\{CANIC_INSTALLER_VERSION:-[0-9]+\\.[0-9]+\\.[0-9]+\\}\"#CANIC_INSTALLER_VERSION=\"\\\${CANIC_INSTALLER_VERSION:-$NEW_VERSION}\"#" \
  scripts/dev/install_dev.sh

sed -E -i \
  "s#https://raw.githubusercontent.com/dragginzgame/canic/v[0-9]+\\.[0-9]+\\.[0-9]+/scripts/dev/install_dev.sh#https://raw.githubusercontent.com/dragginzgame/canic/v$NEW_VERSION/scripts/dev/install_dev.sh#g" \
  README.md \
  crates/canic-installer/README.md

sed -E -i \
  "s#- \`canic-installer\` \`[0-9]+\\.[0-9]+\\.[0-9]+\`#- \`canic-installer\` \`$NEW_VERSION\`#" \
  README.md
