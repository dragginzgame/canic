#!/usr/bin/env bash
set -euo pipefail

LABEL="runner disk"
MIN_FREE_MIB="${CANIC_CI_MIN_FREE_MIB:-0}"
TOP_LIMIT="${CANIC_CI_DISK_TOP_LIMIT:-16}"

usage() {
    cat >&2 <<'USAGE'
usage: check-runner-disk-space.sh [--label <name>] [--min-free-mib <mib>] [--top-limit <count>]

Print GitHub runner disk availability plus the largest likely CI disk consumers.
When --min-free-mib is set above zero, fail if the workspace filesystem has
less free space than that threshold.
USAGE
}

while [ "$#" -gt 0 ]; do
    case "$1" in
    --label)
        LABEL="${2:-}"
        if [ -z "$LABEL" ]; then
            echo "missing value for --label" >&2
            exit 2
        fi
        shift 2
        ;;
    --min-free-mib)
        MIN_FREE_MIB="${2:-}"
        if ! [[ "$MIN_FREE_MIB" =~ ^[0-9]+$ ]]; then
            echo "--min-free-mib must be a non-negative integer" >&2
            exit 2
        fi
        shift 2
        ;;
    --top-limit)
        TOP_LIMIT="${2:-}"
        if ! [[ "$TOP_LIMIT" =~ ^[0-9]+$ ]] || [ "$TOP_LIMIT" -eq 0 ]; then
            echo "--top-limit must be a positive integer" >&2
            exit 2
        fi
        shift 2
        ;;
    -h | --help)
        usage
        exit 0
        ;;
    *)
        echo "unknown argument: $1" >&2
        usage
        exit 2
        ;;
    esac
done

existing_unique_paths() {
    local seen=""
    local path

    for path in "$@"; do
        if [ -z "$path" ] || [ ! -e "$path" ]; then
            continue
        fi

        case "
$seen
" in
        *"
$path
"*) ;;
        *)
            printf '%s\n' "$path"
            seen="$seen
$path"
            ;;
        esac
    done
}

workspace_path() {
    local path="${GITHUB_WORKSPACE:-$PWD}"

    if [ -e "$path" ]; then
        printf '%s\n' "$path"
    else
        printf '%s\n' "$PWD"
    fi
}

available_mib() {
    local path="$1"

    df -Pm "$path" | awk 'NR == 2 { print $4 }'
}

print_filesystem_summary() {
    local workspace
    workspace="$(workspace_path)"

    echo "==> disk availability: $LABEL"
    mapfile -t paths < <(
        existing_unique_paths \
            "$workspace" \
            "${RUNNER_TEMP:-}" \
            "${TMPDIR:-/tmp}" \
            "$HOME/.cargo" \
            "$HOME/.rustup" \
            "/opt/hostedtoolcache"
    )

    if [ "${#paths[@]}" -eq 0 ]; then
        echo "no filesystem paths available for df"
        return
    fi

    df -h "${paths[@]}"
}

print_largest_under() {
    local path="$1"
    local label="$2"

    if [ ! -e "$path" ]; then
        return
    fi

    echo
    echo "==> largest entries under $label"
    du -xhd1 "$path" 2>/dev/null | sort -hr | awk -v limit="$TOP_LIMIT" 'NR <= limit { print }' || true
}

print_disk_consumers() {
    local workspace
    workspace="$(workspace_path)"

    echo
    echo "==> likely CI disk pressure sources"
    echo "- target/: Rust debug/test artifacts, restored rust-cache entries, wasm targets, and incremental state"
    echo "- .icp/: generated canister wasm artifacts staged for PocketIC and local ICP flows"
    echo "- \$HOME/.cargo and \$HOME/.rustup: installed Rust helper tools, registry/cache data, and toolchains"
    echo "- \$RUNNER_TEMP and /tmp: PocketIC server cache, temporary installs, and test scratch space"
    echo "- /opt/hostedtoolcache: preinstalled GitHub runner toolchains sharing the same root filesystem"

    print_largest_under "$workspace" "workspace"
    print_largest_under "$workspace/target" "workspace/target"
    print_largest_under "$workspace/.icp" "workspace/.icp"
    print_largest_under "$HOME/.cargo" "\$HOME/.cargo"
    print_largest_under "$HOME/.rustup" "\$HOME/.rustup"
    print_largest_under "${RUNNER_TEMP:-}" "\$RUNNER_TEMP"
    print_largest_under "${TMPDIR:-/tmp}" "\${TMPDIR:-/tmp}"
    print_largest_under "/opt/hostedtoolcache" "/opt/hostedtoolcache"
}

assert_min_free_space() {
    local workspace
    local free_mib

    if [ "$MIN_FREE_MIB" -eq 0 ]; then
        return
    fi

    workspace="$(workspace_path)"
    free_mib="$(available_mib "$workspace")"

    if [ "$free_mib" -lt "$MIN_FREE_MIB" ]; then
        echo
        echo "error: low GitHub runner disk space before $LABEL" >&2
        echo "available: ${free_mib} MiB; required: ${MIN_FREE_MIB} MiB" >&2
        echo "The runner is likely to fail later with 'No space left on device' while Rust writes target artifacts." >&2
        exit 1
    fi
}

print_filesystem_summary
print_disk_consumers
assert_min_free_space
