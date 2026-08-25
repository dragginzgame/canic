#!/bin/bash

set -euo pipefail

SELF_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SELF_DIR/../.." && pwd)"
VERSION_READER="$ROOT_DIR/scripts/ci/read-workspace-version.sh"
cd "$ROOT_DIR"

PUBLISH_DRY_RUN="${PUBLISH_DRY_RUN:-0}"
PUBLISH_FROM="${PUBLISH_FROM:-}"
PUBLISH_POLL_SECS="${PUBLISH_POLL_SECS:-10}"
PUBLISH_TIMEOUT_SECS="${PUBLISH_TIMEOUT_SECS:-300}"

PUBLISH_ORDER=(
    canic-backup
    canic-core
    canic-control-plane
    canic-macros
    canic
    canic-fleet-coordinator
    canic-host
    canic-cli
    canic-wasm-store
)

if [ -n "$PUBLISH_FROM" ]; then
    publish_from_is_known=0
    for crate in "${PUBLISH_ORDER[@]}"; do
        if [ "$crate" = "$PUBLISH_FROM" ]; then
            publish_from_is_known=1
            break
        fi
    done
    if [ "$publish_from_is_known" -ne 1 ]; then
        echo "PUBLISH_FROM=$PUBLISH_FROM is not in the publish order" >&2
        exit 1
    fi
fi

# Fails before any publish attempt if an explicitly publishable workspace crate
# depends at runtime or build time on a local crate marked `publish = false`.
validate_publish_manifest_boundary() {
    cargo test --locked -p canic --test workspace_manifest publishable_members_do_not_depend_on_unpublished_workspace_members
}

# Returns success once crates.io reports the expected version for a crate.
registry_has_version() {
    local crate="$1"
    local version="$2"

    cargo info "$crate@$version" --registry crates-io >/dev/null 2>&1
}

# Waits until crates.io exposes the freshly published version.
wait_for_registry_version() {
    local crate="$1"
    local version="$2"
    local deadline=$((SECONDS + PUBLISH_TIMEOUT_SECS))

    while [ "$SECONDS" -lt "$deadline" ]; do
        if registry_has_version "$crate" "$version"; then
            echo "Observed $crate $version on crates.io"
            return 0
        fi

        echo "Waiting for crates.io to expose $crate $version..."
        sleep "$PUBLISH_POLL_SECS"
    done

    echo "Timed out waiting for $crate $version to appear on crates.io" >&2
    return 1
}

version="$(bash "$VERSION_READER")"

bash "$ROOT_DIR/scripts/ci/check-release-candidate.sh"
validate_publish_manifest_boundary

started=0
if [ -z "$PUBLISH_FROM" ]; then
    started=1
fi

for crate in "${PUBLISH_ORDER[@]}"; do
    if [ "$started" -eq 0 ]; then
        if [ "$crate" != "$PUBLISH_FROM" ]; then
            continue
        fi
        started=1
    fi

    if registry_has_version "$crate" "$version"; then
        echo "Skipping $crate $version (already on crates.io)"
        continue
    fi

    echo "Publishing $crate $version"
    publish_args=(publish -p "$crate" --locked)
    if [ "$crate" = "canic-core" ]; then
        publish_args+=(--no-verify)
    fi
    if [ "$PUBLISH_DRY_RUN" = "1" ]; then
        publish_args+=(--dry-run)
    fi

    cargo "${publish_args[@]}"

    if [ "$PUBLISH_DRY_RUN" != "1" ]; then
        wait_for_registry_version "$crate" "$version"
    fi
done

if [ "$PUBLISH_DRY_RUN" != "1" ]; then
    missing_packages=()
    for crate in "${PUBLISH_ORDER[@]}"; do
        if ! registry_has_version "$crate" "$version"; then
            missing_packages+=("$crate")
        fi
    done
    if [ "${#missing_packages[@]}" -ne 0 ]; then
        echo "Publication is incomplete for $version:" >&2
        printf '  %s\n' "${missing_packages[@]}" >&2
        exit 1
    fi
    echo "Verified complete matching Canic package set at $version"
fi
