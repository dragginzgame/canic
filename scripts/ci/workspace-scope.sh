#!/usr/bin/env bash
# Shared default scope. External composition remains available through its
# explicit test selector and is never a release prerequisite.
canic_workspace_args() {
    printf '%s\n' --workspace \
        --exclude canic-icydb-lifecycle-schema \
        --exclude canic_icydb_lifecycle_probe
}
