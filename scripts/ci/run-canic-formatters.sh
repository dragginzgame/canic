#!/usr/bin/env bash
set -euo pipefail

# Canic owns the formatter order and its three workspace roots.
[[ $# -ge 1 && ( "$1" == --write || "$1" == --check ) ]] || {
    echo 'usage: run-canic-formatters.sh --write|--check [WORKSPACE ...]' >&2; exit 2;
}
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
# shellcheck source=/dev/null
source "$ROOT/ci/tool-versions.env"
export CARGO_NET_OFFLINE=true RUSTUP_AUTO_INSTALL=0
bash "$ROOT/scripts/ci/check-format-tools.sh" "$SHARED_TOOLING_CARGO_SORT_VERSION"
check=()
fmt=()
if [[ "$1" == --check ]]; then check=(--check); fmt=(-- --check); fi
shift
cd "$ROOT"
cargo sort --workspace "${check[@]}"
cargo sort-derives "${check[@]}"
cargo fmt --all "${fmt[@]}"
for workspace in "$@"; do
    cargo sort --workspace "${check[@]}" --order workspace,package,lib,dependencies,dev-dependencies,build-dependencies,profile "$workspace"
    (cd "$workspace" && cargo fmt --all "${fmt[@]}")
done
