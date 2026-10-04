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
        exec cargo clippy --locked "${workspace_args[@]}" --all-targets --all-features -- "$@"
        ;;
    *)
        echo 'usage: run-workspace-cargo.sh <build|check> [cargo options] | clippy [lint options]' >&2
        exit 2
        ;;
esac
