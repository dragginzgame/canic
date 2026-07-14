#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
ORIGINAL_ARGS=("$@")

METHOD_ID="CANIC-INSTRUCTION-001"
METHOD_VERSION="1"
DEFINITION_PATH="docs/audits/recurring/system/instruction-footprint.md"

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
  --estimate-canister-principal <principal>
      Resolve a canister principal through the cached mainnet subnet catalog.
  --allow-stale-subnet-catalog
      Permit catalog-derived estimates from stale cached catalog data.
  --subnet-catalog-stale-after <duration>
      Mark cached catalog data stale after this duration; defaults to 7d.
  -h, --help
      Print this help.
EOF
}

ESTIMATE_EXECUTION_CYCLES=0
ESTIMATE_NODE_COUNT=""
CYCLES_PER_BILLION_INSTRUCTIONS=""
ESTIMATE_CANISTER_PRINCIPAL=""
ALLOW_STALE_SUBNET_CATALOG=0
SUBNET_CATALOG_STALE_AFTER=""

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
    --estimate-canister-principal)
      if [[ $# -lt 2 ]]; then
        echo "error: --estimate-canister-principal requires a value" >&2
        usage >&2
        exit 2
      fi
      ESTIMATE_CANISTER_PRINCIPAL="$2"
      shift 2
      ;;
    --allow-stale-subnet-catalog)
      ALLOW_STALE_SUBNET_CATALOG=1
      shift
      ;;
    --subnet-catalog-stale-after)
      if [[ $# -lt 2 ]]; then
        echo "error: --subnet-catalog-stale-after requires a value" >&2
        usage >&2
        exit 2
      fi
      SUBNET_CATALOG_STALE_AFTER="$2"
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

if [[ -n "$SUBNET_CATALOG_STALE_AFTER" && ! "$SUBNET_CATALOG_STALE_AFTER" =~ ^[1-9][0-9]*(s|m|h|d)?$ ]]; then
  echo "error: --subnet-catalog-stale-after must be positive seconds or a value ending in s, m, h, or d" >&2
  exit 2
fi

if [[ "$ESTIMATE_EXECUTION_CYCLES" == "1" && -z "$ESTIMATE_NODE_COUNT" && -z "$CYCLES_PER_BILLION_INSTRUCTIONS" && -z "$ESTIMATE_CANISTER_PRINCIPAL" ]]; then
  echo "error: --estimate-execution-cycles requires --estimate-node-count, --cycles-per-billion-instructions, or --estimate-canister-principal" >&2
  exit 2
fi

if [[ "$ESTIMATE_EXECUTION_CYCLES" != "1" && ( -n "$ESTIMATE_NODE_COUNT" || -n "$CYCLES_PER_BILLION_INSTRUCTIONS" || -n "$ESTIMATE_CANISTER_PRINCIPAL" || "$ALLOW_STALE_SUBNET_CATALOG" == "1" || -n "$SUBNET_CATALOG_STALE_AFTER" ) ]]; then
  echo "error: estimate source flags require --estimate-execution-cycles" >&2
  exit 2
fi

if [[ "$ESTIMATE_EXECUTION_CYCLES" == "1" && -z "$ESTIMATE_CANISTER_PRINCIPAL" && ( "$ALLOW_STALE_SUBNET_CATALOG" == "1" || -n "$SUBNET_CATALOG_STALE_AFTER" ) ]]; then
  echo "error: catalog stale flags require --estimate-canister-principal" >&2
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

if [[ "$RUN_INDEX" -eq 1 ]]; then
  BASELINE_REPORT="N/A"
else
  BASELINE_REPORT="docs/audits/reports/$MONTH_DIR/$DATE_UTC/instruction-footprint.md"
fi

CODE_SNAPSHOT="$(git rev-parse HEAD)"
BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [[ -n "$(git status --short)" ]]; then
  WORKTREE="dirty"
else
  WORKTREE="clean"
fi
RUN_TIMESTAMP_UTC="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
METHOD_FINGERPRINT="$(
  sha256sum \
    "$ROOT/$DEFINITION_PATH" \
    "$ROOT/scripts/ci/instruction-audit-report.sh" \
    "$ROOT/crates/canic-tests/tests/instruction_audit.rs" \
    "$ROOT"/crates/canic-tests/tests/instruction_audit_support/*.rs \
    "$ROOT"/crates/canic-tests/tests/instruction_audit_support/estimates/*.rs \
  | sort -k2 \
  | sha256sum \
  | awk '{print $1}'
)"

RUN_TMP="$(mktemp -d)"
mkdir -p "$RUN_TMP/tmp"
cleanup() {
  rm -rf "$RUN_TMP"
}
trap cleanup EXIT

export CARGO_TARGET_DIR="${CANIC_AUDIT_CARGO_TARGET_DIR:-$RUN_TMP/target}"
export TMPDIR="$RUN_TMP/tmp"

if [[ "${ICP_ENVIRONMENT:-local}" == "ic" ]]; then
  echo "error: instruction audit must not run against the ic environment" >&2
  exit 2
fi

capture_source_status() {
  local report_rel="${REPORT_PATH#$ROOT/}"
  local artifacts_rel="${ARTIFACTS_DIR#$ROOT/}/"

  git status --porcelain=v1 --untracked-files=all \
    | awk -v report="$report_rel" -v artifacts="$artifacts_rel" '
        {
          path = substr($0, 4)
          if (path == report || index(path, artifacts) == 1) {
            next
          }
          print
        }
      '
}

capture_source_status >"$RUN_TMP/source-before.txt"

export CANIC_INSTRUCTION_AUDIT_REPORT_PATH="$REPORT_PATH"
export CANIC_INSTRUCTION_AUDIT_ARTIFACTS_DIR="$ARTIFACTS_DIR"
export CANIC_INSTRUCTION_AUDIT_BASELINE_REPORT="$BASELINE_REPORT"
export CANIC_INSTRUCTION_AUDIT_CODE_SNAPSHOT="$CODE_SNAPSHOT"
export CANIC_INSTRUCTION_AUDIT_BRANCH="$BRANCH"
export CANIC_INSTRUCTION_AUDIT_WORKTREE="$WORKTREE"
export CANIC_INSTRUCTION_AUDIT_TIMESTAMP_UTC="$RUN_TIMESTAMP_UTC"
export CANIC_INSTRUCTION_AUDIT_METHOD_ID="$METHOD_ID"
export CANIC_INSTRUCTION_AUDIT_METHOD_VERSION="$METHOD_VERSION"
export CANIC_INSTRUCTION_AUDIT_METHOD_FINGERPRINT="$METHOD_FINGERPRINT"
export CANIC_INSTRUCTION_AUDIT_ESTIMATE_EXECUTION_CYCLES="$ESTIMATE_EXECUTION_CYCLES"
export CANIC_INSTRUCTION_AUDIT_ESTIMATE_NODE_COUNT="$ESTIMATE_NODE_COUNT"
export CANIC_INSTRUCTION_AUDIT_CYCLES_PER_BILLION_INSTRUCTIONS="$CYCLES_PER_BILLION_INSTRUCTIONS"
export CANIC_INSTRUCTION_AUDIT_ESTIMATE_CANISTER_PRINCIPAL="$ESTIMATE_CANISTER_PRINCIPAL"
export CANIC_INSTRUCTION_AUDIT_ALLOW_STALE_SUBNET_CATALOG="$ALLOW_STALE_SUBNET_CATALOG"
export CANIC_INSTRUCTION_AUDIT_SUBNET_CATALOG_STALE_AFTER="$SUBNET_CATALOG_STALE_AFTER"

echo "Running instruction audit report into $REPORT_PATH"
cargo_status=0
cargo test --offline --locked -p canic-tests --test instruction_audit generate_instruction_footprint_report -- --ignored --nocapture || cargo_status=$?

printf -v command_display '%q ' bash scripts/ci/instruction-audit-report.sh "${ORIGINAL_ARGS[@]}"
command_display="${command_display% }"
baseline_identity="$BASELINE_REPORT"
if [[ "$BASELINE_REPORT" != "N/A" && -f "$BASELINE_REPORT" ]]; then
  baseline_identity="$BASELINE_REPORT@sha256:$(sha256sum "$BASELINE_REPORT" | awk '{print $1}')"
fi
completed_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
retained_hashes="$(
  {
    if [[ -f "$REPORT_PATH" ]]; then
      printf '%s\n' "${REPORT_PATH#$ROOT/}"
    fi
    find "${ARTIFACTS_DIR#$ROOT/}" -maxdepth 1 -type f ! -name evidence-manifest.yml -print
  } | sort | xargs -r sha256sum
)"
{
  printf 'command: "%s"\n' "$command_display"
  printf 'working_directory: "repository_root"\n'
  printf 'exit_code: %s\n' "$cargo_status"
  printf 'stdout_path: "not_retained"\n'
  printf 'stderr_path: "not_retained"\n'
  printf 'baseline_identity: "%s"\n' "$baseline_identity"
  printf 'method_identity: "%s/v%s@sha256:%s"\n' "$METHOD_ID" "$METHOD_VERSION" "$METHOD_FINGERPRINT"
  printf 'tool_versions:\n'
  printf '  cargo: "%s"\n' "$(cargo --version)"
  printf '  rustc: "%s"\n' "$(rustc --version)"
  printf 'timestamps:\n'
  printf '  started_at: "%s"\n' "$RUN_TIMESTAMP_UTC"
  printf '  completed_at: "%s"\n' "$completed_at"
  printf 'artifact_hashes:\n'
  if [[ -n "$retained_hashes" ]]; then
    sed 's/^/  /' <<<"$retained_hashes"
  else
    printf '  not_retained\n'
  fi
  printf 'retention_class: "primary_markdown_and_compact_supporting_evidence"\n'
  printf 'redactions_applied: "repository root normalized; no credentials, tokens, or private material retained"\n'
} >"$ARTIFACTS_DIR/evidence-manifest.yml"

capture_source_status >"$RUN_TMP/source-after.txt"
if ! diff -u "$RUN_TMP/source-before.txt" "$RUN_TMP/source-after.txt"; then
  echo "error: instruction audit mutated source outside its report paths" >&2
  exit 1
fi

if [[ "$cargo_status" -ne 0 ]]; then
  exit "$cargo_status"
fi

echo "Wrote $REPORT_PATH"
