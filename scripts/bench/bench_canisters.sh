#!/usr/bin/env bash
set -euo pipefail

# Runs `make test` and then hammers canister creation until failure or an optional cap.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../env.sh"
ROOT_DIR="$ROOT"

usage() {
    cat <<'EOF'
Usage: bench_canisters.sh [--max N] [--root <principal>] [--method <name>]

Environment:
  TMPDIR               Override tmp dir (defaults to $ROOT/target_tmp)
  CARGO_TARGET_DIR     Defaults to TMPDIR
  MAX_CANISTERS        Same as --max
  ROOT_CANISTER_ID     Same as --root (defaults to dfx canister id root)
  CANISTER_METHOD      Same as --method (default: create_blank on the root canister)
EOF
}

MAX_CANISTERS="${MAX_CANISTERS:-0}"
ROOT_CANISTER_ID="${ROOT_CANISTER_ID:-}"
CANISTER_METHOD="${CANISTER_METHOD:-create_blank}"
REPORT_EVERY="${REPORT_EVERY:-10}"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --max)
            MAX_CANISTERS="$2"
            shift 2
            ;;
        --root)
            ROOT_CANISTER_ID="$2"
            shift 2
            ;;
        --method)
            CANISTER_METHOD="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            usage
            exit 1
            ;;
    esac
done

if [[ "$REPORT_EVERY" -le 0 ]]; then
    echo "REPORT_EVERY must be >= 1" >&2
    exit 1
fi

export TMPDIR="${TMPDIR:-$ROOT_DIR/target_tmp}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$TMPDIR}"

echo "Running make test (TMPDIR=$TMPDIR, CARGO_TARGET_DIR=$CARGO_TARGET_DIR)..."
(cd "$ROOT_DIR" && make test)

dargs=(--network "$NETWORK")
if [[ -z "$ROOT_CANISTER_ID" ]]; then
    ROOT_CANISTER_ID="$(dfx "${dargs[@]}" canister id root)"
fi

echo "Benchmarking canister creation against $ROOT_CANISTER_ID using '$CANISTER_METHOD'"
[[ "$MAX_CANISTERS" -gt 0 ]] && echo "Max creations: $MAX_CANISTERS (0 = unlimited)"
[[ "$REPORT_EVERY" -gt 0 ]] && echo "Reporting every $REPORT_EVERY creations"

created=0
start_secs="$(date +%s)"
last_report_ms="$(date +%s%3N)"

cleanup() {
    local elapsed="$(( $(date +%s) - start_secs ))"
    (( elapsed == 0 )) && elapsed=1
    local rate="$(( created / elapsed ))"
    echo ""
    echo "Canisters created: $created"
    echo "Elapsed: ${elapsed}s (approx ${rate}/s)"
}
trap cleanup EXIT INT TERM

while :; do
    if [[ "$MAX_CANISTERS" -gt 0 && "$created" -ge "$MAX_CANISTERS" ]]; then
        echo "Reached max canister count ($MAX_CANISTERS)."
        break
    fi

    output="$(dfx "${dargs[@]}" canister call "$ROOT_CANISTER_ID" "$CANISTER_METHOD" 2>&1)" || {
        echo "Call failed after $created creations:"
        echo "$output"
        break
    }

    if [[ "$output" == *"Err"* ]]; then
        echo "Received error response after $created creations:"
        echo "$output"
        break
    fi

    created=$(( created + 1 ))
    if (( created % REPORT_EVERY == 0 )); then
        now_ms="$(date +%s%3N)"
        window_ms="$(( now_ms - last_report_ms ))"
        avg_ms="$(( window_ms / REPORT_EVERY ))"
        echo "Created $created canisters so far... ~${avg_ms}ms/call over last $REPORT_EVERY (window ${window_ms}ms)"
        last_report_ms="$now_ms"
    fi
done
