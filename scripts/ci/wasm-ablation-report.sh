#!/usr/bin/env bash

set -euo pipefail

METHOD_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNNER_SOURCE="$METHOD_ROOT/scripts/ci/wasm-ablation-report.sh"
BUILD_HARNESS_SOURCE="$METHOD_ROOT/scripts/ci/wasm-ablation-build-artifact.rs"
EXPERIMENTS="$METHOD_ROOT/scripts/ci/wasm-ablation-experiments.tsv"
ARTIFACTS="$METHOD_ROOT/scripts/ci/wasm-ablation-artifacts.tsv"
FUNCTION_COUNTER_SOURCE="$METHOD_ROOT/scripts/ci/wasm-replica-function-count.rs"
FROZEN_IC_VALIDATOR_COMMIT="2f8dc21e2e5c37a4cae7f65d2a4230ac8f143e5a"
IC_REPLICA_MAX_DEFINED_FUNCTIONS=50000
IC_REPLICA_REQUIRED_FUNCTION_RESERVE=2500

usage() {
    cat <<'EOF'
Usage:
  scripts/ci/wasm-ablation-report.sh --check
  scripts/ci/wasm-ablation-report.sh --list
  scripts/ci/wasm-ablation-report.sh --experiment <id> --source <commit> \
    --product-root <clean-linked-worktree> --output-root <directory>
  scripts/ci/wasm-ablation-report.sh --smoke --experiment <id> \
    [--artifact <artifact-id>] --source <commit> \
    --product-root <clean-linked-worktree> --output-root <directory>
  scripts/ci/wasm-ablation-report.sh --qualify --experiment <id> \
    --source <commit> --product-root <clean-linked-worktree> \
    --output-root <directory>

The runner builds each selected artifact twice through canic-host's release
artifact authority, recreating one fixed Cargo target path before each clean
repetition. Its repository-owned function counter implements the exact local-
function quantity limited by the frozen IC replica validator source.

Smoke mode is development-only evidence. It builds the patched condition first
and then its baseline, selects only the first configured artifact unless an
exact artifact ID is supplied, performs one repetition and uses the repository
sccache wrapper when available. Smoke output is never retention-eligible and
does not make a determinism claim.

Qualification mode is also development-only. It accepts one `specified` patch,
builds its variant once across every selected artifact and validates the exact
artifacts and structured metrics. It emits no baseline or determinism claim and
is never retention-eligible.
EOF
}

fail() {
    echo "error: $*" >&2
    exit 2
}

file_hash() {
    sha256sum "$1" | awk '{print $1}'
}

tool_version() {
    "$1" --version 2>&1 | head -n 1
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

select_artifacts() {
    local selectors="$1"
    awk -F '\t' -v selectors="$selectors" '
        BEGIN {
            count = split(selectors, selected, ",")
            for (i = 1; i <= count; i++) {
                wanted[selected[i]] = 1
            }
        }
        NR > 1 && (wanted[$1] || wanted[$2]) { print }
    ' "$ARTIFACTS"
}

check_manifests() {
    local expected_experiment_header
    local expected_artifact_header
    local sequence
    local experiment
    local state
    local switch_kind
    local switch_value
    local switch_sha256
    local selectors
    local immediate_baseline
    local instruction_evidence
    local source_owners
    local artifact_id
    local group
    local config_path
    local canister
    local owner
    local selector
    local experiment_ids

    expected_experiment_header=$'sequence\texperiment\tstate\tswitch_kind\tswitch_value\tswitch_sha256\tartifact_selectors\timmediate_baseline\tinstruction_evidence\tsource_owners'
    expected_artifact_header=$'artifact_id\tgroup\tconfig_path\tcanister'
    [[ "$(head -n 1 "$EXPERIMENTS")" == "$expected_experiment_header" ]] ||
        fail "unexpected experiment manifest header"
    [[ "$(head -n 1 "$ARTIFACTS")" == "$expected_artifact_header" ]] ||
        fail "unexpected artifact manifest header"

    awk -F '\t' '
        NR == 1 { next }
        {
            expected = sprintf("%02d", NR - 1)
            if ($1 != expected || seen[$2]++ || NF != 10) exit 1
        }
        END { if (NR != 19) exit 1 }
    ' "$EXPERIMENTS" || fail "experiment manifest must contain ordered unique rows 01 through 18"
    experiment_ids="$(awk -F '\t' 'NR > 1 { print $2 }' "$EXPERIMENTS")"

    awk -F '\t' '
        NR == 1 { next }
        { if (NF != 4 || seen[$1]++) exit 1 }
        END { if (NR != 16) exit 1 }
    ' "$ARTIFACTS" || fail "artifact manifest must contain fifteen unique artifacts"

    while IFS=$'\t' read -r artifact_id group config_path canister; do
        [[ "$artifact_id" == "artifact_id" ]] && continue
        [[ "$artifact_id" =~ ^[a-z0-9_]+$ ]] || fail "invalid artifact id: $artifact_id"
        [[ "$group" == "canonical" || "$group" == "fixture" ]] ||
            fail "invalid artifact group for $artifact_id: $group"
        [[ "$canister" =~ ^[a-z0-9_]+$ ]] || fail "invalid canister name: $canister"
        [[ "$config_path" != /* && "$config_path" != ../* ]] ||
            fail "artifact config must remain repository-relative: $config_path"
        [[ -f "$METHOD_ROOT/$config_path" ]] || fail "missing artifact config: $config_path"
    done <"$ARTIFACTS"

    while IFS=$'\t' read -r sequence experiment state switch_kind switch_value switch_sha256 \
        selectors immediate_baseline instruction_evidence source_owners; do
        [[ "$sequence" == "sequence" ]] && continue
        [[ "$experiment" =~ ^b1-[0-9][0-9]-[a-z0-9-]+$ ]] ||
            fail "invalid experiment id: $experiment"
        [[ "$state" == "ready" || "$state" == "specified" || "$state" == "planned" ]] ||
            fail "invalid state for $experiment: $state"
        case "$switch_kind" in
            none|patch|env_matrix|cross_commit) ;;
            *) fail "invalid switch kind for $experiment: $switch_kind" ;;
        esac
        [[ -n "$instruction_evidence" ]] || fail "missing instruction disposition for $experiment"
        [[ -n "$(select_artifacts "$selectors")" ]] ||
            fail "artifact selectors resolve to an empty set for $experiment"
        IFS=',' read -r -a selected_values <<<"$selectors"
        for selector in "${selected_values[@]}"; do
            awk -F '\t' -v selector="$selector" \
                'NR > 1 && ($1 == selector || $2 == selector) { found = 1 } END { exit !found }' \
                "$ARTIFACTS" || fail "unknown artifact selector for $experiment: $selector"
        done
        IFS=';' read -r -a owner_values <<<"$source_owners"
        for owner in "${owner_values[@]}"; do
            [[ "$owner" != /* && "$owner" != ../* ]] ||
                fail "source owner must remain repository-relative: $owner"
            [[ -e "$METHOD_ROOT/$owner" ]] || fail "missing source owner for $experiment: $owner"
        done
        if [[ "$immediate_baseline" != "-" && "$immediate_baseline" != v* ]]; then
            case $'\n'"$experiment_ids"$'\n' in
                *$'\n'"$immediate_baseline"$'\n'*) ;;
                *) fail "unknown immediate baseline for $experiment: $immediate_baseline" ;;
            esac
        fi
        if [[ ( "$state" == "ready" || "$state" == "specified" ) && "$switch_kind" == "patch" ]]; then
            [[ -f "$METHOD_ROOT/$switch_value" ]] ||
                fail "$state patch experiment lacks its switch: $switch_value"
            [[ "$switch_sha256" =~ ^[0-9a-f]{64}$ ]] ||
                fail "$state patch experiment lacks an exact SHA-256: $experiment"
            [[ "$(file_hash "$METHOD_ROOT/$switch_value")" == "$switch_sha256" ]] ||
                fail "$state patch experiment SHA-256 does not match: $experiment"
            if [[ "$state" == "specified" ]]; then
                git -C "$METHOD_ROOT" apply --check "$METHOD_ROOT/$switch_value" ||
                    fail "specified patch no longer applies to the current source: $switch_value"
            fi
        elif [[ "$switch_sha256" != "-" ]]; then
            fail "non-runnable or non-patch experiment has a switch SHA-256: $experiment"
        fi
        if [[ "$switch_kind" == "env_matrix" ]]; then
            [[ "$switch_value" == "CANIC_GENERIC_COHORT_WIDTH=1..5" ]] ||
                fail "unexpected environment matrix for $experiment"
        fi
    done <"$EXPERIMENTS"

    if rg -i 'toko' "$EXPERIMENTS" "$ARTIFACTS" >/dev/null; then
        fail "consumer-specific source or artifact entered the Canic ablation manifests"
    fi
    [[ -f "$FUNCTION_COUNTER_SOURCE" ]] || fail "missing replica function counter source"
    [[ -f "$BUILD_HARNESS_SOURCE" ]] || fail "missing structured artifact build harness source"
}

ACTION=""
EXPERIMENT=""
EXPECTED_SOURCE=""
PRODUCT_ROOT_INPUT="${WASM_ABLATION_PRODUCT_ROOT:-}"
OUTPUT_ROOT_INPUT="${WASM_ABLATION_OUTPUT_ROOT:-}"
RUN_MODE="retained"
ARTIFACT_OVERRIDE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --check|--list)
            [[ -z "$ACTION" ]] || fail "choose exactly one action"
            ACTION="${1#--}"
            shift
            ;;
        --experiment)
            [[ -z "$ACTION" ]] || fail "choose exactly one action"
            ACTION="run"
            EXPERIMENT="${2:-}"
            shift 2
            ;;
        --source)
            EXPECTED_SOURCE="${2:-}"
            shift 2
            ;;
        --product-root)
            PRODUCT_ROOT_INPUT="${2:-}"
            shift 2
            ;;
        --output-root)
            OUTPUT_ROOT_INPUT="${2:-}"
            shift 2
            ;;
        --smoke|--qualify)
            [[ "$RUN_MODE" == "retained" ]] || fail "choose at most one development run mode"
            RUN_MODE="${1#--}"
            shift
            ;;
        --artifact)
            ARTIFACT_OVERRIDE="${2:-}"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            fail "unknown argument: $1"
            ;;
    esac
done

[[ -n "$ACTION" ]] || {
    usage >&2
    exit 2
}

check_manifests

if [[ "$RUN_MODE" != "retained" && "$ACTION" != "run" ]]; then
    fail "--$RUN_MODE is valid only with --experiment"
fi
if [[ -n "$ARTIFACT_OVERRIDE" && "$RUN_MODE" != "smoke" ]]; then
    fail "--artifact is valid only with --smoke"
fi

if [[ "$ACTION" == "check" ]]; then
    echo "Wasm ablation manifests passed"
    exit 0
fi

if [[ "$ACTION" == "list" ]]; then
    awk -F '\t' 'BEGIN { OFS="\t" } NR == 1 { print $1,$2,$3,$4,$7; next } { print $1,$2,$3,$4,$7 }' "$EXPERIMENTS"
    exit 0
fi

[[ -n "$EXPERIMENT" && -n "$EXPECTED_SOURCE" && -n "$PRODUCT_ROOT_INPUT" &&
    -n "$OUTPUT_ROOT_INPUT" ]] || fail "a run requires experiment, source, product root and output root"

EXPERIMENT_ROW="$(awk -F '\t' -v experiment="$EXPERIMENT" 'NR > 1 && $2 == experiment { print; exit }' "$EXPERIMENTS")"
[[ -n "$EXPERIMENT_ROW" ]] || fail "unknown experiment: $EXPERIMENT"
IFS=$'\t' read -r SEQUENCE _ STATE SWITCH_KIND SWITCH_VALUE SWITCH_SHA256 \
    ARTIFACT_SELECTORS IMMEDIATE_BASELINE INSTRUCTION_EVIDENCE SOURCE_OWNERS <<<"$EXPERIMENT_ROW"
if [[ "$RUN_MODE" == "qualify" ]]; then
    [[ "$STATE" == "specified" && "$SWITCH_KIND" == "patch" ]] ||
        fail "qualification requires one specified patch experiment: $EXPERIMENT"
else
    [[ "$STATE" == "ready" ]] ||
        fail "experiment is not runnable until its one-switch input exists: $EXPERIMENT"
fi
[[ "$SWITCH_KIND" != "cross_commit" ]] || fail "cross-commit comparison requires its separately frozen compatible pair"

selected_run_artifacts() {
    local selected_artifacts
    local selected_override

    selected_artifacts="$(select_artifacts "$ARTIFACT_SELECTORS")"
    [[ -n "$selected_artifacts" ]] || fail "no artifacts selected for $EXPERIMENT"
    if [[ "$RUN_MODE" != "smoke" ]]; then
        printf '%s\n' "$selected_artifacts"
        return
    fi
    if [[ -z "$ARTIFACT_OVERRIDE" ]]; then
        printf '%s\n' "$selected_artifacts" | awk 'NR == 1 { print; exit }'
        return
    fi
    [[ "$ARTIFACT_OVERRIDE" =~ ^[a-z0-9_]+$ ]] ||
        fail "invalid smoke artifact ID: $ARTIFACT_OVERRIDE"
    selected_override="$(printf '%s\n' "$selected_artifacts" |
        awk -F '\t' -v artifact="$ARTIFACT_OVERRIDE" '$1 == artifact { print; exit }')"
    [[ -n "$selected_override" ]] ||
        fail "smoke artifact $ARTIFACT_OVERRIDE is not selected by $EXPERIMENT"
    printf '%s\n' "$selected_override"
}

require_command cargo
require_command cmp
require_command didc
require_command git
require_command gzip
require_command ic-wasm
require_command jq
require_command rg
require_command rustc
require_command sha256sum
require_command stat
require_command wasm-objdump
require_command wasm-validate

PRODUCT_ROOT="$(cd "$PRODUCT_ROOT_INPUT" && pwd)"
mkdir -p "$OUTPUT_ROOT_INPUT"
OUTPUT_ROOT="$(cd "$OUTPUT_ROOT_INPUT" && pwd)"
case "$OUTPUT_ROOT/" in
    "$PRODUCT_ROOT/"*) fail "output root must not be inside the disposable product worktree" ;;
esac

git -C "$PRODUCT_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1 ||
    fail "product root is not a Git worktree: $PRODUCT_ROOT"
PRODUCT_GIT_DIR="$(git -C "$PRODUCT_ROOT" rev-parse --absolute-git-dir)"
PRODUCT_COMMON_DIR="$(git -C "$PRODUCT_ROOT" rev-parse --path-format=absolute --git-common-dir)"
[[ "$PRODUCT_GIT_DIR" != "$PRODUCT_COMMON_DIR" ]] ||
    fail "product root must be a disposable linked Git worktree"
SOURCE_STATUS_BEFORE="$(git -C "$PRODUCT_ROOT" status --porcelain=v1 --untracked-files=all)"
[[ -z "$SOURCE_STATUS_BEFORE" ]] || fail "product worktree must be clean before the ablation run"
SOURCE_COMMIT="$(git -C "$PRODUCT_ROOT" rev-parse 'HEAD^{commit}')"
EXPECTED_COMMIT="$(git -C "$PRODUCT_ROOT" rev-parse "$EXPECTED_SOURCE^{commit}")"
[[ "$SOURCE_COMMIT" == "$EXPECTED_COMMIT" ]] ||
    fail "product worktree is at $SOURCE_COMMIT, expected $EXPECTED_COMMIT"
BASE_SOURCE_TREE="$(git -C "$PRODUCT_ROOT" rev-parse 'HEAD^{tree}')"
BASE_CARGO_LOCK_SHA256="$(file_hash "$PRODUCT_ROOT/Cargo.lock")"

RUN_STEM="$EXPERIMENT-${SOURCE_COMMIT:0:12}"
if [[ "$RUN_MODE" != "retained" ]]; then
    RUN_STEM="$RUN_STEM-$RUN_MODE"
fi
RUN_ROOT="$OUTPUT_ROOT/$RUN_STEM"
[[ ! -e "$RUN_ROOT" ]] || fail "output already exists: $RUN_ROOT"
mkdir -p "$RUN_ROOT/artifacts" "$RUN_ROOT/logs" "$RUN_ROOT/analysis" "$RUN_ROOT/method"
cp "$EXPERIMENTS" "$RUN_ROOT/experiments.tsv"
cp "$ARTIFACTS" "$RUN_ROOT/artifacts.tsv"
cp "$RUNNER_SOURCE" "$RUN_ROOT/method/wasm-ablation-report.sh"
cp "$BUILD_HARNESS_SOURCE" "$RUN_ROOT/method/wasm-ablation-build-artifact.rs"
cp "$FUNCTION_COUNTER_SOURCE" "$RUN_ROOT/method/wasm-replica-function-count.rs"
RUNNER_SOURCE_SHA256="$(file_hash "$RUN_ROOT/method/wasm-ablation-report.sh")"
BUILD_HARNESS_SOURCE_SHA256="$(file_hash "$RUN_ROOT/method/wasm-ablation-build-artifact.rs")"

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/canic-wasm-ablation.XXXXXX")"
BUILD_TARGET_DIR="$SCRATCH/cargo-target"
BUILD_HARNESS_ROOT="$SCRATCH/build-harness"
BUILD_HARNESS_MANIFEST="$BUILD_HARNESS_ROOT/Cargo.toml"
BUILD_HARNESS_PRODUCT="$SCRATCH/product"
PATCH_APPLIED="false"
PATCH_PATH=""
cleanup() {
    rm -rf "$PRODUCT_ROOT/.icp"
    if [[ "$PATCH_APPLIED" == "true" ]]; then
        git -C "$PRODUCT_ROOT" apply --reverse "$PATCH_PATH" ||
            echo "warning: failed to reverse the measurement patch in $PRODUCT_ROOT" >&2
    fi
    rm -rf "$SCRATCH"
}
trap cleanup EXIT

mkdir -p "$BUILD_HARNESS_ROOT/src"
ln -s "$PRODUCT_ROOT" "$BUILD_HARNESS_PRODUCT"
cp "$RUN_ROOT/method/wasm-ablation-build-artifact.rs" "$BUILD_HARNESS_ROOT/src/main.rs"
cp "$PRODUCT_ROOT/Cargo.lock" "$BUILD_HARNESS_ROOT/Cargo.lock"
cat >"$BUILD_HARNESS_MANIFEST" <<'EOF'
[package]
name = "canic-wasm-ablation-build-artifact"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
canic-core = { path = "../product/crates/canic-core" }
canic-host = { path = "../product/crates/canic-host" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = "symbols"
debug = false
panic = "abort"
overflow-checks = false
incremental = false

[profile.fast]
inherits = "release"
lto = false
codegen-units = 16
incremental = false

[workspace]
EOF

# The method harness is a separate Cargo root, so seed its dependency graph from
# the immutable product lock and resolve that graph once without network access.
# All measured builds below then require this retained harness lock exactly.
CARGO_NET_OFFLINE=true cargo metadata --offline --format-version 1 \
    --manifest-path "$BUILD_HARNESS_MANIFEST" >/dev/null
BUILD_HARNESS_CARGO_LOCK="$RUN_ROOT/method/wasm-ablation-build-artifact.Cargo.lock"
cp "$BUILD_HARNESS_ROOT/Cargo.lock" "$BUILD_HARNESS_CARGO_LOCK"
BUILD_HARNESS_CARGO_LOCK_SHA256="$(file_hash "$BUILD_HARNESS_CARGO_LOCK")"

FUNCTION_COUNTER_INPUT="$SCRATCH/wasm-replica-function-count.rs"
FUNCTION_COUNTER="$SCRATCH/wasm-replica-function-count"
cp "$RUN_ROOT/method/wasm-replica-function-count.rs" "$FUNCTION_COUNTER_INPUT"
FUNCTION_COUNTER_SOURCE_SHA256="$(file_hash "$FUNCTION_COUNTER_INPUT")"
rustc --edition 2024 -D warnings -C debuginfo=0 -C strip=symbols \
    --remap-path-prefix "$SCRATCH=canic-b1-tool" \
    "$FUNCTION_COUNTER_INPUT" -o "$FUNCTION_COUNTER"
FUNCTION_COUNTER_IDENTITY="$("$FUNCTION_COUNTER" --identity)"
EXPECTED_FUNCTION_COUNTER_IDENTITY=$'canic-b1-replica-function-count/v1\tic_source_commit='"$FROZEN_IC_VALIDATOR_COMMIT"$'\tquantity=local-defined-functions\tlimit='"$IC_REPLICA_MAX_DEFINED_FUNCTIONS"
[[ "$FUNCTION_COUNTER_IDENTITY" == "$EXPECTED_FUNCTION_COUNTER_IDENTITY" ]] ||
    fail "replica function counter identity does not match the frozen IC source contract"
FUNCTION_COUNTER_EXECUTABLE_SHA256="$(file_hash "$FUNCTION_COUNTER")"

METRICS="$RUN_ROOT/artifact-metrics.tsv"
DETERMINISM="$RUN_ROOT/determinism.tsv"
printf 'experiment\tcondition\trepetition\tartifact_id\tcanister\twasm_bytes\tgzip_bytes\tcode_section_bytes\tdata_section_bytes\tic_wasm_total_functions\treplica_limited_defined_functions\toptimizer_defined_functions\ttable_minimum\telement_entries\twasm_export_entries\tic_wasm_exported_methods\tcandid_bytes\tcandid_service_methods\twasm_sha256\tgzip_sha256\tcandid_sha256\toptimizer_before_raw\toptimizer_before_gzip\toptimizer_before_code\toptimizer_before_data\toptimizer_before_functions\n' >"$METRICS"
printf 'experiment\tcondition\tartifact_id\twasm\tgzip\tcandid\tmetrics\tresult\n' >"$DETERMINISM"

capture_artifact() {
    local condition="$1"
    local repetition="$2"
    local artifact_id="$3"
    local canister="$4"
    local transform_metrics_path="$5"
    local artifact_root="$PRODUCT_ROOT/.icp/local/canisters/$canister"
    local wasm_path="$artifact_root/$canister.wasm"
    local gzip_path="$artifact_root/$canister.wasm.gz"
    local candid_path="$artifact_root/$canister.did"
    local output_dir="$RUN_ROOT/artifacts/$condition-$repetition/$artifact_id"
    local analysis_prefix="$RUN_ROOT/analysis/$condition-$repetition-$artifact_id"
    local optimizer_metrics
    local ic_wasm_functions
    local replica_limited_defined_functions
    local ic_wasm_exported_methods
    local table_minimum
    local element_entries
    local wasm_export_entries
    local candid_service_methods
    local decoded_hash

    [[ -s "$wasm_path" && -s "$gzip_path" && -s "$candid_path" ]] ||
        fail "builder did not produce the complete artifact for $artifact_id"
    gzip -t "$gzip_path"
    decoded_hash="$(gzip -cd "$gzip_path" | sha256sum | awk '{print $1}')"
    [[ "$decoded_hash" == "$(file_hash "$wasm_path")" ]] ||
        fail "gzip does not decode to the canonical Wasm for $artifact_id"
    wasm-validate "$wasm_path"
    didc check "$candid_path" >/dev/null
    ic-wasm "$wasm_path" info >"$analysis_prefix.ic-wasm-info.txt"
    wasm-objdump -x "$wasm_path" >"$analysis_prefix.objdump.txt"

    optimizer_metrics="$(jq -er --arg role "$canister" '
        if .schema_version != 1 or .role != $role
        then error("unexpected transform metrics identity")
        else
            [.transforms[] |
                select(.transform == "optimize" and .outcome == "applied")] as $records |
            if ($records | length) != 1 or $records[0].metrics == null
            then error("expected one applied optimizer metrics record")
            else
                $records[0].metrics |
                [
                    .before.raw_bytes,
                    .after.raw_bytes,
                    .before.gzip_bytes,
                    .after.gzip_bytes,
                    .before.code_section_bytes,
                    .after.code_section_bytes,
                    .before.data_section_bytes,
                    .after.data_section_bytes,
                    .before.defined_functions,
                    .after.defined_functions
                ] as $values |
                if all($values[]; type == "number")
                then $values | @tsv
                else error("optimizer metrics must be numeric")
                end
            end
        end
    ' "$transform_metrics_path")" || fail "invalid optimizer metrics for $artifact_id"
    local before_raw
    local after_raw
    local before_gzip
    local after_gzip
    local before_code
    local after_code
    local before_data
    local after_data
    local before_functions
    local after_functions
    IFS=$'\t' read -r before_raw after_raw before_gzip after_gzip before_code after_code \
        before_data after_data before_functions after_functions <<<"$optimizer_metrics"
    [[ "$after_raw" == "$(stat -c%s "$wasm_path")" &&
        "$after_gzip" == "$(stat -c%s "$gzip_path")" ]] ||
        fail "optimizer and canonical artifact sizes disagree for $artifact_id"

    ic_wasm_functions="$(sed -n 's/^Number of functions: //p' "$analysis_prefix.ic-wasm-info.txt" | head -n 1)"
    [[ "$ic_wasm_functions" =~ ^[0-9]+$ ]] || fail "missing ic-wasm function count for $artifact_id"
    replica_limited_defined_functions="$("$FUNCTION_COUNTER" "$wasm_path")"
    [[ "$replica_limited_defined_functions" =~ ^[0-9]+$ ]] ||
        fail "replica function counter did not return one integer for $artifact_id"
    (( replica_limited_defined_functions <= IC_REPLICA_MAX_DEFINED_FUNCTIONS )) ||
        fail "$artifact_id has $replica_limited_defined_functions defined functions, exceeding the frozen IC limit of $IC_REPLICA_MAX_DEFINED_FUNCTIONS"
    [[ "$replica_limited_defined_functions" == "$after_functions" ]] ||
        fail "replica-limited and optimizer-defined function counts disagree for $artifact_id"
    ic_wasm_exported_methods="$(awk '
        /^Exported methods: \[/ { active=1; next }
        active && /^\]/ { active=0 }
        active && /"/ { count++ }
        END { print count + 0 }
    ' "$analysis_prefix.ic-wasm-info.txt")"
    table_minimum="$(awk '
        /^Table\[/ { active=1; next }
        /^[A-Za-z]+\[/ { active=0 }
        active && /initial=/ { print }
    ' "$analysis_prefix.objdump.txt" | sed -n 's/.* initial=\([0-9][0-9]*\).*/\1/p' | awk '{ total += $1 } END { print total + 0 }')"
    element_entries="$(awk '
        /^Elem\[/ { active=1; next }
        /^[A-Za-z]+\[/ { active=0 }
        active && /count=/ { print }
    ' "$analysis_prefix.objdump.txt" | sed -n 's/.* count=\([0-9][0-9]*\).*/\1/p' | awk '{ total += $1 } END { print total + 0 }')"
    wasm_export_entries="$(sed -n 's/^Export\[\([0-9][0-9]*\)\]:$/\1/p' "$analysis_prefix.objdump.txt" | head -n 1)"
    [[ "$wasm_export_entries" =~ ^[0-9]+$ ]] || fail "missing Wasm export count for $artifact_id"
    candid_service_methods="$(awk '
        /^service[[:space:]]*:/ { active=1; next }
        active && /^}/ { active=0 }
        active && /^[[:space:]]+("[^"]+"|[A-Za-z_][A-Za-z0-9_]*)[[:space:]]*:/ { count++ }
        END { print count + 0 }
    ' "$candid_path")"

    mkdir -p "$output_dir"
    cp "$wasm_path" "$output_dir/$artifact_id.wasm"
    cp "$gzip_path" "$output_dir/$artifact_id.wasm.gz"
    cp "$candid_path" "$output_dir/$artifact_id.did"

    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$EXPERIMENT" "$condition" "$repetition" "$artifact_id" "$canister" \
        "$after_raw" "$after_gzip" "$after_code" "$after_data" \
        "$ic_wasm_functions" "$replica_limited_defined_functions" "$after_functions" \
        "$table_minimum" "$element_entries" "$wasm_export_entries" \
        "$ic_wasm_exported_methods" "$(stat -c%s "$candid_path")" \
        "$candid_service_methods" "$(file_hash "$wasm_path")" \
        "$(file_hash "$gzip_path")" "$(file_hash "$candid_path")" \
        "$before_raw" "$before_gzip" "$before_code" "$before_data" \
        "$before_functions" >>"$METRICS"
}

build_condition() {
    local condition="$1"
    local environment_name="$2"
    local environment_value="$3"
    local repetition
    local artifact_id
    local group
    local config_path
    local canister
    local target_dir
    local log_path
    local transform_metrics_path
    local first_dir
    local second_dir
    local first_metrics
    local second_metrics
    local extension
    local environment_args=("RUSTC_WRAPPER=")
    local selected_artifacts
    local artifact_count
    local artifact_index
    local build_started
    local repetitions=(a b)

    selected_artifacts="$(selected_run_artifacts)"
    artifact_count="$(printf '%s\n' "$selected_artifacts" | awk 'NF { count++ } END { print count + 0 }')"
    if [[ "$RUN_MODE" != "retained" ]]; then
        repetitions=(a)
        if [[ -x "$METHOD_ROOT/scripts/ci/run-sccache.sh" ]] &&
            command -v sccache >/dev/null 2>&1; then
            if "$METHOD_ROOT/scripts/ci/run-sccache.sh" "$(command -v rustc)" -vV \
                >/dev/null 2>&1; then
                environment_args=("RUSTC_WRAPPER=$METHOD_ROOT/scripts/ci/run-sccache.sh")
            else
                echo "warning: sccache is installed but unusable; running the development build without it" >&2
            fi
        fi
    fi
    if [[ -n "$environment_name" ]]; then
        [[ "$environment_name" =~ ^[A-Z][A-Z0-9_]*$ ]] || fail "invalid measurement environment name"
        environment_args+=("$environment_name=$environment_value")
    fi

    for repetition in "${repetitions[@]}"; do
        target_dir="$BUILD_TARGET_DIR"
        if [[ "$RUN_MODE" == "retained" ]]; then
            rm -rf "$target_dir"
        fi
        mkdir -p "$target_dir"
        artifact_index=0
        while IFS=$'\t' read -r artifact_id group config_path canister; do
            artifact_index=$((artifact_index + 1))
            rm -rf "$PRODUCT_ROOT/.icp"
            log_path="$RUN_ROOT/logs/$condition-$repetition-$artifact_id.log"
            transform_metrics_path="$RUN_ROOT/analysis/$condition-$repetition-$artifact_id.transform-metrics.json"
            build_started="$SECONDS"
            echo "building $EXPERIMENT $condition-$repetition $artifact_id ($artifact_index/$artifact_count)"
            if ! (
                cd "$PRODUCT_ROOT"
                env ICP_ENVIRONMENT=local CARGO_NET_OFFLINE=true CARGO_INCREMENTAL=0 \
                    CARGO_TARGET_DIR="$target_dir" "${environment_args[@]}" \
                    cargo run --offline --locked -q --profile fast \
                        --manifest-path "$BUILD_HARNESS_MANIFEST" -- \
                        "$canister" release "$PRODUCT_ROOT" "$PRODUCT_ROOT" \
                        "$PRODUCT_ROOT/$config_path" "$transform_metrics_path"
            ) >"$log_path" 2>&1; then
                tail -n 80 "$log_path" >&2
                fail "release build failed for $artifact_id"
            fi
            capture_artifact "$condition" "$repetition" "$artifact_id" "$canister" \
                "$transform_metrics_path"
            echo "built $EXPERIMENT $condition-$repetition $artifact_id in $((SECONDS - build_started))s"
        done <<<"$selected_artifacts"
    done

    if [[ "$RUN_MODE" != "retained" ]]; then
        return
    fi

    while IFS=$'\t' read -r artifact_id group config_path canister; do
        first_dir="$RUN_ROOT/artifacts/$condition-a/$artifact_id"
        second_dir="$RUN_ROOT/artifacts/$condition-b/$artifact_id"
        for extension in wasm wasm.gz did; do
            cmp -s "$first_dir/$artifact_id.$extension" "$second_dir/$artifact_id.$extension" ||
                fail "nondeterministic $extension for $condition $artifact_id"
        done
        first_metrics="$(awk -F '\t' -v condition="$condition" -v artifact="$artifact_id" \
            '$2 == condition && $3 == "a" && $4 == artifact { for (i=6; i<=NF; i++) printf "%s%s", $i, (i == NF ? ORS : OFS) }' OFS='\t' "$METRICS")"
        second_metrics="$(awk -F '\t' -v condition="$condition" -v artifact="$artifact_id" \
            '$2 == condition && $3 == "b" && $4 == artifact { for (i=6; i<=NF; i++) printf "%s%s", $i, (i == NF ? ORS : OFS) }' OFS='\t' "$METRICS")"
        [[ -n "$first_metrics" && "$first_metrics" == "$second_metrics" ]] ||
            fail "nondeterministic metrics for $condition $artifact_id"
        printf '%s\t%s\t%s\tPASS\tPASS\tPASS\tPASS\tPASS\n' \
            "$EXPERIMENT" "$condition" "$artifact_id" >>"$DETERMINISM"
    done <<<"$selected_artifacts"
}

PATCH_SHA256="NA"
PATCH_DIFF_SHA256="NA"
PATCH_EXPECTED_PATHS=""
case "$SWITCH_KIND" in
    none)
        build_condition baseline "" ""
        ;;
    env_matrix)
        for width in 1 2 3 4 5; do
            build_condition "width-$width" CANIC_GENERIC_COHORT_WIDTH "$width"
        done
        ;;
    patch)
        PATCH_PATH="$METHOD_ROOT/$SWITCH_VALUE"
        [[ -f "$PATCH_PATH" ]] || fail "missing measurement patch: $SWITCH_VALUE"
        PATCH_SHA256="$(file_hash "$PATCH_PATH")"
        [[ "$PATCH_SHA256" == "$SWITCH_SHA256" ]] ||
            fail "measurement patch SHA-256 does not match its experiment record"
        PATCH_EXPECTED_PATHS="$(git -C "$PRODUCT_ROOT" apply --numstat "$PATCH_PATH" | cut -f3 | sort -u)"
        [[ -n "$PATCH_EXPECTED_PATHS" ]] || fail "measurement patch has no paths: $SWITCH_VALUE"
        if [[ "$RUN_MODE" == "smoke" ]]; then
            git -C "$PRODUCT_ROOT" apply --check "$PATCH_PATH"
            git -C "$PRODUCT_ROOT" apply "$PATCH_PATH"
            PATCH_APPLIED="true"
            PATCH_DIFF_SHA256="$(git -C "$PRODUCT_ROOT" diff --binary | sha256sum | awk '{print $1}')"
            build_condition variant "" ""
            git -C "$PRODUCT_ROOT" apply --reverse "$PATCH_PATH"
            PATCH_APPLIED="false"
            build_condition baseline "" ""
            git -C "$PRODUCT_ROOT" apply "$PATCH_PATH"
            PATCH_APPLIED="true"
        elif [[ "$RUN_MODE" == "qualify" ]]; then
            git -C "$PRODUCT_ROOT" apply --check "$PATCH_PATH"
            git -C "$PRODUCT_ROOT" apply "$PATCH_PATH"
            PATCH_APPLIED="true"
            PATCH_DIFF_SHA256="$(git -C "$PRODUCT_ROOT" diff --binary | sha256sum | awk '{print $1}')"
            build_condition variant "" ""
        else
            build_condition baseline "" ""
            git -C "$PRODUCT_ROOT" apply --check "$PATCH_PATH"
            git -C "$PRODUCT_ROOT" apply "$PATCH_PATH"
            PATCH_APPLIED="true"
            build_condition variant "" ""
            PATCH_DIFF_SHA256="$(git -C "$PRODUCT_ROOT" diff --binary | sha256sum | awk '{print $1}')"
        fi
        ;;
    *)
        fail "unsupported runnable switch kind: $SWITCH_KIND"
        ;;
esac

SOURCE_STATUS_DURING="$(git -C "$PRODUCT_ROOT" status --porcelain=v1 --untracked-files=all | awk '$2 !~ /^\.icp\// { print }')"
if [[ "$SWITCH_KIND" == "patch" ]]; then
    [[ -n "$SOURCE_STATUS_DURING" ]] || fail "measurement patch produced no source difference"
    PATCH_ACTUAL_PATHS="$(printf '%s\n' "$SOURCE_STATUS_DURING" | awk '{ print $2 }' | sort -u)"
    [[ "$PATCH_ACTUAL_PATHS" == "$PATCH_EXPECTED_PATHS" ]] ||
        fail "measurement patch or build changed an unexpected source path"
else
    [[ -z "$SOURCE_STATUS_DURING" ]] || fail "ablation build mutated tracked or unexpected source"
fi

{
    [[ "$(file_hash "$RUNNER_SOURCE")" == "$RUNNER_SOURCE_SHA256" ]] ||
        fail "Wasm ablation runner source changed during the ablation run"
    [[ "$(file_hash "$BUILD_HARNESS_SOURCE")" == "$BUILD_HARNESS_SOURCE_SHA256" ]] ||
        fail "structured artifact build harness source changed during the ablation run"
    [[ "$(file_hash "$BUILD_HARNESS_ROOT/Cargo.lock")" == "$BUILD_HARNESS_CARGO_LOCK_SHA256" ]] ||
        fail "structured artifact build harness lock changed during the ablation run"
    [[ "$(file_hash "$FUNCTION_COUNTER_SOURCE")" == "$FUNCTION_COUNTER_SOURCE_SHA256" ]] ||
        fail "replica function counter source changed during the ablation run"
    printf 'field\tvalue\n'
    printf 'experiment\t%s\n' "$EXPERIMENT"
    printf 'run_mode\t%s\n' "$RUN_MODE"
    printf 'sequence\t%s\n' "$SEQUENCE"
    printf 'source_commit\t%s\n' "$SOURCE_COMMIT"
    printf 'source_tree\t%s\n' "$BASE_SOURCE_TREE"
    printf 'baseline_cargo_lock_sha256\t%s\n' "$BASE_CARGO_LOCK_SHA256"
    printf 'measured_cargo_lock_sha256\t%s\n' "$(file_hash "$PRODUCT_ROOT/Cargo.lock")"
    printf 'switch_kind\t%s\n' "$SWITCH_KIND"
    printf 'switch_value\t%s\n' "$SWITCH_VALUE"
    printf 'switch_sha256\t%s\n' "$PATCH_SHA256"
    printf 'switch_diff_sha256\t%s\n' "$PATCH_DIFF_SHA256"
    printf 'immediate_baseline\t%s\n' "$IMMEDIATE_BASELINE"
    printf 'artifact_selectors\t%s\n' "$ARTIFACT_SELECTORS"
    printf 'measured_artifacts\t%s\n' "$(selected_run_artifacts | cut -f1 | paste -sd, -)"
    printf 'instruction_evidence\t%s\n' "$INSTRUCTION_EVIDENCE"
    printf 'source_owners\t%s\n' "$SOURCE_OWNERS"
    printf 'runner_source_sha256\t%s\n' "$RUNNER_SOURCE_SHA256"
    printf 'build_harness_source_sha256\t%s\n' "$BUILD_HARNESS_SOURCE_SHA256"
    printf 'build_harness_cargo_lock_sha256\t%s\n' "$BUILD_HARNESS_CARGO_LOCK_SHA256"
    printf 'replica_function_counter_identity\t%s\n' "$FUNCTION_COUNTER_IDENTITY"
    printf 'replica_function_counter_source_sha256\t%s\n' "$FUNCTION_COUNTER_SOURCE_SHA256"
    printf 'replica_function_counter_executable_sha256\t%s\n' "$FUNCTION_COUNTER_EXECUTABLE_SHA256"
    printf 'frozen_ic_validator_commit\t%s\n' "$FROZEN_IC_VALIDATOR_COMMIT"
    printf 'ic_replica_max_defined_functions\t%s\n' "$IC_REPLICA_MAX_DEFINED_FUNCTIONS"
    printf 'ic_replica_required_function_reserve\t%s\n' "$IC_REPLICA_REQUIRED_FUNCTION_RESERVE"
    if [[ "$RUN_MODE" == "retained" ]]; then
        printf 'retention_eligible\tyes\n'
        printf 'determinism_repetitions\t2\n'
        printf 'determinism_claim\tpass\n'
    else
        printf 'retention_eligible\tno\n'
        printf 'determinism_repetitions\t1\n'
        printf 'determinism_claim\tnot_claimed\n'
    fi
    printf 'execution_path_sha256\t%s\n' "$(printf '%s' "$PRODUCT_ROOT" | sha256sum | awk '{print $1}')"
    printf 'cargo_version\t%s\n' "$(tool_version cargo)"
    printf 'rustc_version\t%s\n' "$(tool_version rustc)"
    printf 'ic_wasm_version\t%s\n' "$(tool_version ic-wasm)"
    printf 'didc_version\t%s\n' "$(tool_version didc)"
    printf 'wasm_objdump_version\t%s\n' "$(tool_version wasm-objdump)"
    printf 'wasm_validate_version\t%s\n' "$(tool_version wasm-validate)"
} >"$RUN_ROOT/run-metadata.tsv"

if [[ "$PATCH_APPLIED" == "true" ]]; then
    git -C "$PRODUCT_ROOT" apply --reverse "$PATCH_PATH"
    PATCH_APPLIED="false"
fi
rm -rf "$PRODUCT_ROOT/.icp"
SOURCE_STATUS_AFTER="$(git -C "$PRODUCT_ROOT" status --porcelain=v1 --untracked-files=all)"
[[ -z "$SOURCE_STATUS_AFTER" ]] || fail "ablation runner did not restore its clean product worktree"

if [[ "$RUN_MODE" == "smoke" ]]; then
    echo "Wasm ablation smoke passed (development-only; not retention-eligible): $RUN_ROOT"
else
    echo "Wasm ablation report: $RUN_ROOT"
fi
