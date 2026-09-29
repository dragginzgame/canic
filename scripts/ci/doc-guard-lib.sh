#!/usr/bin/env bash

if [ -z "${ROOT:-}" ]; then
    DOC_GUARD_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    ROOT="$(cd "$DOC_GUARD_LIB_DIR/../.." && pwd)"
fi

guard_path() {
    local path="$1"
    printf '%s\n' "${path#"$ROOT"/}"
}

require_file() {
    local path="$1"
    local label="$2"

    if [ ! -f "$path" ]; then
        echo "missing required $label file: $(guard_path "$path")" >&2
        exit 1
    fi
}

require_files() {
    local label="$1"
    shift
    local path=""

    for path in "$@"; do
        require_file "$path" "$label"
    done
}
