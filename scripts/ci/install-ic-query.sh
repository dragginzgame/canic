#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
if [ -z "${CANIC_ICQ_VERSION:-${CANIC_IC_QUERY_VERSION:-}}" ] &&
    [ -f "$ROOT_DIR/tool-versions.env" ]; then
    # shellcheck source=tool-versions.env
    source "$ROOT_DIR/tool-versions.env"
fi

version="${1:-${CANIC_ICQ_VERSION:-${CANIC_IC_QUERY_VERSION:-}}}"
cargo_bin_dir="${CARGO_HOME:-$HOME/.cargo}/bin"
install_path="${CANIC_ICQ_PATH:-${CANIC_IC_QUERY_PATH:-}}"
install_git="${CANIC_ICQ_GIT:-${CANIC_IC_QUERY_GIT:-}}"
install_rev="${CANIC_ICQ_REV:-${CANIC_IC_QUERY_REV:-}}"

icq_version_matches() {
    local output="$1"
    local required_version="$2"

    case "$output" in
        *" $required_version"|*" $required_version "*)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

latest_ic_query_version() {
    local latest_version=""
    local search_output=""

    search_output="$(cargo search ic-query --limit 1 2>/dev/null)" || return 1
    latest_version="$(
        printf '%s\n' "$search_output" |
            sed -n 's/^ic-query = "\([^"]\+\)".*/\1/p' |
            head -n 1
    )"
    if [[ "$latest_version" =~ ^[0-9]+(\.[0-9]+){1,2}([-+][0-9A-Za-z.-]+)?$ ]]; then
        printf '%s\n' "$latest_version"
        return 0
    fi
    return 1
}

warn_if_newer_ic_query_exists() {
    local latest_version=""
    local pinned_version="$1"

    if [ "${CANIC_ICQ_LATEST_CHECK:-1}" = "0" ]; then
        return 0
    fi
    latest_version="$(latest_ic_query_version)" || return 0
    if [ "$latest_version" != "$pinned_version" ]; then
        echo "warning: crates.io latest ic-query release is v$latest_version; Canic remains pinned to v$pinned_version in tool-versions.env" >&2
    fi
}

if [ -z "$version" ]; then
    echo "missing ic-query version; set CANIC_ICQ_VERSION or update tool-versions.env" >&2
    exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is required to install ic-query" >&2
    exit 1
fi

mkdir -p "$cargo_bin_dir"
export PATH="$cargo_bin_dir:$PATH"
hash -r 2>/dev/null || true

if [ -n "${GITHUB_PATH:-}" ]; then
    printf '%s\n' "$cargo_bin_dir" >>"$GITHUB_PATH"
fi

if [ -n "$install_path" ]; then
    if [ ! -f "$install_path/Cargo.toml" ]; then
        echo "CANIC_ICQ_PATH does not point at an ic-query crate: $install_path" >&2
        exit 1
    fi
    cargo install --locked --path "$install_path" --bin icq
elif [ -n "$install_git" ]; then
    install_args=(install --locked --git "$install_git")
    if [ -n "$install_rev" ]; then
        install_args+=(--rev "$install_rev")
    fi
    install_args+=(--bin icq ic-query)
    cargo "${install_args[@]}"
else
    cargo install --locked ic-query --version "$version" --bin icq
fi
hash -r 2>/dev/null || true

if ! command -v icq >/dev/null 2>&1; then
    echo "ic-query installer completed, but icq is not on PATH" >&2
    echo "expected $cargo_bin_dir to contain icq" >&2
    exit 1
fi

icq_path="$(command -v icq)"
if ! icq_version_output="$(icq --version 2>&1)"; then
    echo "ic-query installer completed, but icq is not working" >&2
    echo "$icq_version_output" >&2
    echo "resolved path: $icq_path" >&2
    exit 1
fi

if ! icq_version_matches "$icq_version_output" "$version"; then
    echo "ic-query installer completed, but icq is not the requested version" >&2
    echo "found: $icq_version_output ($icq_path)" >&2
    echo "required: icq $version" >&2
    echo "expected install directory: $cargo_bin_dir" >&2
    exit 1
fi

echo "icq ready: $icq_version_output ($icq_path)" >&2
warn_if_newer_ic_query_exists "$version"
