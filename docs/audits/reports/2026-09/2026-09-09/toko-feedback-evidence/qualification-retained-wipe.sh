#!/usr/bin/env bash
set -euo pipefail
source /tmp/toko-feedback-fleet/qualification-retained-context.sh
export PATH="${ROOT_DIR}/qualification-retained-bin:${PATH}"
[[ "$(prepare_local_reinstall)" == "$release" ]]
converge_local_fleet reinstall
canic --environment "$ENVIRONMENT" info env "$FLEET_NAME" > qualification-retained-environment-after.txt
cp "$FLEET_ENSURE_DIRECTORY/state.json" qualification-retained-state-after.json
