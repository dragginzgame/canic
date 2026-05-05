#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat >&2 <<'USAGE'
usage: scripts/restore/apply_journal.sh --journal <file> [--dry-run | --execute | --unclaim-pending] [--network <name>] [--canic <path>] [--dfx <path>] [--out <file>] [--max-steps <n>] [--require-complete] [--require-no-attention] [--require-run-mode <text>] [--require-stopped-reason <text>] [--require-next-action <text>] [--require-executed-count <n>]

Compatibility aliases:
  --report-out <file>   same as --out
  --status-out <file>   accepted for older callers, ignored
  --command-out <file>  accepted for older callers, ignored
USAGE
}

require_value() {
    if [ -z "${2:-}" ]; then
        echo "$1 requires a value" >&2
        usage
        exit 2
    fi
}

journal=""
network="${DFX_NETWORK:-local}"
canic_bin="${CANIC_BIN:-canic}"
dfx_bin="${DFX_BIN:-dfx}"
out="restore-run.json"
max_steps=""
require_complete=0
require_no_attention=0
mode="--execute"
mode_count=0
require_run_mode=""
require_stopped_reason=""
require_next_action=""
require_executed_count=""

while [ "$#" -gt 0 ]; do
    case "$1" in
        --journal)
            require_value "$1" "${2:-}"
            journal="$2"
            shift 2
            ;;
        --network)
            require_value "$1" "${2:-}"
            network="$2"
            shift 2
            ;;
        --canic)
            require_value "$1" "${2:-}"
            canic_bin="$2"
            shift 2
            ;;
        --dfx)
            require_value "$1" "${2:-}"
            dfx_bin="$2"
            shift 2
            ;;
        --out)
            require_value "$1" "${2:-}"
            out="$2"
            shift 2
            ;;
        --max-steps)
            require_value "$1" "${2:-}"
            max_steps="$2"
            shift 2
            ;;
        --dry-run)
            mode="--dry-run"
            mode_count=$((mode_count + 1))
            shift
            ;;
        --execute)
            mode="--execute"
            mode_count=$((mode_count + 1))
            shift
            ;;
        --unclaim-pending)
            mode="--unclaim-pending"
            mode_count=$((mode_count + 1))
            shift
            ;;
        --require-complete)
            require_complete=1
            shift
            ;;
        --require-no-attention)
            require_no_attention=1
            shift
            ;;
        --require-run-mode)
            require_value "$1" "${2:-}"
            require_run_mode="$2"
            shift 2
            ;;
        --require-stopped-reason)
            require_value "$1" "${2:-}"
            require_stopped_reason="$2"
            shift 2
            ;;
        --require-next-action)
            require_value "$1" "${2:-}"
            require_next_action="$2"
            shift 2
            ;;
        --require-executed-count)
            require_value "$1" "${2:-}"
            require_executed_count="$2"
            shift 2
            ;;
        --report-out)
            require_value "$1" "${2:-}"
            out="$2"
            shift 2
            ;;
        --status-out|--command-out)
            require_value "$1" "${2:-}"
            shift 2
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            echo "unknown option: $1" >&2
            usage
            exit 2
            ;;
    esac
done

if [ -z "$journal" ]; then
    echo "--journal is required" >&2
    usage
    exit 2
fi

if [ "$mode_count" -gt 1 ]; then
    echo "use only one runner mode: --dry-run, --execute, or --unclaim-pending" >&2
    usage
    exit 2
fi

if ! command -v "$canic_bin" >/dev/null 2>&1; then
    echo "canic binary is not available: $canic_bin" >&2
    exit 1
fi

if [ "$mode" = "--execute" ] && ! command -v "$dfx_bin" >/dev/null 2>&1; then
    echo "dfx binary is not available: $dfx_bin" >&2
    exit 1
fi

run_args=(
    restore run
    --journal "$journal"
    "$mode"
    --dfx "$dfx_bin"
    --network "$network"
    --out "$out"
)

if [ -n "$max_steps" ]; then
    run_args+=(--max-steps "$max_steps")
fi

if [ "$require_complete" -eq 1 ]; then
    run_args+=(--require-complete)
fi

if [ "$require_no_attention" -eq 1 ]; then
    run_args+=(--require-no-attention)
fi

if [ -n "$require_run_mode" ]; then
    run_args+=(--require-run-mode "$require_run_mode")
fi

if [ -n "$require_stopped_reason" ]; then
    run_args+=(--require-stopped-reason "$require_stopped_reason")
fi

if [ -n "$require_next_action" ]; then
    run_args+=(--require-next-action "$require_next_action")
fi

if [ -n "$require_executed_count" ]; then
    run_args+=(--require-executed-count "$require_executed_count")
fi

exec "$canic_bin" "${run_args[@]}"
