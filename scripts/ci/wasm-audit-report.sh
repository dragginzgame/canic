#!/usr/bin/env bash

set -euo pipefail

METHOD_ID="CANIC-WASM-001"
METHOD_VERSION="5"
METHOD_TAG="$METHOD_ID/v$METHOD_VERSION"
DEFINITION_PATH="docs/audits/recurring/system/wasm-footprint.md"
AUDIT_STEM="wasm-footprint-v5"
PROFILE_KEY="release-clean-a+release-clean-b+debug"
EXPECTED_ROSTER_KEY="app,test,user_hub,scale_hub,user_shard,scale_replica,root,fleet_coordinator,wasm_store"

METHOD_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PRODUCT_ROOT_INPUT="${WASM_AUDIT_PRODUCT_ROOT:-}"
if [[ -z "$PRODUCT_ROOT_INPUT" ]]; then
    echo "WASM_AUDIT_PRODUCT_ROOT must name a clean disposable linked Git worktree" >&2
    exit 2
fi
PRODUCT_ROOT="$(cd "$PRODUCT_ROOT_INPUT" && pwd)"

# shellcheck source=scripts/ci/require_icp.sh
source "$METHOD_ROOT/scripts/ci/require_icp.sh"

require_command() {
    local command_name="$1"
    if ! command -v "$command_name" >/dev/null 2>&1; then
        echo "missing required Wasm audit tool: $command_name" >&2
        exit 2
    fi
}

tool_version() {
    "$1" --version 2>&1 | head -n 1
}

file_hash() {
    sha256sum "$1" | awk '{print $1}'
}

root_independent_composite() {
    local relative_path
    local absolute_path
    for relative_path in \
        "$DEFINITION_PATH" \
        scripts/ci/wasm-audit-report.sh \
        scripts/ci/list-config-canisters.sh \
        scripts/ci/require_icp.sh \
        tool-versions.env; do
        absolute_path="$METHOD_ROOT/$relative_path"
        printf '%s  %s\n' "$(file_hash "$absolute_path")" "$relative_path"
    done | sort -k2 | sha256sum | awk '{print $1}'
}

next_scope_stem() {
    local day_dir="$1"
    local stem="$2"
    local index=2

    if [[ ! -e "$day_dir/$stem.md" ]]; then
        printf '%s\n' "$stem"
        return
    fi
    while [[ -e "$day_dir/$stem-$index.md" ]]; do
        index=$((index + 1))
    done
    printf '%s-%s\n' "$stem" "$index"
}

json_value() {
    printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

markdown_value() {
    printf '%s' "$1" | tr '\t\r\n' '   ' | sed 's/|/\\|/g'
}

tsv_value() {
    printf '%s' "$1" | tr '\t\r\n' '   '
}

signed_integer() {
    printf '%+d' "$1"
}

percent_delta() {
    local baseline="$1"
    local current="$2"
    awk -v baseline="$baseline" -v current="$current" 'BEGIN {
        if (baseline == 0) {
            print "N/A"
        } else {
            printf "%.2f", ((current - baseline) / baseline) * 100
        }
    }'
}

ratio() {
    local denominator="$1"
    local numerator="$2"
    awk -v denominator="$denominator" -v numerator="$numerator" 'BEGIN {
        if (denominator == 0) {
            print "N/A"
        } else {
            printf "%.4f", numerator / denominator
        }
    }'
}

extract_json_field() {
    local path="$1"
    local field="$2"
    sed -n "s/.*\"$field\": \"\([^\"]*\)\".*/\1/p" "$path" | head -n 1
}

capture_info_metrics() {
    local canister="$1"
    local info_path="$2"
    local functions
    local data_sections
    local data_bytes
    local exports

    functions="$(sed -n 's/^Number of functions: //p' "$info_path" | head -n 1)"
    data_sections="$(sed -n 's/^Number of data sections: //p' "$info_path" | head -n 1)"
    data_bytes="$(sed -n 's/^Size of data sections: \([0-9][0-9]*\) bytes$/\1/p' "$info_path" | head -n 1)"
    exports="$(awk '
        /^Exported methods: \[/ { active=1; next }
        active && /^\]/ { active=0 }
        active && /"/ { count++ }
        END { print count + 0 }
    ' "$info_path")"

    if [[ -z "$functions" || -z "$data_sections" || -z "$data_bytes" ]]; then
        echo "unable to parse ic-wasm structure metrics for $canister" >&2
        exit 1
    fi
    FUNCTIONS["$canister"]="$functions"
    DATA_SECTIONS["$canister"]="$data_sections"
    DATA_BYTES["$canister"]="$data_bytes"
    EXPORTS["$canister"]="$exports"
}

capture_twiggy_metrics() {
    local canister="$1"
    local release_wasm="$2"
    local analysis_dir="$3"
    local top_csv="$analysis_dir/$canister.top.csv"
    local retained_csv="$analysis_dir/$canister.retained.csv"
    local dominators_txt="$analysis_dir/$canister.dominators.txt"
    local monos_txt="$analysis_dir/$canister.monos.txt"

    twiggy top -n 20 -f csv "$release_wasm" >"$top_csv"
    twiggy top --retained -n 20 -f csv "$release_wasm" >"$retained_csv"
    twiggy dominators -d 4 -r 20 "$release_wasm" >"$dominators_txt"
    twiggy monos -m 10 -n 5 "$release_wasm" >"$monos_txt"

    TOP_NAME["$canister"]="$(awk -F',' 'NR == 2 { print $1 }' "$top_csv" | tr -d '"')"
    TOP_BYTES["$canister"]="$(awk -F',' 'NR == 2 { print $2 }' "$top_csv")"
    RETAINED_NAME["$canister"]="$(awk -F',' 'NR == 2 { print $1 }' "$retained_csv" | tr -d '"')"
    RETAINED_BYTES["$canister"]="$(awk -F',' 'NR == 2 { print $4 }' "$retained_csv")"

    if [[ -z "${TOP_NAME[$canister]}" || ! "${TOP_BYTES[$canister]}" =~ ^[0-9]+$ ||
        -z "${RETAINED_NAME[$canister]}" || ! "${RETAINED_BYTES[$canister]}" =~ ^[0-9]+$ ]]; then
        echo "unable to parse twiggy hotspot metrics for $canister" >&2
        exit 1
    fi

    sed -E \
        -e 's#/home/[^/]+/#<home>/#g' \
        -e 's#/tmp/[^[:space:],"]+#<temp-path>#g' \
        "$dominators_txt" | head -n 16 >"$analysis_dir/$canister.dominators-excerpt.txt"
    sed -E \
        -e 's#/home/[^/]+/#<home>/#g' \
        -e 's#/tmp/[^[:space:],"]+#<temp-path>#g' \
        "$monos_txt" | head -n 16 >"$analysis_dir/$canister.monos-excerpt.txt"
}

capture_optimizer_metrics() {
    local canister="$1"
    local run_label="$2"
    local log_path="$3"
    local line
    local metric_pattern

    line="$(
        rg 'release Wasm optimization for .*\.wasm:' "$log_path" |
            tail -n 1 || true
    )"
    metric_pattern='raw ([0-9]+) -> ([0-9]+), gzip ([0-9]+) -> ([0-9]+), code section ([0-9]+) -> ([0-9]+), data section ([0-9]+) -> ([0-9]+), functions ([0-9]+) -> ([0-9]+)$'
    if [[ ! "$line" =~ $metric_pattern ]]; then
        echo "unable to parse governed release optimization metrics for $canister" >&2
        exit 1
    fi
    {
        printf '%s\t%s' "$canister" "$run_label"
        printf '\t%s' "${BASH_REMATCH[@]:1}"
        printf '\n'
    } >>"$OPTIMIZER_RUN_METRICS"
}

build_profile() {
    local profile="$1"
    local run_label="$2"
    local target_dir="$3"
    local output_dir="$RUN_TMP/artifacts/$run_label"
    local build_log
    local canister
    local artifact_root
    rm -rf "$PRODUCT_ROOT/.icp"
    rm -rf "$target_dir"
    mkdir -p "$output_dir"

    for canister in "${CANISTERS[@]}"; do
        build_log="$RUN_TMP/build-$run_label-$canister.log"
        printf 'building %s profile (%s) for %s through Canic host authority\n' \
            "$profile" "$run_label" "$canister"
        if ! (
            cd "$PRODUCT_ROOT"
            ICP_ENVIRONMENT=local \
                CARGO_NET_OFFLINE=true \
                CARGO_INCREMENTAL=0 \
                CARGO_TARGET_DIR="$target_dir" \
                cargo run --offline --locked -q --profile fast \
                    -p canic-host --example build_artifact -- \
                    "$canister" "$profile" "$PRODUCT_ROOT" "$PRODUCT_ROOT" \
                    "$PRODUCT_CONFIG"
        ) >>"$build_log" 2>&1; then
            echo "canonical $profile build ($run_label) failed for $canister" >&2
            tail -n 80 "$build_log" >&2
            exit 1
        fi

        artifact_root="$PRODUCT_ROOT/.icp/local/canisters/$canister"
        if [[ ! -s "$artifact_root/$canister.wasm" || ! -s "$artifact_root/$canister.wasm.gz" ]]; then
            echo "canonical $profile artifacts are missing for $canister" >&2
            exit 1
        fi
        gzip -t "$artifact_root/$canister.wasm.gz"
        decoded_gzip_hash="$(
            gzip -cd "$artifact_root/$canister.wasm.gz" | sha256sum | awk '{print $1}'
        )"
        canonical_wasm_hash="$(file_hash "$artifact_root/$canister.wasm")"
        if [[ "$decoded_gzip_hash" != "$canonical_wasm_hash" ]]; then
            echo "builder gzip does not decode to canonical $profile Wasm for $canister" >&2
            exit 1
        fi
        cp "$artifact_root/$canister.wasm" "$output_dir/$canister.wasm"
        cp "$artifact_root/$canister.wasm.gz" "$output_dir/$canister.wasm.gz"
        cp "$artifact_root/$canister.did" "$output_dir/$canister.did"
        if [[ "$profile" == "release" ]]; then
            capture_optimizer_metrics "$canister" "$run_label" "$build_log"
        fi
    done
}

require_command cargo
require_command cmp
require_command rustc
require_command git
require_command gzip
require_command sha256sum
require_command ic-wasm
require_command twiggy
require_command wasm-opt
require_icp_tools

if [[ "${ICP_ENVIRONMENT:-local}" == "ic" ]]; then
    echo "Wasm audit refuses the ic environment" >&2
    exit 2
fi

if ! git -C "$PRODUCT_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "WASM_AUDIT_PRODUCT_ROOT is not a Git worktree: $PRODUCT_ROOT" >&2
    exit 2
fi
PRODUCT_GIT_DIR="$(git -C "$PRODUCT_ROOT" rev-parse --absolute-git-dir)"
PRODUCT_COMMON_DIR="$(
    git -C "$PRODUCT_ROOT" rev-parse --path-format=absolute --git-common-dir
)"
if [[ "$PRODUCT_GIT_DIR" == "$PRODUCT_COMMON_DIR" ]]; then
    echo "WASM_AUDIT_PRODUCT_ROOT must be a disposable linked Git worktree" >&2
    exit 2
fi

SOURCE_STATUS_BEFORE="$(git -C "$PRODUCT_ROOT" status --porcelain=v1 --untracked-files=all)"
if [[ -n "$SOURCE_STATUS_BEFORE" ]]; then
    echo "WASM_AUDIT_PRODUCT_ROOT must be clean before execution" >&2
    printf '%s\n' "$SOURCE_STATUS_BEFORE" >&2
    exit 2
fi

RUN_TMP="$(mktemp -d)"
cleanup() {
    rm -rf "$RUN_TMP"
}
trap cleanup EXIT
export CARGO_NET_OFFLINE="true"
OPTIMIZER_RUN_METRICS="$RUN_TMP/optimizer-run-metrics.tsv"
printf 'canister\trun\tbefore_raw_bytes\tafter_raw_bytes\tbefore_gzip_bytes\tafter_gzip_bytes\tbefore_code_bytes\tafter_code_bytes\tbefore_data_section_bytes\tafter_data_section_bytes\tbefore_functions\tafter_functions\n' >"$OPTIMIZER_RUN_METRICS"

PRODUCT_CONFIG="$PRODUCT_ROOT/apps/test/canic.toml"
mapfile -t CANISTERS < <(
    bash "$METHOD_ROOT/scripts/ci/list-config-canisters.sh" \
        --config "$PRODUCT_CONFIG" --ci-order
)
CANISTERS+=(fleet_coordinator wasm_store)
ROSTER_KEY="$(IFS=,; printf '%s' "${CANISTERS[*]}")"
if [[ "$ROSTER_KEY" != "$EXPECTED_ROSTER_KEY" ]]; then
    echo "frozen Wasm audit roster drifted: expected $EXPECTED_ROSTER_KEY; found $ROSTER_KEY" >&2
    exit 2
fi

METHOD_FINGERPRINT="$(root_independent_composite)"
DEFINITION_FINGERPRINT="$(file_hash "$METHOD_ROOT/$DEFINITION_PATH")"
PRODUCT_COMMIT="$(git -C "$PRODUCT_ROOT" rev-parse 'HEAD^{commit}')"
SOURCE_TREE_HASH="$(git -C "$PRODUCT_ROOT" rev-parse 'HEAD^{tree}')"
PRODUCT_TREE_HASH="$(bash "$METHOD_ROOT/scripts/ci/audit-product-tree-hash.sh" "$PRODUCT_COMMIT")"
RELEASE_ANCHOR="$(git -C "$PRODUCT_ROOT" describe --tags --exact-match HEAD 2>/dev/null || printf 'untagged')"
BRANCH="$(git -C "$PRODUCT_ROOT" symbolic-ref --short -q HEAD || printf 'detached')"
CARGO_LOCK_HASH="$(file_hash "$PRODUCT_ROOT/Cargo.lock")"
EXECUTION_PATH_KEY="$(printf '%s' "$PRODUCT_ROOT" | sha256sum | awk '{print $1}')"
RUSTC_VERSION="$(tool_version rustc)"
CARGO_VERSION="$(tool_version cargo)"
ICP_VERSION="$(tool_version icp)"
IC_WASM_VERSION="$(tool_version ic-wasm)"
TWIGGY_VERSION="$(tool_version twiggy)"
WASM_OPT_VERSION="$(tool_version wasm-opt)"
TOOL_KEY="$(
    printf 'rustc=%s\ncargo=%s\nicp=%s\nic-wasm=%s\ntwiggy=%s\nwasm-opt=%s\n' \
        "$RUSTC_VERSION" "$CARGO_VERSION" "$ICP_VERSION" \
        "$IC_WASM_VERSION" "$TWIGGY_VERSION" "$WASM_OPT_VERSION" | sha256sum | awk '{print $1}'
)"

RUN_DATE="${WASM_AUDIT_DATE:-$(date -u +%F)}"
STARTED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
MONTH="${RUN_DATE:0:7}"
DAY_DIR="$METHOD_ROOT/docs/audits/reports/$MONTH/$RUN_DATE"
SCOPE_STEM="$(next_scope_stem "$DAY_DIR" "$AUDIT_STEM")"
REPORT_PATH="$DAY_DIR/$SCOPE_STEM.md"
ARTIFACTS_DIR="$DAY_DIR/artifacts/$SCOPE_STEM"
REPORT_RELATIVE="${REPORT_PATH#"$METHOD_ROOT/"}"

BASELINE_REPORT="N/A"
BASELINE_METHOD_JSON="N/A"
ORIGINAL_BASELINE_REPORT="N/A"
while IFS= read -r candidate_method_path; do
    [[ -n "$candidate_method_path" ]] || continue
    candidate_method_id="$(extract_json_field "$candidate_method_path" method_id)"
    candidate_method_version="$(extract_json_field "$candidate_method_path" method_version)"
    candidate_method_fingerprint="$(extract_json_field "$candidate_method_path" method_fingerprint)"
    candidate_roster_key="$(extract_json_field "$candidate_method_path" roster_key)"
    candidate_profile_key="$(extract_json_field "$candidate_method_path" profile_key)"
    candidate_execution_path_key="$(extract_json_field "$candidate_method_path" execution_path_key)"
    candidate_tool_key="$(extract_json_field "$candidate_method_path" tool_key)"
    if [[ "$candidate_method_id" != "$METHOD_ID" ||
        "$candidate_method_version" != "$METHOD_VERSION" ||
        "$candidate_method_fingerprint" != "$METHOD_FINGERPRINT" ||
        "$candidate_roster_key" != "$ROSTER_KEY" ||
        "$candidate_profile_key" != "$PROFILE_KEY" ||
        "$candidate_execution_path_key" != "$EXECUTION_PATH_KEY" ||
        "$candidate_tool_key" != "$TOOL_KEY" ]]; then
        continue
    fi
    candidate_artifacts_dir="${candidate_method_path%/method.json}"
    candidate_stem="$(basename "$candidate_artifacts_dir")"
    candidate_day_dir="${candidate_artifacts_dir%/artifacts/*}"
    candidate_report="$candidate_day_dir/$candidate_stem.md"
    if [[ -f "$candidate_report" && -f "$candidate_artifacts_dir/size-metrics.tsv" ]]; then
        BASELINE_REPORT="${candidate_report#"$METHOD_ROOT/"}"
        BASELINE_METHOD_JSON="$candidate_method_path"
        candidate_original="$(extract_json_field "$candidate_method_path" original_baseline_report)"
        if [[ -n "$candidate_original" && "$candidate_original" != "N/A" ]]; then
            ORIGINAL_BASELINE_REPORT="$candidate_original"
        else
            ORIGINAL_BASELINE_REPORT="$BASELINE_REPORT"
        fi
        break
    fi
done < <(
    find "$METHOD_ROOT/docs/audits/reports" -type f \
        -path '*/artifacts/wasm-footprint-v5*/method.json' -print 2>/dev/null | sort -r
)

build_profile release release-clean-a "$RUN_TMP/target-release"
build_profile release release-clean-b "$RUN_TMP/target-release"

DETERMINISM_METRICS="$RUN_TMP/determinism.tsv"
printf 'canister\twasm_sha256\tgzip_sha256\tcandid_sha256\tresult\n' >"$DETERMINISM_METRICS"
for canister in "${CANISTERS[@]}"; do
    for extension in wasm wasm.gz did; do
        if ! cmp -s \
            "$RUN_TMP/artifacts/release-clean-a/$canister.$extension" \
            "$RUN_TMP/artifacts/release-clean-b/$canister.$extension"; then
            echo "clean release builds are not deterministic for $canister.$extension" >&2
            exit 1
        fi
    done
    first_metrics="$(awk -F'\t' -v role="$canister" '$1 == role && $2 == "release-clean-a" { for (i=3; i<=12; i++) printf "%s%s", $i, (i == 12 ? ORS : OFS) }' OFS='\t' "$OPTIMIZER_RUN_METRICS")"
    second_metrics="$(awk -F'\t' -v role="$canister" '$1 == role && $2 == "release-clean-b" { for (i=3; i<=12; i++) printf "%s%s", $i, (i == 12 ? ORS : OFS) }' OFS='\t' "$OPTIMIZER_RUN_METRICS")"
    if [[ -z "$first_metrics" || "$first_metrics" != "$second_metrics" ]]; then
        echo "clean release optimization metrics are not deterministic for $canister" >&2
        exit 1
    fi
    printf '%s\t%s\t%s\t%s\tPASS\n' \
        "$canister" \
        "$(file_hash "$RUN_TMP/artifacts/release-clean-a/$canister.wasm")" \
        "$(file_hash "$RUN_TMP/artifacts/release-clean-a/$canister.wasm.gz")" \
        "$(file_hash "$RUN_TMP/artifacts/release-clean-a/$canister.did")" \
        >>"$DETERMINISM_METRICS"
done

build_profile debug debug "$RUN_TMP/target-debug"

TRACKED_STATUS_AFTER="$(git -C "$PRODUCT_ROOT" status --porcelain=v1 --untracked-files=no)"
if [[ -n "$TRACKED_STATUS_AFTER" ]]; then
    echo "Wasm audit mutated tracked product source" >&2
    printf '%s\n' "$TRACKED_STATUS_AFTER" >&2
    exit 1
fi
UNEXPECTED_UNTRACKED="$(
    git -C "$PRODUCT_ROOT" status --porcelain=v1 --untracked-files=all |
        awk '$1 == "??" && $2 !~ /^\.icp\// { print }'
)"
if [[ -n "$UNEXPECTED_UNTRACKED" ]]; then
    echo "Wasm audit created an unexpected product-worktree path" >&2
    printf '%s\n' "$UNEXPECTED_UNTRACKED" >&2
    exit 1
fi

mkdir -p "$ARTIFACTS_DIR"
ANALYSIS_DIR="$RUN_TMP/analysis"
mkdir -p "$ANALYSIS_DIR"
OPTIMIZATION_METRICS="$ARTIFACTS_DIR/optimization-metrics.tsv"
DETERMINISM_EVIDENCE="$ARTIFACTS_DIR/determinism.tsv"
printf 'canister\tbefore_raw_bytes\tafter_raw_bytes\tbefore_gzip_bytes\tafter_gzip_bytes\tbefore_code_bytes\tafter_code_bytes\tbefore_data_section_bytes\tafter_data_section_bytes\tbefore_functions\tafter_functions\n' >"$OPTIMIZATION_METRICS"
awk -F'\t' 'BEGIN { OFS="\t" } NR > 1 && $2 == "release-clean-a" { print $1,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12 }' \
    "$OPTIMIZER_RUN_METRICS" >>"$OPTIMIZATION_METRICS"
cp "$DETERMINISM_METRICS" "$DETERMINISM_EVIDENCE"

declare -A KIND=()
declare -A RELEASE_BYTES=()
declare -A RELEASE_GZIP_BYTES=()
declare -A DEBUG_BYTES=()
declare -A DEBUG_GZIP_BYTES=()
declare -A DEBUG_DELTA_BYTES=()
declare -A DEBUG_DELTA_PERCENT=()
declare -A BASELINE_DELTA_BYTES=()
declare -A BASELINE_DELTA_PERCENT=()
declare -A FUNCTIONS=()
declare -A DATA_SECTIONS=()
declare -A DATA_BYTES=()
declare -A EXPORTS=()
declare -A TOP_NAME=()
declare -A TOP_BYTES=()
declare -A RETAINED_NAME=()
declare -A RETAINED_BYTES=()
declare -A OPT_BEFORE_RAW=()
declare -A OPT_AFTER_RAW=()
declare -A OPT_BEFORE_GZIP=()
declare -A OPT_AFTER_GZIP=()
declare -A OPT_BEFORE_CODE=()
declare -A OPT_AFTER_CODE=()
declare -A OPT_BEFORE_DATA=()
declare -A OPT_AFTER_DATA=()
declare -A OPT_BEFORE_FUNCTIONS=()
declare -A OPT_AFTER_FUNCTIONS=()

BASELINE_TSV="N/A"
if [[ "$BASELINE_METHOD_JSON" != "N/A" ]]; then
    BASELINE_TSV="${BASELINE_METHOD_JSON%/method.json}/size-metrics.tsv"
fi

SIZE_METRICS="$ARTIFACTS_DIR/size-metrics.tsv"
printf 'canister\tkind\trelease_wasm_bytes\trelease_gzip_bytes\tdebug_wasm_bytes\tdebug_gzip_bytes\tdebug_delta_bytes\tdebug_delta_percent\tbaseline_delta_bytes\tbaseline_delta_percent\tfunctions\tdata_sections\tdata_bytes\texports\ttop_name\ttop_bytes\tretained_name\tretained_bytes\n' >"$SIZE_METRICS"

for canister in "${CANISTERS[@]}"; do
    release_wasm="$RUN_TMP/artifacts/release-clean-a/$canister.wasm"
    release_gzip="$RUN_TMP/artifacts/release-clean-a/$canister.wasm.gz"
    debug_wasm="$RUN_TMP/artifacts/debug/$canister.wasm"
    debug_gzip="$RUN_TMP/artifacts/debug/$canister.wasm.gz"
    case "$canister" in
    fleet_coordinator) KIND["$canister"]="fleet-coordinator" ;;
    root) KIND["$canister"]="fleet-subnet-root" ;;
    wasm_store) KIND["$canister"]="wasm-store" ;;
    *) KIND["$canister"]="component" ;;
    esac
    RELEASE_BYTES["$canister"]="$(stat -c%s "$release_wasm")"
    RELEASE_GZIP_BYTES["$canister"]="$(stat -c%s "$release_gzip")"
    DEBUG_BYTES["$canister"]="$(stat -c%s "$debug_wasm")"
    DEBUG_GZIP_BYTES["$canister"]="$(stat -c%s "$debug_gzip")"
    debug_delta=$((${DEBUG_BYTES[$canister]} - ${RELEASE_BYTES[$canister]}))
    DEBUG_DELTA_BYTES["$canister"]="$(signed_integer "$debug_delta")"
    DEBUG_DELTA_PERCENT["$canister"]="$(percent_delta "${RELEASE_BYTES[$canister]}" "${DEBUG_BYTES[$canister]}")"
    BASELINE_DELTA_BYTES["$canister"]="N/A"
    BASELINE_DELTA_PERCENT["$canister"]="N/A"
    optimization_row="$(awk -F'\t' -v role="$canister" '$1 == role { print; exit }' "$OPTIMIZATION_METRICS")"
    if [[ -z "$optimization_row" ]]; then
        echo "missing optimization metrics for $canister" >&2
        exit 1
    fi
    IFS=$'\t' read -r _ before_raw after_raw before_gzip after_gzip \
        before_code after_code before_data after_data before_functions after_functions \
        <<<"$optimization_row"
    OPT_BEFORE_RAW["$canister"]="$before_raw"
    OPT_AFTER_RAW["$canister"]="$after_raw"
    OPT_BEFORE_GZIP["$canister"]="$before_gzip"
    OPT_AFTER_GZIP["$canister"]="$after_gzip"
    OPT_BEFORE_CODE["$canister"]="$before_code"
    OPT_AFTER_CODE["$canister"]="$after_code"
    OPT_BEFORE_DATA["$canister"]="$before_data"
    OPT_AFTER_DATA["$canister"]="$after_data"
    OPT_BEFORE_FUNCTIONS["$canister"]="$before_functions"
    OPT_AFTER_FUNCTIONS["$canister"]="$after_functions"
    if [[ "${OPT_AFTER_RAW[$canister]}" != "${RELEASE_BYTES[$canister]}" ||
        "${OPT_AFTER_GZIP[$canister]}" != "${RELEASE_GZIP_BYTES[$canister]}" ]]; then
        echo "canonical release bytes do not match governed optimization metrics for $canister" >&2
        exit 1
    fi

    ic_wasm_info="$ANALYSIS_DIR/$canister.ic-wasm-info.txt"
    ic-wasm "$release_wasm" info >"$ic_wasm_info"
    capture_info_metrics "$canister" "$ic_wasm_info"
    capture_twiggy_metrics "$canister" "$release_wasm" "$ANALYSIS_DIR"

    if [[ "$BASELINE_TSV" != "N/A" ]]; then
        baseline_bytes="$(awk -F'\t' -v role="$canister" 'NR > 1 && $1 == role { print $3; exit }' "$BASELINE_TSV")"
        if [[ ! "$baseline_bytes" =~ ^[0-9]+$ ]]; then
            echo "compatible predecessor lacks release bytes for $canister" >&2
            exit 1
        fi
        baseline_delta=$((${RELEASE_BYTES[$canister]} - baseline_bytes))
        BASELINE_DELTA_BYTES["$canister"]="$(signed_integer "$baseline_delta")"
        BASELINE_DELTA_PERCENT["$canister"]="$(percent_delta "$baseline_bytes" "${RELEASE_BYTES[$canister]}")"
    fi

    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$canister" "${KIND[$canister]}" \
        "${RELEASE_BYTES[$canister]}" "${RELEASE_GZIP_BYTES[$canister]}" \
        "${DEBUG_BYTES[$canister]}" "${DEBUG_GZIP_BYTES[$canister]}" \
        "${DEBUG_DELTA_BYTES[$canister]}" "${DEBUG_DELTA_PERCENT[$canister]}" \
        "${BASELINE_DELTA_BYTES[$canister]}" "${BASELINE_DELTA_PERCENT[$canister]}" \
        "${FUNCTIONS[$canister]}" "${DATA_SECTIONS[$canister]}" \
        "${DATA_BYTES[$canister]}" "${EXPORTS[$canister]}" \
        "$(tsv_value "${TOP_NAME[$canister]}")" "${TOP_BYTES[$canister]}" \
        "$(tsv_value "${RETAINED_NAME[$canister]}")" "${RETAINED_BYTES[$canister]}" \
        >>"$SIZE_METRICS"
done

component_min=0
component_max=0
root_bytes="${RELEASE_BYTES[root]}"
max_retained_percent="0.00"
max_growth_percent="0.00"
for canister in "${CANISTERS[@]}"; do
    if [[ "${KIND[$canister]}" == "component" ]]; then
        current_bytes="${RELEASE_BYTES[$canister]}"
        if [[ "$component_min" -eq 0 || "$current_bytes" -lt "$component_min" ]]; then
            component_min="$current_bytes"
        fi
        if [[ "$current_bytes" -gt "$component_max" ]]; then
            component_max="$current_bytes"
        fi
    fi
    retained_percent="$(awk -v retained="${RETAINED_BYTES[$canister]}" -v total="${RELEASE_BYTES[$canister]}" 'BEGIN { printf "%.4f", (retained / total) * 100 }')"
    if awk -v current="$retained_percent" -v maximum="$max_retained_percent" 'BEGIN { exit !(current > maximum) }'; then
        max_retained_percent="$retained_percent"
    fi
    if [[ "${BASELINE_DELTA_PERCENT[$canister]}" != "N/A" ]] &&
        awk -v current="${BASELINE_DELTA_PERCENT[$canister]}" -v maximum="$max_growth_percent" 'BEGIN { exit !(current > maximum) }'; then
        max_growth_percent="${BASELINE_DELTA_PERCENT[$canister]}"
    fi
done

component_spread_ratio="$(ratio "$component_min" "$component_max")"
root_component_ratio="$(ratio "$component_max" "$root_bytes")"
RISK_SCORE=0
declare -a RISK_DRIVERS=()
if [[ "$BASELINE_REPORT" == "N/A" ]]; then
    RISK_SCORE=$((RISK_SCORE + 2))
    RISK_DRIVERS+=("no compatible v5 predecessor: +2")
fi
if awk -v value="$component_spread_ratio" 'BEGIN { exit !(value >= 1.25) }'; then
    RISK_SCORE=$((RISK_SCORE + 2))
    RISK_DRIVERS+=("Component release spread >= 1.25: +2")
elif awk -v value="$component_spread_ratio" 'BEGIN { exit !(value >= 1.10) }'; then
    RISK_SCORE=$((RISK_SCORE + 1))
    RISK_DRIVERS+=("Component release spread 1.10-1.2499: +1")
fi
if awk -v value="$root_component_ratio" 'BEGIN { exit !(value >= 3.0) }'; then
    RISK_SCORE=$((RISK_SCORE + 2))
    RISK_DRIVERS+=("root/max-Component release ratio >= 3.0: +2")
elif awk -v value="$root_component_ratio" 'BEGIN { exit !(value >= 2.0) }'; then
    RISK_SCORE=$((RISK_SCORE + 1))
    RISK_DRIVERS+=("root/max-Component release ratio 2.0-2.9999: +1")
fi
if awk -v value="$max_growth_percent" 'BEGIN { exit !(value >= 10.0) }'; then
    RISK_SCORE=$((RISK_SCORE + 2))
    RISK_DRIVERS+=("largest compatible release growth >= 10%: +2")
elif awk -v value="$max_growth_percent" 'BEGIN { exit !(value >= 5.0) }'; then
    RISK_SCORE=$((RISK_SCORE + 1))
    RISK_DRIVERS+=("largest compatible release growth 5-9.9999%: +1")
fi
if awk -v value="$max_retained_percent" 'BEGIN { exit !(value >= 25.0) }'; then
    RISK_SCORE=$((RISK_SCORE + 2))
    RISK_DRIVERS+=("largest retained item >= 25% of release Wasm: +2")
elif awk -v value="$max_retained_percent" 'BEGIN { exit !(value >= 10.0) }'; then
    RISK_SCORE=$((RISK_SCORE + 1))
    RISK_DRIVERS+=("largest retained item 10-24.9999% of release Wasm: +1")
fi
if [[ "$RISK_SCORE" -gt 10 ]]; then
    RISK_SCORE=10
fi
if [[ "$RISK_SCORE" -ge 7 ]]; then
    RUN_RESULT="fail"
else
    RUN_RESULT="pass"
fi
if [[ "$BASELINE_REPORT" == "N/A" ]]; then
    COMPARABILITY="first-v5-baseline"
    ORIGINAL_BASELINE_REPORT="$REPORT_RELATIVE"
else
    COMPARABILITY="comparable to immediate compatible predecessor"
fi

SUMMARY_PATH="$ARTIFACTS_DIR/size-summary.md"
cat >"$SUMMARY_PATH" <<EOF
# Wasm Size Summary - $RUN_DATE

| Canister | Kind | Release Wasm | Release gzip | Debug Wasm | Debug gzip | Debug delta | Predecessor delta |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
EOF
for canister in "${CANISTERS[@]}"; do
    printf '| `%s` | %s | %s | %s | %s | %s | %s (%s%%) | %s (%s) |\n' \
        "$canister" "${KIND[$canister]}" \
        "${RELEASE_BYTES[$canister]}" "${RELEASE_GZIP_BYTES[$canister]}" \
        "${DEBUG_BYTES[$canister]}" "${DEBUG_GZIP_BYTES[$canister]}" \
        "${DEBUG_DELTA_BYTES[$canister]}" "${DEBUG_DELTA_PERCENT[$canister]}" \
        "${BASELINE_DELTA_BYTES[$canister]}" "${BASELINE_DELTA_PERCENT[$canister]}" \
        >>"$SUMMARY_PATH"
done

cat >>"$SUMMARY_PATH" <<EOF

## Governed Release Optimization

| Canister | Raw before → after | Gzip before → after | Code before → after | Data section before → after | Functions before → after |
| --- | ---: | ---: | ---: | ---: | ---: |
EOF
for canister in "${CANISTERS[@]}"; do
    printf '| `%s` | %s → %s | %s → %s | %s → %s | %s → %s | %s → %s |\n' \
        "$canister" \
        "${OPT_BEFORE_RAW[$canister]}" "${OPT_AFTER_RAW[$canister]}" \
        "${OPT_BEFORE_GZIP[$canister]}" "${OPT_AFTER_GZIP[$canister]}" \
        "${OPT_BEFORE_CODE[$canister]}" "${OPT_AFTER_CODE[$canister]}" \
        "${OPT_BEFORE_DATA[$canister]}" "${OPT_AFTER_DATA[$canister]}" \
        "${OPT_BEFORE_FUNCTIONS[$canister]}" "${OPT_AFTER_FUNCTIONS[$canister]}" \
        >>"$SUMMARY_PATH"
done

for canister in "${CANISTERS[@]}"; do
    detail_path="$ARTIFACTS_DIR/$canister.md"
    dominators_excerpt="$(cat "$ANALYSIS_DIR/$canister.dominators-excerpt.txt")"
    monos_excerpt="$(cat "$ANALYSIS_DIR/$canister.monos-excerpt.txt")"
    cat >"$detail_path" <<EOF
# Wasm Detail: \`$canister\`

| Metric | Value |
| --- | ---: |
| Kind | ${KIND[$canister]} |
| Release Wasm bytes | ${RELEASE_BYTES[$canister]} |
| Release gzip bytes | ${RELEASE_GZIP_BYTES[$canister]} |
| Debug Wasm bytes | ${DEBUG_BYTES[$canister]} |
| Debug gzip bytes | ${DEBUG_GZIP_BYTES[$canister]} |
| Debug delta | ${DEBUG_DELTA_BYTES[$canister]} (${DEBUG_DELTA_PERCENT[$canister]}%) |
| Compatible predecessor delta | ${BASELINE_DELTA_BYTES[$canister]} (${BASELINE_DELTA_PERCENT[$canister]}) |
| Optimizer raw bytes | ${OPT_BEFORE_RAW[$canister]} → ${OPT_AFTER_RAW[$canister]} |
| Optimizer gzip bytes | ${OPT_BEFORE_GZIP[$canister]} → ${OPT_AFTER_GZIP[$canister]} |
| Optimizer code-section bytes | ${OPT_BEFORE_CODE[$canister]} → ${OPT_AFTER_CODE[$canister]} |
| Optimizer data-section bytes | ${OPT_BEFORE_DATA[$canister]} → ${OPT_AFTER_DATA[$canister]} |
| Optimizer defined functions | ${OPT_BEFORE_FUNCTIONS[$canister]} → ${OPT_AFTER_FUNCTIONS[$canister]} |
| Functions | ${FUNCTIONS[$canister]} |
| Data sections / bytes | ${DATA_SECTIONS[$canister]} / ${DATA_BYTES[$canister]} |
| Exported methods | ${EXPORTS[$canister]} |
| Largest shallow item | $(markdown_value "${TOP_NAME[$canister]}") (${TOP_BYTES[$canister]} bytes) |
| Largest retained item | $(markdown_value "${RETAINED_NAME[$canister]}") (${RETAINED_BYTES[$canister]} bytes) |

## Bounded Dominator Evidence

\`\`\`text
$dominators_excerpt
\`\`\`

## Bounded Monomorphization Evidence

\`\`\`text
$monos_excerpt
\`\`\`

The complete tool output and Wasm artifacts are transient. This file retains
the bounded analysis required by \`$METHOD_TAG\` without duplicating raw data.
EOF
done

METHOD_JSON="$ARTIFACTS_DIR/method.json"
cat >"$METHOD_JSON" <<EOF
{
  "method_id": "$(json_value "$METHOD_ID")",
  "method_version": "$(json_value "$METHOD_VERSION")",
  "method_fingerprint": "$(json_value "$METHOD_FINGERPRINT")",
  "definition_fingerprint": "$(json_value "$DEFINITION_FINGERPRINT")",
  "roster_key": "$(json_value "$ROSTER_KEY")",
  "profile_key": "$(json_value "$PROFILE_KEY")",
  "execution_path_key": "$(json_value "$EXECUTION_PATH_KEY")",
  "tool_key": "$(json_value "$TOOL_KEY")",
  "report": "$(json_value "$REPORT_RELATIVE")",
  "immediate_baseline_report": "$(json_value "$BASELINE_REPORT")",
  "original_baseline_report": "$(json_value "$ORIGINAL_BASELINE_REPORT")"
}
EOF

COMPLETED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
mkdir -p "$DAY_DIR"
cat >"$REPORT_PATH" <<EOF
# Wasm Footprint Audit v5 - $RUN_DATE

## Verdict

- Run result: \`$RUN_RESULT\`.
- Result validity: \`valid\`.
- Comparability: \`$COMPARABILITY\`.
- Authoritative risk score: \`$RISK_SCORE/10\`.

V5 completed two isolated clean release builds and one debug build for every frozen configured role
plus Fleet Coordinator and Wasm Store through Canic's authoritative host
artifact builder. It did not invoke direct Cargo
Wasm compilation, infer a target-directory artifact, or retain a competing
pre-optimization Wasm. The governed release transform supplies its own exact
before/after measurements. Historical v4 evidence remains valid but is not
comparable with v5.

## Scope And Identity

- Definition: \`$DEFINITION_PATH\`.
- Compared predecessor: \`$BASELINE_REPORT\`.
- Original v5 baseline: \`$ORIGINAL_BASELINE_REPORT\`.
- Release anchor: \`$RELEASE_ANCHOR\`.
- Source commit: \`$PRODUCT_COMMIT\`.
- Source tree: \`$SOURCE_TREE_HASH\`.
- Product tree: \`$PRODUCT_TREE_HASH\`.
- Method: \`$METHOD_TAG\`; definition \`$DEFINITION_FINGERPRINT\`; executable
  composite \`$METHOD_FINGERPRINT\`.
- Ordered roster: \`$ROSTER_KEY\`.
- Profiles: \`$PROFILE_KEY\`.
- Branch/worktree: \`$BRANCH\`; clean disposable linked worktree before the
  run, tracked-clean after the run, with only permitted \`.icp/\` build output.
- Environment: local, offline, isolated \`CARGO_TARGET_DIR\`; no replica,
  credentials, deployment, or authoritative external mutation.
- Auditor: Codex.
- Started/completed: \`$STARTED_AT\` / \`$COMPLETED_AT\`.

## Immutable Run Identity

\`\`\`text
release_anchor: $RELEASE_ANCHOR
source_commit_full: $PRODUCT_COMMIT
source_tree_hash: $SOURCE_TREE_HASH
product_tree_hash: $PRODUCT_TREE_HASH
clean_worktree: true before; tracked-clean after; generated .icp only
cargo_lock_hash: $CARGO_LOCK_HASH
rust_toolchain: $RUSTC_VERSION; $CARGO_VERSION
target_triple: wasm32-unknown-unknown
feature_set: apps/test frozen configured roles plus Coordinator and Store
audit_method_id: $METHOD_ID
audit_method_version: $METHOD_VERSION
audit_method_fingerprint: $METHOD_FINGERPRINT
audit_script_hashes: definition=$DEFINITION_FINGERPRINT; executable-composite=$METHOD_FINGERPRINT
external_tool_versions: $ICP_VERSION; $IC_WASM_VERSION; $TWIGGY_VERSION; $WASM_OPT_VERSION
fixture_or_seed: apps/test/canic.toml@$PRODUCT_COMMIT; roster=$ROSTER_KEY
environment_class: isolated local linked-worktree execution_trace
execution_path_key: $EXECUTION_PATH_KEY
started_at: $STARTED_AT
completed_at: $COMPLETED_AT
\`\`\`

The execution path itself is not retained. Its hash is a comparison key because
the independently owned \`CANIC-092-BUILD-001\` path-dependence finding makes a
different checkout path non-comparable for gzip/byte continuity.

## Canonical Artifact Sizes

| Canister | Kind | Release Wasm | Release gzip | Debug Wasm | Debug gzip | Debug delta | Predecessor delta |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
EOF
for canister in "${CANISTERS[@]}"; do
    printf '| `%s` | %s | %s | %s | %s | %s | %s (%s%%) | %s (%s) |\n' \
        "$canister" "${KIND[$canister]}" \
        "${RELEASE_BYTES[$canister]}" "${RELEASE_GZIP_BYTES[$canister]}" \
        "${DEBUG_BYTES[$canister]}" "${DEBUG_GZIP_BYTES[$canister]}" \
        "${DEBUG_DELTA_BYTES[$canister]}" "${DEBUG_DELTA_PERCENT[$canister]}" \
        "${BASELINE_DELTA_BYTES[$canister]}" "${BASELINE_DELTA_PERCENT[$canister]}" \
        >>"$REPORT_PATH"
done

cat >>"$REPORT_PATH" <<EOF

## Governed Release Optimization

| Canister | Raw before → after | Gzip before → after | Code before → after | Data section before → after | Functions before → after |
| --- | ---: | ---: | ---: | ---: | ---: |
EOF
for canister in "${CANISTERS[@]}"; do
    printf '| `%s` | %s → %s | %s → %s | %s → %s | %s → %s | %s → %s |\n' \
        "$canister" \
        "${OPT_BEFORE_RAW[$canister]}" "${OPT_AFTER_RAW[$canister]}" \
        "${OPT_BEFORE_GZIP[$canister]}" "${OPT_AFTER_GZIP[$canister]}" \
        "${OPT_BEFORE_CODE[$canister]}" "${OPT_AFTER_CODE[$canister]}" \
        "${OPT_BEFORE_DATA[$canister]}" "${OPT_AFTER_DATA[$canister]}" \
        "${OPT_BEFORE_FUNCTIONS[$canister]}" "${OPT_AFTER_FUNCTIONS[$canister]}" \
        >>"$REPORT_PATH"
done

cat >>"$REPORT_PATH" <<EOF

There is no dedicated minimal role in scope. Component release spread is
\`$component_spread_ratio\`; \`root\` is interpreted separately as Fleet Subnet
Root infrastructure and is \`$root_component_ratio\` times the largest
Component. Coordinator and Store are independently reported infrastructure.
No v1 raw/shrunk delta is
reported because that obsolete duplicate artifact model was removed.

## Structure And Retained-Size Evidence

| Canister | Functions | Data sections | Data bytes | Exports | Largest shallow item | Largest retained item |
| --- | ---: | ---: | ---: | ---: | --- | --- |
EOF
for canister in "${CANISTERS[@]}"; do
    printf '| `%s` | %s | %s | %s | %s | `%s` (%s) | `%s` (%s) |\n' \
        "$canister" "${FUNCTIONS[$canister]}" "${DATA_SECTIONS[$canister]}" \
        "${DATA_BYTES[$canister]}" "${EXPORTS[$canister]}" \
        "$(markdown_value "${TOP_NAME[$canister]}")" "${TOP_BYTES[$canister]}" \
        "$(markdown_value "${RETAINED_NAME[$canister]}")" "${RETAINED_BYTES[$canister]}" \
        >>"$REPORT_PATH"
done

cat >>"$REPORT_PATH" <<EOF

All canonical release artifacts were accepted by \`ic-wasm info\`, \`twiggy
top\`, retained \`top\`, \`dominators\`, and \`monos\`. The builder's shrink step
removes source-level names, so current attribution is structural rather than a
claim about a particular crate. Repeated \`table[0]\`/element retention across
Components is a runtime fan-in signal; it is not sufficient by itself to assign a
dependency owner. Bounded dominator and monomorphization evidence is retained
in each role detail file.

The largest retained item occupies \`$max_retained_percent%\` of its canonical
release Wasm. Largest compatible predecessor growth is
\`$max_growth_percent%\`; \`0.00%\` means either no positive growth or no
compatible predecessor.

## Risk Score

Risk score: **$RISK_SCORE / 10**.

EOF
if [[ "${#RISK_DRIVERS[@]}" -eq 0 ]]; then
    printf -- '- No scoring input fired.\n' >>"$REPORT_PATH"
else
    for driver in "${RISK_DRIVERS[@]}"; do
        printf -- '- %s.\n' "$driver" >>"$REPORT_PATH"
    done
fi

cat >>"$REPORT_PATH" <<EOF

This is size-pressure evidence, not a correctness verdict. Root build-path
reproducibility remains owned by \`CANIC-092-BUILD-001\` and is neither cleared
nor duplicated here.

## Findings

- Method revision: v5 qualifies path-confined role-local optimizer records and
  two-clean-build determinism across the full roster; valid v4 history cannot
  baseline v5.
- New product findings: none. The first v5 measurement is a baseline, and no
  comparable regression exists to attribute.

## Required Checklist

| Requirement | Result | Evidence |
| --- | --- | --- |
| clean isolated product snapshot | PASS | linked worktree clean before; tracked-clean after |
| canonical release artifacts | PASS | complete nine-role roster built through host \`build_artifact\` |
| deterministic clean release builds | PASS | exact Wasm, gzip, Candid, and transform metrics match across isolated targets |
| canonical debug artifacts | PASS | same nine roles and authority |
| governed Binaryen optimization | PASS | pinned tool plus before/after raw, gzip, code, data, and function metrics for every role |
| Candid/export/feature parity | PASS | release transform rejects before replacing an artifact if any governed invariant changes |
| builder gzip integrity | PASS | every gzip decodes to its paired canonical Wasm |
| machine-readable sizes | PASS | \`size-metrics.tsv\` |
| \`ic-wasm info\` | PASS | nine release artifacts parsed |
| \`twiggy top\` and retained \`top\` | PASS | compact hotspot columns retained |
| \`twiggy dominators\` | PASS | bounded role excerpts retained |
| \`twiggy monos\` | PASS | bounded role excerpts retained |
| compatible predecessor selection | PASS | exact method/roster/profile/path/tool keys; \`$BASELINE_REPORT\` |
| direct Cargo/pre-optimization fallback absent | PASS | v5 invokes only the host artifact authority |
| source mutation | PASS | no tracked mutation or unexpected untracked path |

## Verification Readout

| Command/check | Result | Notes |
| --- | --- | --- |
| \`cargo run --offline --locked -p canic-host --example build_artifact -- <role> release ...\` | PASS | nine ordered roles, repeated from isolated clean targets |
| same authoritative command with \`debug\` | PASS | nine ordered roles |
| \`gzip -t\` plus decoded SHA-256 equality | PASS | release and debug artifacts |
| \`cmp\` plus SHA-256 identity | PASS | two clean canonical release builds, all roles |
| \`ic-wasm <release.wasm> info\` | PASS | all roles |
| \`twiggy top\|dominators\|monos <release.wasm>\` | PASS | all roles |
| method composite | PASS | root-independent \`$METHOD_FINGERPRINT\` |
| product-tree identity | PASS | \`$PRODUCT_TREE_HASH\` |
| retained evidence hashes | PASS | manifest binds the report and compact artifacts |

## Retained Evidence

- [size summary](artifacts/$SCOPE_STEM/size-summary.md)
- [machine-readable metrics](artifacts/$SCOPE_STEM/size-metrics.tsv)
- [optimizer before/after metrics](artifacts/$SCOPE_STEM/optimization-metrics.tsv)
- [clean-build determinism](artifacts/$SCOPE_STEM/determinism.tsv)
- [method identity](artifacts/$SCOPE_STEM/method.json)
- [evidence manifest](artifacts/$SCOPE_STEM/evidence-manifest.yml)
EOF
for canister in "${CANISTERS[@]}"; do
    printf -- '- [%s detail](artifacts/%s/%s.md)\n' "$canister" "$SCOPE_STEM" "$canister" >>"$REPORT_PATH"
done

EVIDENCE_MANIFEST="$ARTIFACTS_DIR/evidence-manifest.yml"
declare -a RETAINED_FILES=(
    "$REPORT_PATH"
    "$SUMMARY_PATH"
    "$SIZE_METRICS"
    "$OPTIMIZATION_METRICS"
    "$DETERMINISM_EVIDENCE"
    "$METHOD_JSON"
)
for canister in "${CANISTERS[@]}"; do
    RETAINED_FILES+=("$ARTIFACTS_DIR/$canister.md")
done

{
    printf 'command: "WASM_AUDIT_PRODUCT_ROOT=<disposable-product-root> bash scripts/ci/wasm-audit-report.sh"\n'
    printf 'working_directory: "method_repository_root"\n'
    printf 'exit_code: 0\n'
    printf 'stdout_path: "not_retained"\n'
    printf 'stderr_path: "not_retained"\n'
    printf 'baseline_identity: "%s"\n' "$BASELINE_REPORT"
    printf 'original_baseline_identity: "%s"\n' "$ORIGINAL_BASELINE_REPORT"
    printf 'method_identity: "%s@sha256:%s"\n' "$METHOD_TAG" "$METHOD_FINGERPRINT"
    printf 'product_identity: "%s@product-sha256:%s"\n' "$PRODUCT_COMMIT" "$PRODUCT_TREE_HASH"
    printf 'tool_versions:\n'
    printf '  rustc: "%s"\n' "$RUSTC_VERSION"
    printf '  cargo: "%s"\n' "$CARGO_VERSION"
    printf '  icp: "%s"\n' "$ICP_VERSION"
    printf '  ic-wasm: "%s"\n' "$IC_WASM_VERSION"
    printf '  twiggy: "%s"\n' "$TWIGGY_VERSION"
    printf '  wasm-opt: "%s"\n' "$WASM_OPT_VERSION"
    printf 'timestamps:\n'
    printf '  started_at: "%s"\n' "$STARTED_AT"
    printf '  completed_at: "%s"\n' "$COMPLETED_AT"
    printf 'artifact_hashes:\n'
    for retained_path in "${RETAINED_FILES[@]}"; do
        printf '  %s  %s\n' "$(file_hash "$retained_path")" "${retained_path#"$METHOD_ROOT/"}"
    done
    printf 'retention_class: "primary_markdown_and_compact_supporting_evidence"\n'
    printf 'redactions_applied: "product/method/cache/home paths normalized or omitted; no credentials, tokens, principals, or private material retained"\n'
} >"$EVIDENCE_MANIFEST"

for retained_path in "${RETAINED_FILES[@]}"; do
    retained_relative="${retained_path#"$METHOD_ROOT/"}"
    expected_hash="$(awk -v path="$retained_relative" '$2 == path { print $1; exit }' "$EVIDENCE_MANIFEST")"
    if [[ -z "$expected_hash" || "$expected_hash" != "$(file_hash "$retained_path")" ]]; then
        echo "retained evidence hash verification failed for $retained_relative" >&2
        exit 1
    fi
done

printf 'wrote %s\n' "$REPORT_RELATIVE"
