#!/usr/bin/env bash
set -euo pipefail

NEW_VERSION="${1:?usage: sync-release-surface-version.sh <x.y.z>}"

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

sed -E -i \
  "s#CANIC_CLI_VERSION=\"\\\$\\{CANIC_CLI_VERSION:-[0-9]+\\.[0-9]+\\.[0-9]+\\}\"#CANIC_CLI_VERSION=\"\\\${CANIC_CLI_VERSION:-$NEW_VERSION}\"#" \
  scripts/dev/install_dev.sh

sed -E -i \
  "s#https://raw.githubusercontent.com/dragginzgame/canic/v[0-9]+\\.[0-9]+\\.[0-9]+/scripts/dev/install_dev.sh#https://raw.githubusercontent.com/dragginzgame/canic/v$NEW_VERSION/scripts/dev/install_dev.sh#g" \
  README.md \
  crates/canic-host/README.md
