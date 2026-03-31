#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

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

BASELINE_REPORT="N/A"
if [[ "$RUN_INDEX" -gt 1 ]]; then
  if [[ "$RUN_INDEX" -eq 2 ]]; then
    BASELINE_REPORT="docs/audits/reports/$MONTH_DIR/$DATE_UTC/instruction-footprint.md"
  else
    PREV_INDEX=$((RUN_INDEX - 1))
    BASELINE_REPORT="docs/audits/reports/$MONTH_DIR/$DATE_UTC/instruction-footprint-$PREV_INDEX.md"
  fi
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

echo "Running instruction audit report into $REPORT_PATH"
cargo test -p canic-tests --test instruction_audit generate_instruction_footprint_report -- --ignored --nocapture
echo "Wrote $REPORT_PATH"
