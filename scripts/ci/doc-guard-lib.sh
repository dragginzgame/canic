#!/usr/bin/env bash

if [ -z "${ROOT:-}" ]; then
    DOC_GUARD_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    ROOT="$(cd "$DOC_GUARD_LIB_DIR/../.." && pwd)"
fi

guard_path() {
    local path="$1"
    printf '%s\n' "${path#$ROOT/}"
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

require_text() {
    local path="$1"
    local needle="$2"
    local label="$3"

    if ! grep -Fq "$needle" "$path"; then
        echo "missing required $label text in $(guard_path "$path"): $needle" >&2
        exit 1
    fi
}

require_texts() {
    local path="$1"
    local label="$2"
    shift 2
    local needle=""

    for needle in "$@"; do
        require_text "$path" "$needle" "$label"
    done
}

forbid_operations_file() {
    local file_name="$1"
    local message="$2"

    if [ -e "$ROOT/docs/operations/$file_name" ]; then
        echo "$message" >&2
        exit 1
    fi
}

forbid_git_reference() {
    local needle="$1"
    local message="$2"
    shift 2

    if git -C "$ROOT" grep -n "$needle" -- "$@" >&2; then
        echo "$message" >&2
        exit 1
    fi
}
