#!/usr/bin/env bash

set -euo pipefail

CONFIG="fleets/demo/canic.toml"
MODE="default"

while [ "$#" -gt 0 ]; do
    case "$1" in
    --config)
        CONFIG="$2"
        shift 2
        ;;
    --ci-order | --root-last)
        MODE="root-last"
        shift
        ;;
    --exclude-root)
        MODE="exclude-root"
        shift
        ;;
    *)
        echo "unknown option: $1" >&2
        exit 1
        ;;
    esac
done

[ -f "$CONFIG" ] || {
    echo "missing Canic config: $CONFIG" >&2
    exit 1
}

mapfile -t ROLES < <(
    sed -n 's/^\[subnets\.[^.]*\.canisters\.\([^].]*\)\]$/\1/p' "$CONFIG" |
        awk '!seen[$0]++'
)

case "$MODE" in
default)
    printf '%s\n' "${ROLES[@]}"
    ;;
exclude-root)
    for role in "${ROLES[@]}"; do
        [ "$role" = "root" ] || printf '%s\n' "$role"
    done
    ;;
root-last)
    root_seen=0
    for role in "${ROLES[@]}"; do
        if [ "$role" = "root" ]; then
            root_seen=1
        else
            printf '%s\n' "$role"
        fi
    done
    [ "$root_seen" -eq 0 ] || printf '%s\n' root
    ;;
*)
    echo "unknown mode: $MODE" >&2
    exit 1
    ;;
esac
