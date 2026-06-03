#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

usage() {
  cat <<'EOF'
Usage: scripts/ci/instruction-audit-report.sh [OPTIONS]

Options:
  --estimate-execution-cycles
      Add offline execution-cycle estimates for update instruction rows.
  --estimate-node-count <13|34>
      Use Canic's pinned IC cost table for the supplied operator-assumed node count.
  --cycles-per-billion-instructions <cycles>
      Use an explicit operator-supplied rate. Wins over --estimate-node-count.
  -h, --help
      Print this help.
EOF
}

ESTIMATE_EXECUTION_CYCLES=0
ESTIMATE_NODE_COUNT=""
CYCLES_PER_BILLION_INSTRUCTIONS=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --estimate-execution-cycles)
      ESTIMATE_EXECUTION_CYCLES=1
      shift
      ;;
    --estimate-node-count)
      if [[ $# -lt 2 ]]; then
        echo "error: --estimate-node-count requires a value" >&2
        usage >&2
        exit 2
      fi
      ESTIMATE_NODE_COUNT="$2"
      shift 2
      ;;
    --cycles-per-billion-instructions)
      if [[ $# -lt 2 ]]; then
        echo "error: --cycles-per-billion-instructions requires a value" >&2
        usage >&2
        exit 2
      fi
      CYCLES_PER_BILLION_INSTRUCTIONS="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -n "$ESTIMATE_NODE_COUNT" && ! "$ESTIMATE_NODE_COUNT" =~ ^[1-9][0-9]*$ ]]; then
  echo "error: --estimate-node-count must be a positive integer" >&2
  exit 2
fi

if [[ -n "$CYCLES_PER_BILLION_INSTRUCTIONS" && ! "$CYCLES_PER_BILLION_INSTRUCTIONS" =~ ^[1-9][0-9]*$ ]]; then
  echo "error: --cycles-per-billion-instructions must be a positive integer" >&2
  exit 2
fi

if [[ "$ESTIMATE_EXECUTION_CYCLES" == "1" && -z "$ESTIMATE_NODE_COUNT" && -z "$CYCLES_PER_BILLION_INSTRUCTIONS" ]]; then
  echo "error: --estimate-execution-cycles requires --estimate-node-count or --cycles-per-billion-instructions" >&2
  exit 2
fi

if [[ "$ESTIMATE_EXECUTION_CYCLES" != "1" && ( -n "$ESTIMATE_NODE_COUNT" || -n "$CYCLES_PER_BILLION_INSTRUCTIONS" ) ]]; then
  echo "error: estimate source flags require --estimate-execution-cycles" >&2
  exit 2
fi

DATE_UTC="$(date -u +%F)"
MONTH_DIR="${DATE_UTC%*-*}"
DAY_DIR="$ROOT/docs/audits/reports/$MONTH_DIR/$DATE_UTC"

mkdir -p "$DAY_DIR/artifacts"

RUN_STEM="instruction-footprint"
RUN_INDEX=1
while [[ -e "$DAY_DIR/$RUN_STEM.md" ]]; do
  ((RUN_INDEX+=1))
  RUN_STEM="instruction-footprint-$RUN_INDEX"
done

REPORT_PATH="$DAY_DIR/$RUN_STEM.md"
ARTIFACTS_DIR="$DAY_DIR/artifacts/$RUN_STEM"
mkdir -p "$ARTIFACTS_DIR"

BASELINE_REPORT="$(
  find docs/audits/reports \
    -type f \
    \( -name 'instruction-footprint.md' -o -name 'instruction-footprint-[0-9]*.md' \) \
    ! -path "${REPORT_PATH#$ROOT/}" \
    -print \
    | sort -V \
    | tail -n 1
)"
if [[ -z "$BASELINE_REPORT" ]]; then
  BASELINE_REPORT="N/A"
fi

CODE_SNAPSHOT="$(git rev-parse --short HEAD)"
BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [[ -n "$(git status --short)" ]]; then
  WORKTREE="dirty"
else
  WORKTREE="clean"
fi
RUN_TIMESTAMP_UTC="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

export CANIC_INSTRUCTION_AUDIT_REPORT_PATH="$REPORT_PATH"
export CANIC_INSTRUCTION_AUDIT_ARTIFACTS_DIR="$ARTIFACTS_DIR"
export CANIC_INSTRUCTION_AUDIT_BASELINE_REPORT="$BASELINE_REPORT"
export CANIC_INSTRUCTION_AUDIT_CODE_SNAPSHOT="$CODE_SNAPSHOT"
export CANIC_INSTRUCTION_AUDIT_BRANCH="$BRANCH"
export CANIC_INSTRUCTION_AUDIT_WORKTREE="$WORKTREE"
export CANIC_INSTRUCTION_AUDIT_TIMESTAMP_UTC="$RUN_TIMESTAMP_UTC"
export CANIC_INSTRUCTION_AUDIT_ESTIMATE_EXECUTION_CYCLES="$ESTIMATE_EXECUTION_CYCLES"
export CANIC_INSTRUCTION_AUDIT_ESTIMATE_NODE_COUNT="$ESTIMATE_NODE_COUNT"
export CANIC_INSTRUCTION_AUDIT_CYCLES_PER_BILLION_INSTRUCTIONS="$CYCLES_PER_BILLION_INSTRUCTIONS"

echo "Running instruction audit report into $REPORT_PATH"
cargo test -p canic-tests --test instruction_audit generate_instruction_footprint_report -- --ignored --nocapture
echo "Wrote $REPORT_PATH"
