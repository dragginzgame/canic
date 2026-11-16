#!/usr/bin/env bash
# Shared companions: scripts/ci/read-cargo-workspace-rust-version.sh
set -euo pipefail

usage() {
    echo 'usage: prepare-rust-toolchain.sh TOOLCHAIN [--component NAME[,NAME]...] [--target TARGET]...' >&2
    echo '       prepare-rust-toolchain.sh --toolchain-file FILE' >&2
    echo '       prepare-rust-toolchain.sh --workspace-manifest FILE [--component NAME[,NAME]...] [--target TARGET]...' >&2
    exit 2
}

if [[ "${1:-}" == --toolchain-file || "${1:-}" == --workspace-manifest ]]; then
    [[ $# -ge 2 && -f "$2" ]] || usage
    mode="$1" file="$2"
    shift 2
    # All parsing and argument admission finish before the first rustup effect.
    if [[ "$mode" == --toolchain-file ]]; then
        [[ $# == 0 ]] || usage
        case "$file$PWD" in *$'\n'*|*$'\r'*) echo 'Rust policy path contains CR/LF' >&2; exit 2 ;; esac
        [[ "$file" == /* ]] || file="$PWD/$file"
        directory="$(cd -P -- "${file%/*}" && printf '%s/.' "$PWD")"
        case "$directory" in *$'\n'*|*$'\r'*) echo 'resolved Rust policy directory contains CR/LF' >&2; exit 2 ;; esac
        document="$("${YQ:-yq}" -p toml -o json '.' "$file")"
        selection="$(printf '%s' "$document" | jq -ers '
            def token: type == "string" and test("^[A-Za-z0-9]") and
                (test("[^A-Za-z0-9_.+-]") | not);
            if length != 1 then error("expected one Rust policy document") else .[0] end |
            select(type == "object") |
            .toolchain | select(type == "object") |
            select(.channel | token) |
            select((has("profile") | not) or (.profile | IN("minimal", "default", "complete"))) |
            select((has("path") | not)) |
            select((has("components") | not) or (.components | type == "array" and all(.[]; token))) |
            select((has("targets") | not) or (.targets | type == "array" and all(.[]; token))) |
            .channel, "--profile", (.profile // "minimal"),
            ((.components // [])[] | "--component", .),
            ((.targets // [])[] | "--target", .)
        ')" || { status=$?; echo 'Rust policy has no valid toolchain selection' >&2; exit "$status"; }
        arguments=(toolchain install)
        while IFS= read -r argument; do arguments+=("$argument"); done <<< "$selection"
        exec rustup "${arguments[@]}"
    fi
    script="${BASH_SOURCE[0]}"
    [[ "$script" == /* ]] || script="$PWD/$script"
    version="$(bash "${script%/*}/../ci/read-cargo-workspace-rust-version.sh" "$file")"
    set -- "$version" "$@"
fi

[[ $# -ge 1 && -n "$1" && "$1" != -* && "$1" != *[[:space:]]* ]] || usage
arguments=(toolchain install "$1" --profile minimal)
shift
while [[ $# -gt 0 ]]; do
    [[ $# -ge 2 && -n "$2" && "$2" != -* && "$2" != *[[:space:]]* ]] || usage
    case "$1" in
        --component)
            [[ "$2" != ,* && "$2" != *, && "$2" != *,,* ]] || usage
            ;;
        --target) ;;
        *) usage ;;
    esac
    arguments+=("$1" "$2")
    shift 2
done
# Admit the complete request before performing any installation. Never change
# the default toolchain or turn a missing validation prerequisite into setup.
exec rustup "${arguments[@]}"
