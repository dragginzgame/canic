#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat >&2 <<'USAGE'
usage: scripts/restore/apply_journal.sh --journal <file> [--network <name>] [--canic <path>] [--dfx <path>] [--status-out <file>] [--command-out <file>] [--report-out <file>]
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
status_out="restore-apply-status.json"
command_out="restore-apply-command.json"
report_out="restore-apply-report.json"

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
        --status-out)
            require_value "$1" "${2:-}"
            status_out="$2"
            shift 2
            ;;
        --command-out)
            require_value "$1" "${2:-}"
            command_out="$2"
            shift 2
            ;;
        --report-out)
            require_value "$1" "${2:-}"
            report_out="$2"
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

if ! command -v jq >/dev/null 2>&1; then
    echo "jq is required to run restore apply journal commands" >&2
    exit 1
fi

if ! command -v "$canic_bin" >/dev/null 2>&1; then
    echo "canic binary is not available: $canic_bin" >&2
    exit 1
fi

if ! command -v "$dfx_bin" >/dev/null 2>&1; then
    echo "dfx binary is not available: $dfx_bin" >&2
    exit 1
fi

while true; do
    "$canic_bin" restore apply-status \
        --journal "$journal" \
        --out "$status_out" \
        --require-ready \
        --require-no-pending \
        --require-no-failed

    if "$canic_bin" restore apply-status \
        --journal "$journal" \
        --out "$status_out" \
        --require-complete; then
        "$canic_bin" restore apply-report \
            --journal "$journal" \
            --out "$report_out" \
            --require-no-attention
        exit 0
    fi

    "$canic_bin" restore apply-command \
        --journal "$journal" \
        --dfx "$dfx_bin" \
        --network "$network" \
        --out "$command_out" \
        --require-command

    sequence="$(jq -r '.operation.sequence // empty' "$command_out")"
    if [ -z "$sequence" ]; then
        echo "restore apply command did not include an operation sequence" >&2
        exit 1
    fi

    command="$(jq -r '[.command.program] + .command.args | @sh' "$command_out")"
    updated_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

    "$canic_bin" restore apply-claim \
        --journal "$journal" \
        --sequence "$sequence" \
        --updated-at "$updated_at" \
        --out "$journal"

    if eval "$command"; then
        "$canic_bin" restore apply-mark \
            --journal "$journal" \
            --sequence "$sequence" \
            --state completed \
            --updated-at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
            --out "$journal" \
            --require-pending
    else
        status="$?"
        "$canic_bin" restore apply-mark \
            --journal "$journal" \
            --sequence "$sequence" \
            --state failed \
            --reason "runner-command-exit-$status" \
            --updated-at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
            --out "$journal" \
            --require-pending
        "$canic_bin" restore apply-report \
            --journal "$journal" \
            --out "$report_out"
        exit "$status"
    fi
done
