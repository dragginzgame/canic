#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=scripts/ci/workspace-scope.sh
source "$ROOT/scripts/ci/workspace-scope.sh"
mapfile -t workspace_args < <(canic_workspace_args)
command="${1:-}"
[[ $# -eq 0 ]] || shift
cd "$ROOT"

case "$command" in
    build | check)
        exec cargo "$command" --locked "${workspace_args[@]}" "$@"
        ;;
    clippy)
        # The integration package's only opt-in feature is external composition.
        # Its ordinary all-targets selection still includes every maintained test.
        status=0
        cargo clippy --locked "${workspace_args[@]}" --exclude canic-tests \
            --all-targets --all-features -- "$@" || status=$?
        cargo clippy --locked -p canic-tests --all-targets -- "$@" || status=$?
        echo 'Optional external composition is not selected; qualify it explicitly when needed.'
        exit "$status"
        ;;
    *)
        echo 'usage: run-workspace-cargo.sh <build|check> [cargo options] | clippy [lint options]' >&2
        exit 2
        ;;
esac
