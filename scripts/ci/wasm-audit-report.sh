#!/usr/bin/env bash

set -euo pipefail

METHOD_TAG="Method V1"
AUDIT_SLUG="wasm-footprint"
DEFINITION_PATH="docs/audits/recurring/system/wasm-footprint.md"
DEFAULT_PROFILE="release"

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"
source "$ROOT_DIR/scripts/app/reference_canisters.sh"

DEFAULT_CANISTERS=("${REFERENCE_CANISTERS[@]}")

declare -a VERIFICATION_ROWS=()
declare -A BUILT_WASM_BYTES=()
declare -A BUILT_WASM_GZ_BYTES=()
declare -A SHRUNK_WASM_BYTES=()
declare -A SHRUNK_WASM_GZ_BYTES=()
declare -A SHRINK_DELTA_BYTES=()
declare -A SHRINK_DELTA_PCT=()
declare -A BUILT_FUNCTIONS=()
declare -A BUILT_DATA_SECTIONS=()
declare -A BUILT_DATA_BYTES=()
declare -A BUILT_EXPORTS=()
declare -A SHRUNK_FUNCTIONS=()
declare -A SHRUNK_DATA_SECTIONS=()
declare -A SHRUNK_DATA_BYTES=()
declare -A SHRUNK_EXPORTS=()
declare -A BASELINE_DELTA_BYTES=()
declare -A BASELINE_DELTA_PCT=()
declare -A HOTSPOT_NAME=()
declare -A HOTSPOT_SHALLOW=()
declare -A HOTSPOT_RETAINED=()
declare -A CANISTER_KIND=()
declare -A BUILT_ALREADY=()

has_cmd() {
    command -v "$1" >/dev/null 2>&1
}

record_verification() {
    local cmd="$1"
    local status="$2"
    local notes="$3"
    VERIFICATION_ROWS+=("| \`$cmd\` | $status | $notes |")
}

next_scope_stem() {
    local day_dir="$1"
    local base="$2"

    if [ ! -e "$day_dir/$base.md" ]; then
        printf '%s\n' "$base"
        return
    fi

    local idx=2
    while [ -e "$day_dir/$base-$idx.md" ]; do
        idx=$((idx + 1))
    done
    printf '%s-%s\n' "$base" "$idx"
}

byte_size() {
    stat -c%s "$1"
}

gzip_deterministic() {
    local input="$1"
    local output="$2"
    gzip -n -c "$input" >"$output"
}

normalize_profile() {
    case "${WASM_PROFILE:-$DEFAULT_PROFILE}" in
    release)
        PROFILE_NAME="release"
        PROFILE_DIR="release"
        CARGO_PROFILE_FLAG="--release"
        ;;
    fast)
        PROFILE_NAME="fast"
        PROFILE_DIR="fast"
        CARGO_PROFILE_FLAG="--profile fast"
        ;;
    wasm-debug | debug)
        PROFILE_NAME="wasm-debug"
        PROFILE_DIR="debug"
        CARGO_PROFILE_FLAG=""
        ;;
    *)
        echo "unsupported WASM_PROFILE: ${WASM_PROFILE:-$DEFAULT_PROFILE}" >&2
        exit 1
        ;;
    esac
}

select_canisters() {
    if [ -n "${WASM_CANISTER_NAME:-}" ]; then
        CANISTERS=("${WASM_CANISTER_NAME}")
    else
        CANISTERS=("${DEFAULT_CANISTERS[@]}")
    fi
}

capture_info_metrics() {
    local canister="$1"
    local kind="$2"
    local info_file="$3"
    local functions
    local data_sections
    local data_bytes
    local exports

    functions="$(sed -n 's/^Number of functions: //p' "$info_file" | head -n 1)"
    data_sections="$(sed -n 's/^Number of data sections: //p' "$info_file" | head -n 1)"
    data_bytes="$(sed -n 's/^Size of data sections: //p' "$info_file" | sed 's/ bytes$//' | head -n 1)"
    exports="$(
        awk '
            /^Exported methods: \[/ {in_block=1; next}
            in_block && /^\]/ {in_block=0}
            in_block && /"/ {count++}
            END {print count + 0}
        ' "$info_file"
    )"

    case "$kind" in
    built)
        BUILT_FUNCTIONS["$canister"]="${functions:-N/A}"
        BUILT_DATA_SECTIONS["$canister"]="${data_sections:-N/A}"
        BUILT_DATA_BYTES["$canister"]="${data_bytes:-N/A}"
        BUILT_EXPORTS["$canister"]="${exports:-0}"
        ;;
    shrunk)
        SHRUNK_FUNCTIONS["$canister"]="${functions:-N/A}"
        SHRUNK_DATA_SECTIONS["$canister"]="${data_sections:-N/A}"
        SHRUNK_DATA_BYTES["$canister"]="${data_bytes:-N/A}"
        SHRUNK_EXPORTS["$canister"]="${exports:-0}"
        ;;
    *)
        echo "unknown info metric kind: $kind" >&2
        exit 1
        ;;
    esac
}

capture_hotspot_summary() {
    local canister="$1"
    local top_csv="$2"
    local retained_csv="$3"

    local shallow_name shallow_size retained_name retained_size

    shallow_name="$(awk -F',' 'NR==2 {print $1}' "$top_csv" | tr -d '"')"
    shallow_size="$(awk -F',' 'NR==2 {print $2}' "$top_csv")"
    retained_name="$(awk -F',' 'NR==2 {print $1}' "$retained_csv" | tr -d '"')"
    retained_size="$(awk -F',' 'NR==2 {print $4}' "$retained_csv")"

    HOTSPOT_NAME["$canister"]="${retained_name:-${shallow_name:-N/A}}"
    HOTSPOT_SHALLOW["$canister"]="${shallow_size:-N/A}"
    HOTSPOT_RETAINED["$canister"]="${retained_size:-N/A}"
}

ensure_raw_canister() {
    local canister="$1"

    if [ -n "${BUILT_ALREADY[$canister]:-}" ]; then
        return
    fi

    mkdir -p ".dfx/local/canisters/$canister"
    cargo build --target wasm32-unknown-unknown -p "canister_${canister}" $CARGO_PROFILE_FLAG --locked

    local source_wasm="target/wasm32-unknown-unknown/$PROFILE_DIR/canister_${canister}.wasm"
    local raw_wasm="$CACHE_RAW_DIR/$canister.wasm"
    local raw_gz="$CACHE_RAW_DIR/$canister.wasm.gz"

    cp -f "$source_wasm" "$raw_wasm"
    gzip_deterministic "$raw_wasm" "$raw_gz"

    if [ "$canister" != "root" ]; then
        cp -f "$raw_wasm" ".dfx/local/canisters/$canister/$canister.wasm"
        cp -f "$raw_gz" ".dfx/local/canisters/$canister/$canister.wasm.gz"
    fi

    BUILT_ALREADY["$canister"]=1
}

build_and_cache_artifacts() {
    mkdir -p "$CACHE_RAW_DIR" "$CACHE_SHRUNK_DIR" "$CACHE_ANALYSIS_DIR"
    mkdir -p .dfx/local/canisters

    local include_root=0
    local canister
    for canister in "${CANISTERS[@]}"; do
        if [ "$canister" = "root" ]; then
            include_root=1
        fi
    done

    if [ "$include_root" -eq 1 ]; then
        for canister in "${ROOT_RELEASE_SET_CANISTERS[@]}"; do
            ensure_raw_canister "$canister"
        done
    fi

    for canister in "${CANISTERS[@]}"; do
        if [ "$canister" != "root" ]; then
            ensure_raw_canister "$canister"
        fi
    done

    if [ "$include_root" -eq 1 ]; then
        ensure_raw_canister root
    fi

    for canister in "${CANISTERS[@]}"; do
        dfx build "$canister"
    done

    for canister in "${CANISTERS[@]}"; do
        local shrunk_wasm_src=".dfx/local/canisters/$canister/$canister.wasm"
        local shrunk_wasm="$CACHE_SHRUNK_DIR/$canister.wasm"
        local shrunk_gz="$CACHE_SHRUNK_DIR/$canister.wasm.gz"
        local analysis_wasm="$CACHE_ANALYSIS_DIR/$canister.wasm"

        cp -f "$shrunk_wasm_src" "$shrunk_wasm"
        gzip_deterministic "$shrunk_wasm" "$shrunk_gz"
        cp -f "$CACHE_RAW_DIR/$canister.wasm" "$analysis_wasm"
    done
}

ensure_local_dfx_ready() {
    if ! dfx ping local >/dev/null 2>&1; then
        echo "local dfx replica is not reachable; start it manually before running scripts/ci/wasm-audit-report.sh" >&2
        exit 1
    fi

    if ! dfx canister create --all -qq >/dev/null 2>&1; then
        echo "dfx canister create --all failed; verify the local replica is healthy and the current dfx project can create canisters" >&2
        exit 1
    fi
}

validate_cached_artifacts() {
    local canister
    for canister in "${CANISTERS[@]}"; do
        local raw_wasm="$CACHE_RAW_DIR/$canister.wasm"
        local shrunk_wasm="$CACHE_SHRUNK_DIR/$canister.wasm"
        local analysis_wasm="$CACHE_ANALYSIS_DIR/$canister.wasm"

        if [ ! -f "$raw_wasm" ] || [ ! -f "$shrunk_wasm" ]; then
            echo "missing cached artifacts for $canister under $CACHE_ROOT" >&2
            exit 1
        fi

        if [ ! -f "$CACHE_RAW_DIR/$canister.wasm.gz" ]; then
            gzip_deterministic "$raw_wasm" "$CACHE_RAW_DIR/$canister.wasm.gz"
        fi
        if [ ! -f "$CACHE_SHRUNK_DIR/$canister.wasm.gz" ]; then
            gzip_deterministic "$shrunk_wasm" "$CACHE_SHRUNK_DIR/$canister.wasm.gz"
        fi
        if [ ! -f "$analysis_wasm" ]; then
            cp -f "$raw_wasm" "$analysis_wasm"
        fi
    done
}

write_per_canister_json() {
    local canister="$1"
    local output="$2"

    cat >"$output" <<EOF
{
  "canister": "$canister",
  "kind": "${CANISTER_KIND[$canister]}",
  "profile": "$PROFILE_NAME",
  "built": {
    "wasm_bytes": ${BUILT_WASM_BYTES[$canister]},
    "wasm_gz_bytes": ${BUILT_WASM_GZ_BYTES[$canister]},
    "functions": ${BUILT_FUNCTIONS[$canister]},
    "data_sections": ${BUILT_DATA_SECTIONS[$canister]},
    "data_bytes": ${BUILT_DATA_BYTES[$canister]},
    "exports": ${BUILT_EXPORTS[$canister]}
  },
  "shrunk": {
    "wasm_bytes": ${SHRUNK_WASM_BYTES[$canister]},
    "wasm_gz_bytes": ${SHRUNK_WASM_GZ_BYTES[$canister]},
    "functions": ${SHRUNK_FUNCTIONS[$canister]},
    "data_sections": ${SHRUNK_DATA_SECTIONS[$canister]},
    "data_bytes": ${SHRUNK_DATA_BYTES[$canister]},
    "exports": ${SHRUNK_EXPORTS[$canister]}
  },
  "shrink_delta_bytes": ${SHRINK_DELTA_BYTES[$canister]},
  "shrink_delta_percent": ${SHRINK_DELTA_PCT[$canister]},
  "baseline_delta_bytes": "${BASELINE_DELTA_BYTES[$canister]}",
  "baseline_delta_percent": "${BASELINE_DELTA_PCT[$canister]}"
}
EOF
}

write_per_canister_markdown() {
    local canister="$1"
    local output="$2"

    cat >"$output" <<EOF
# Wasm Detail: \`$canister\`

## Artifact Snapshot

| Metric | Value |
| --- | ---: |
| Kind | ${CANISTER_KIND[$canister]} |
| Built wasm bytes | ${BUILT_WASM_BYTES[$canister]} |
| Built wasm.gz bytes | ${BUILT_WASM_GZ_BYTES[$canister]} |
| Shrunk wasm bytes | ${SHRUNK_WASM_BYTES[$canister]} |
| Shrunk wasm.gz bytes | ${SHRUNK_WASM_GZ_BYTES[$canister]} |
| Shrink delta bytes | ${SHRINK_DELTA_BYTES[$canister]} |
| Shrink delta percent | ${SHRINK_DELTA_PCT[$canister]}% |
| Baseline delta bytes | ${BASELINE_DELTA_BYTES[$canister]} |
| Baseline delta percent | ${BASELINE_DELTA_PCT[$canister]} |

## Structure Snapshot

| Metric | Built | Shrunk |
| --- | ---: | ---: |
| Functions | ${BUILT_FUNCTIONS[$canister]} | ${SHRUNK_FUNCTIONS[$canister]} |
| Data sections | ${BUILT_DATA_SECTIONS[$canister]} | ${SHRUNK_DATA_SECTIONS[$canister]} |
| Data bytes | ${BUILT_DATA_BYTES[$canister]} | ${SHRUNK_DATA_BYTES[$canister]} |
| Exported methods | ${BUILT_EXPORTS[$canister]} | ${SHRUNK_EXPORTS[$canister]} |

## Hotspot Snapshot

- Retained hotspot: \`${HOTSPOT_NAME[$canister]}\`
- Retained size: \`${HOTSPOT_RETAINED[$canister]}\`
- Shallow size: \`${HOTSPOT_SHALLOW[$canister]}\`

## Artifacts

- [${canister}.size-report.json](${canister}.size-report.json)
- [${canister}.built.ic-wasm-info.txt](${canister}.built.ic-wasm-info.txt)
- [${canister}.shrunk.ic-wasm-info.txt](${canister}.shrunk.ic-wasm-info.txt)
- [${canister}.twiggy-top.txt](${canister}.twiggy-top.txt)
- [${canister}.twiggy-top.csv](${canister}.twiggy-top.csv)
- [${canister}.twiggy-retained.csv](${canister}.twiggy-retained.csv)
- [${canister}.twiggy-dominators.txt](${canister}.twiggy-dominators.txt)
- [${canister}.twiggy-monos.txt](${canister}.twiggy-monos.txt)
EOF
}

write_aggregate_json() {
    local output="$1"
    local first=1
    local canister

    {
        printf '{\n'
        printf '  "scope": "%s",\n' "$AUDIT_SLUG"
        printf '  "profile": "%s",\n' "$PROFILE_NAME"
        printf '  "canisters": [\n'
        for canister in "${CANISTERS[@]}"; do
            if [ "$first" -eq 0 ]; then
                printf ',\n'
            fi
            first=0
            printf '    {\n'
            printf '      "canister": "%s",\n' "$canister"
            printf '      "kind": "%s",\n' "${CANISTER_KIND[$canister]}"
            printf '      "built_wasm_bytes": %s,\n' "${BUILT_WASM_BYTES[$canister]}"
            printf '      "built_wasm_gz_bytes": %s,\n' "${BUILT_WASM_GZ_BYTES[$canister]}"
            printf '      "shrunk_wasm_bytes": %s,\n' "${SHRUNK_WASM_BYTES[$canister]}"
            printf '      "shrunk_wasm_gz_bytes": %s,\n' "${SHRUNK_WASM_GZ_BYTES[$canister]}"
            printf '      "shrink_delta_bytes": %s,\n' "${SHRINK_DELTA_BYTES[$canister]}"
            printf '      "shrink_delta_percent": %s,\n' "${SHRINK_DELTA_PCT[$canister]}"
            printf '      "baseline_delta_bytes": "%s"\n' "${BASELINE_DELTA_BYTES[$canister]}"
            printf '    }'
        done
        printf '\n  ]\n'
        printf '}\n'
    } >"$output"
}

profile_command_note() {
    if [ "$PROFILE_NAME" = "release" ]; then
        printf 'cargo/dfx release builds'
    elif [ "$PROFILE_NAME" = "fast" ]; then
        printf 'cargo/dfx fast builds'
    else
        printf 'cargo/dfx debug builds'
    fi
}

normalize_delta_pct() {
    local base="$1"
    local current="$2"
    awk -v base="$base" -v current="$current" 'BEGIN {
        if (base == 0) {
            print "N/A"
        } else {
            printf "%.2f", ((base - current) / base) * 100
        }
    }'
}

normalize_baseline_delta_pct() {
    local base="$1"
    local current="$2"
    awk -v base="$base" -v current="$current" 'BEGIN {
        if (base == 0) {
            print "N/A"
        } else {
            printf "%.2f%%", ((current - base) / base) * 100
        }
    }'
}

run_top_summary() {
    local output="$1"
    local canister

    cat >"$output" <<EOF
# Wasm Size Summary - $RUN_DATE

| Canister | Kind | Built wasm | Shrunk wasm | Delta | Built gz | Shrunk gz |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
EOF

    for canister in "${CANISTERS[@]}"; do
        printf '| `%s` | %s | %s | %s | %s | %s | %s |\n' \
            "$canister" \
            "${CANISTER_KIND[$canister]}" \
            "${BUILT_WASM_BYTES[$canister]}" \
            "${SHRUNK_WASM_BYTES[$canister]}" \
            "${SHRINK_DELTA_BYTES[$canister]}" \
            "${BUILT_WASM_GZ_BYTES[$canister]}" \
            "${SHRUNK_WASM_GZ_BYTES[$canister]}" \
            >>"$output"
    done
}

determine_risk_score() {
    local max_leaf_shrunk=0
    local min_leaf_shrunk=0
    local root_shrunk=0
    local canister

    for canister in "${CANISTERS[@]}"; do
        if [ "$canister" = "root" ]; then
            root_shrunk="${SHRUNK_WASM_BYTES[$canister]}"
            continue
        fi

        if [ "$min_leaf_shrunk" -eq 0 ] || [ "${SHRUNK_WASM_BYTES[$canister]}" -lt "$min_leaf_shrunk" ]; then
            min_leaf_shrunk="${SHRUNK_WASM_BYTES[$canister]}"
        fi
        if [ "${SHRUNK_WASM_BYTES[$canister]}" -gt "$max_leaf_shrunk" ]; then
            max_leaf_shrunk="${SHRUNK_WASM_BYTES[$canister]}"
        fi
    done

    RISK_SCORE=3
    if [ "$max_leaf_shrunk" -gt 0 ] && [ "$min_leaf_shrunk" -gt 0 ]; then
        local leaf_spread
        leaf_spread="$(awk -v max="$max_leaf_shrunk" -v min="$min_leaf_shrunk" 'BEGIN { printf "%.2f", max / min }')"
        if awk -v spread="$leaf_spread" 'BEGIN { exit !(spread >= 1.25) }'; then
            RISK_SCORE=$((RISK_SCORE + 2))
        fi
    fi
    if [ "$root_shrunk" -gt 0 ] && [ "$max_leaf_shrunk" -gt 0 ]; then
        if awk -v root="$root_shrunk" -v leaf="$max_leaf_shrunk" 'BEGIN { exit !(root >= (leaf * 3)) }'; then
            RISK_SCORE=$((RISK_SCORE + 2))
        fi
    fi
    if [ "${#CANISTERS[@]}" -ge 2 ]; then
        RISK_SCORE=$((RISK_SCORE + 1))
    fi
    if [ "$RISK_SCORE" -gt 10 ]; then
        RISK_SCORE=10
    fi
}

normalize_worktree() {
    if git diff-index --quiet HEAD --; then
        printf 'clean'
    else
        printf 'dirty'
    fi
}

normalize_branch() {
    git rev-parse --abbrev-ref HEAD 2>/dev/null || printf 'N/A'
}

normalize_commit() {
    git rev-parse --short HEAD 2>/dev/null || printf 'N/A'
}

render_report() {
    local report_path="$1"
    local checklist_artifacts
    local checklist_json
    local checklist_top
    local checklist_dominators
    local checklist_monos
    local checklist_baseline
    local checklist_delta
    local canister

    checklist_artifacts="PASS"
    checklist_json="PASS"
    checklist_top="PASS"
    checklist_dominators="PASS"
    checklist_monos="PASS"
    checklist_baseline="PASS"
    checklist_delta="PASS"

    if [ "$BASELINE_PATH" = "N/A" ]; then
        checklist_delta="PARTIAL"
    fi
    if [ "$TWIGGY_AVAILABLE" -eq 0 ]; then
        checklist_top="PARTIAL"
        checklist_dominators="PARTIAL"
        checklist_monos="PARTIAL"
    fi

    cat >"$report_path" <<EOF
# Wasm Footprint Audit - $RUN_DATE

## Report Preamble

- Scope: Canic wasm footprint
- Definition path: \`$DEFINITION_PATH\`
- Compared baseline report path: \`$BASELINE_PATH\`
- Code snapshot identifier: \`$COMMIT\`
- Method tag/version: \`$METHOD_TAG\`
- Comparability status: \`$COMPARABILITY\`
- Auditor: \`codex\`
- Run timestamp (UTC): \`$RUN_TIMESTAMP\`
- Branch: \`$BRANCH\`
- Worktree: \`$WORKTREE\`
- Profile: \`$PROFILE_NAME\`
- Target canisters in scope: $(printf '`%s` ' "${CANISTERS[@]}")
- Analysis artifact note: \`twiggy\` ran against cached raw Cargo wasm to preserve readable symbol names; built/shrunk byte metrics still use the canonical built and \`dfx\`-shrunk artifacts.

## Findings / Checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Wasm artifacts captured for scope | $checklist_artifacts | Cached raw artifacts under \`artifacts/wasm-size/$PROFILE_NAME/raw/\` and shrunk artifacts under \`artifacts/wasm-size/$PROFILE_NAME/shrunk/\` were recorded for $(printf '`%s` ' "${CANISTERS[@]}"). |
| Artifact sizes recorded in machine-readable artifact | $checklist_json | [size-report.json]($ARTIFACT_LINK_PREFIX/size-report.json) plus per-canister \`*.size-report.json\` files. |
| Twiggy top captured | $checklist_top | \`*.twiggy-top.txt\` and \`*.twiggy-top.csv\` emitted for each canister when \`twiggy\` is available. |
| Twiggy dominators captured | $checklist_dominators | \`*.twiggy-dominators.txt\` emitted for each canister when \`twiggy\` is available. |
| Twiggy monos captured | $checklist_monos | \`*.twiggy-monos.txt\` emitted for each canister when \`twiggy\` is available. |
| Baseline path selected by daily baseline discipline | $checklist_baseline | Current run stem is \`$SCOPE_STEM\`; baseline path resolves to \`$BASELINE_PATH\`. |
| Size deltas versus baseline recorded when baseline exists | $checklist_delta | $( [ "$BASELINE_PATH" = "N/A" ] && printf 'First run of day; baseline deltas are \`N/A\`.' || printf 'Baseline deltas were calculated from \`%s\`.' "$BASELINE_PATH" ) |
| Verification readout captured | PASS | Command outcomes are recorded in the Verification Readout section. |

## Comparison to Previous Relevant Run

EOF

    if [ "$BASELINE_PATH" = "N/A" ]; then
        cat >>"$report_path" <<EOF
- First run of day for \`$AUDIT_SLUG\`; this report establishes the daily baseline.
- Baseline drift values are \`N/A\` until a same-day rerun or later comparable run exists.
EOF
    else
        cat >>"$report_path" <<EOF
- Same-day rerun against baseline \`$BASELINE_PATH\`.
- Per-canister baseline deltas in the snapshot table compare current shrunk wasm bytes to the baseline run.
EOF
    fi

    cat >>"$report_path" <<EOF

## Structural Hotspots

| Canister | Kind | Current hotspot | Retained size | Reason | Evidence |
| --- | --- | --- | ---: | --- | --- |
EOF

    for canister in "${CANISTERS[@]}"; do
        local reason="largest retained symbol from raw-built twiggy analysis"
        if [ "$canister" = "root" ]; then
            reason="control-plane outlier; embeds only the bootstrap wasm_store artifact and should not be compared directly to leaf peers"
        elif [ "$canister" = "minimal" ]; then
            reason="shared-runtime floor; use this to judge workspace baseline pressure"
        fi

        printf '| `%s` | %s | `%s` | %s | %s | [%s.md](%s/%s.md) |\n' \
            "$canister" \
            "${CANISTER_KIND[$canister]}" \
            "${HOTSPOT_NAME[$canister]}" \
            "${HOTSPOT_RETAINED[$canister]}" \
            "$reason" \
            "$canister" \
            "$ARTIFACT_LINK_PREFIX" \
            "$canister" \
            >>"$report_path"
    done

    cat >>"$report_path" <<EOF

## Dependency Fan-In Pressure

- \`minimal\` remains the shared-runtime floor. If \`minimal\` stays close to feature canisters, size pressure is coming from shared crates rather than role-specific logic.
- \`root\` is always interpreted as a control-plane outlier because it still carries the root runtime plus the bootstrap \`wasm_store.wasm.gz\` artifact during build.
- Large retained hotspots that repeat across many per-canister Twiggy reports should be treated as shared fan-in pressure in crates such as \`canic-core\`, DTO/serialization glue, logging, metrics, auth, and lifecycle/runtime support.

## Early Warning Signals

| Signal | Status | Evidence |
| --- | --- | --- |
| Minimal floor close to feature canisters | $( if [ "${SHRUNK_WASM_BYTES[minimal]:-0}" -gt 0 ] && [ "${SHRUNK_WASM_BYTES[app]:-0}" -gt 0 ] && awk -v minimal="${SHRUNK_WASM_BYTES[minimal]}" -v app="${SHRUNK_WASM_BYTES[app]}" 'BEGIN { exit !((app - minimal) <= (app * 0.10)) }'; then printf 'WARN'; else printf 'OK'; fi ) | \`minimal\` shrunk wasm = ${SHRUNK_WASM_BYTES[minimal]:-N/A}, \`app\` shrunk wasm = ${SHRUNK_WASM_BYTES[app]:-N/A}. |
| Root control-plane outlier | $( if [ "${SHRUNK_WASM_BYTES[root]:-0}" -gt 0 ]; then printf 'WARN'; else printf 'N/A'; fi ) | \`root\` shrunk wasm = ${SHRUNK_WASM_BYTES[root]:-N/A}. |
| Shrink delta unexpectedly low | $( if [ "${SHRINK_DELTA_BYTES[minimal]:-0}" -le 0 ]; then printf 'WARN'; else printf 'OK'; fi ) | \`minimal\` shrink delta = ${SHRINK_DELTA_BYTES[minimal]:-N/A} bytes. |

## Per-Canister Snapshot

| Canister | Kind | Built wasm | Shrunk wasm | Shrink delta | Built gz | Shrunk gz | Baseline delta | Built funcs | Shrunk funcs | Exports | Detail |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
EOF

    for canister in "${CANISTERS[@]}"; do
        printf '| `%s` | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | [%s.md](%s/%s.md) |\n' \
            "$canister" \
            "${CANISTER_KIND[$canister]}" \
            "${BUILT_WASM_BYTES[$canister]}" \
            "${SHRUNK_WASM_BYTES[$canister]}" \
            "${SHRINK_DELTA_BYTES[$canister]}" \
            "${BUILT_WASM_GZ_BYTES[$canister]}" \
            "${SHRUNK_WASM_GZ_BYTES[$canister]}" \
            "${BASELINE_DELTA_BYTES[$canister]}" \
            "${BUILT_FUNCTIONS[$canister]}" \
            "${SHRUNK_FUNCTIONS[$canister]}" \
            "${SHRUNK_EXPORTS[$canister]}" \
            "$canister" \
            "$ARTIFACT_LINK_PREFIX" \
            "$canister" \
            >>"$report_path"
    done

    cat >>"$report_path" <<EOF

## Risk Score

Risk Score: **$RISK_SCORE / 10**

Interpretation: this is a wasm drift score, not a correctness score. The main drivers in Canic are the shared runtime floor across leaf canisters and the special-case bundle behavior in \`root\`.

## Verification Readout

| Command | Status | Notes |
| --- | --- | --- |
EOF

    printf '%s\n' "${VERIFICATION_ROWS[@]}" >>"$report_path"

    cat >>"$report_path" <<EOF

## Follow-up Actions

EOF

    if [ "$TWIGGY_AVAILABLE" -eq 0 ]; then
        cat >>"$report_path" <<EOF
1. Owner boundary: \`tooling\`
   Action: install \`twiggy\` before the next wasm footprint run so retained-size and monomorphization evidence is not \`BLOCKED\`.
   Target report date/run: \`docs/audits/reports/$MONTH/$RUN_DATE/$AUDIT_SLUG.md\`
EOF
    else
        cat >>"$report_path" <<EOF
1. Owner boundary: \`shared runtime baseline\`
   Action: compare \`minimal\` retained hotspots against one feature canister in the next run and treat overlapping drivers as shared-cost reduction candidates.
   Target report date/run: \`docs/audits/reports/$MONTH/$RUN_DATE/$AUDIT_SLUG.md\`
2. Owner boundary: \`bundle canister root\`
   Action: keep tracking \`root\` separately from leaf canisters so child bundle growth and root-local growth do not get conflated.
   Target report date/run: \`docs/audits/reports/$MONTH/$RUN_DATE/$AUDIT_SLUG.md\`
EOF
    fi

    cat >>"$report_path" <<EOF

## Report Files

- [$SCOPE_STEM.md]($REPORT_LINK_PREFIX/$SCOPE_STEM.md)
- [size-summary.md]($ARTIFACT_LINK_PREFIX/size-summary.md)
- [size-report.json]($ARTIFACT_LINK_PREFIX/size-report.json)
EOF

    for canister in "${CANISTERS[@]}"; do
        printf -- '- [%s.md](%s/%s.md)\n' "$canister" "$ARTIFACT_LINK_PREFIX" "$canister" >>"$report_path"
    done
}

normalize_profile
select_canisters

RUN_DATE="${WASM_AUDIT_DATE:-$(date -u +%F)}"
RUN_TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
MONTH="${RUN_DATE:0:7}"
DAY_DIR="docs/audits/reports/$MONTH/$RUN_DATE"
mkdir -p "$DAY_DIR"

SCOPE_STEM="$(next_scope_stem "$DAY_DIR" "$AUDIT_SLUG")"
REPORT_PATH="$DAY_DIR/$SCOPE_STEM.md"
ARTIFACTS_DIR="$DAY_DIR/artifacts/$SCOPE_STEM"
mkdir -p "$ARTIFACTS_DIR"

BASELINE_PATH="N/A"
if [ "$SCOPE_STEM" != "$AUDIT_SLUG" ]; then
    BASELINE_PATH="docs/audits/reports/$MONTH/$RUN_DATE/$AUDIT_SLUG.md"
fi

REPORT_LINK_PREFIX="."
ARTIFACT_LINK_PREFIX="artifacts/$SCOPE_STEM"

CACHE_ROOT="artifacts/wasm-size/$PROFILE_NAME"
CACHE_RAW_DIR="$CACHE_ROOT/raw"
CACHE_SHRUNK_DIR="$CACHE_ROOT/shrunk"
CACHE_ANALYSIS_DIR="$CACHE_ROOT/analysis"
mkdir -p "$CACHE_ROOT" "$CACHE_RAW_DIR" "$CACHE_SHRUNK_DIR" "$CACHE_ANALYSIS_DIR"

TWIGGY_AVAILABLE=0
IC_WASM_AVAILABLE=0
if has_cmd twiggy; then
    TWIGGY_AVAILABLE=1
fi
if has_cmd ic-wasm; then
    IC_WASM_AVAILABLE=1
fi

BRANCH="$(normalize_branch)"
COMMIT="$(normalize_commit)"
WORKTREE="$(normalize_worktree)"
COMPARABILITY="comparable"

if [ "${WASM_AUDIT_SKIP_BUILD:-0}" = "1" ]; then
    validate_cached_artifacts
    record_verification "WASM_AUDIT_SKIP_BUILD=1 cache reuse" "PASS" "reused cached artifacts from \`$CACHE_ROOT\`"
else
    if ! has_cmd cargo; then
        echo "cargo is required unless WASM_AUDIT_SKIP_BUILD=1" >&2
        exit 1
    fi
    if ! has_cmd dfx; then
        echo "dfx is required unless WASM_AUDIT_SKIP_BUILD=1" >&2
        exit 1
    fi
    ensure_local_dfx_ready
    build_and_cache_artifacts
    record_verification "cargo build --target wasm32-unknown-unknown ... && dfx build ..." "PASS" "built and cached raw/shrunk artifacts for $(profile_command_note)"
fi

if [ "$IC_WASM_AVAILABLE" -eq 1 ]; then
    record_verification "ic-wasm <artifact> info" "PASS" "structure snapshots captured for built and shrunk artifacts"
else
    record_verification "ic-wasm <artifact> info" "BLOCKED" "ic-wasm not installed; structure snapshot metrics unavailable"
fi

if [ "$TWIGGY_AVAILABLE" -eq 1 ]; then
    record_verification "twiggy top|dominators|monos <analysis.wasm>" "PASS" "twiggy artifacts captured for each canister in scope"
else
    record_verification "twiggy top|dominators|monos <analysis.wasm>" "BLOCKED" "twiggy not installed; hotspot attribution unavailable"
fi

SIZE_METRICS_TSV="$ARTIFACTS_DIR/size-metrics.tsv"
{
    printf 'canister\tkind\tbuilt_wasm_bytes\tbuilt_wasm_gz_bytes\tshrunk_wasm_bytes\tshrunk_wasm_gz_bytes\tshrink_delta_bytes\tshrink_delta_percent\tbuilt_functions\tshrunk_functions\tbuilt_exports\tshrunk_exports\n'
} >"$SIZE_METRICS_TSV"

for canister in "${CANISTERS[@]}"; do
    if [ "$canister" = "root" ]; then
        CANISTER_KIND["$canister"]="bundle-canister"
    else
        CANISTER_KIND["$canister"]="leaf-canister"
    fi

    raw_wasm="$CACHE_RAW_DIR/$canister.wasm"
    raw_gz="$CACHE_RAW_DIR/$canister.wasm.gz"
    shrunk_wasm="$CACHE_SHRUNK_DIR/$canister.wasm"
    shrunk_gz="$CACHE_SHRUNK_DIR/$canister.wasm.gz"
    analysis_wasm="$CACHE_ANALYSIS_DIR/$canister.wasm"

    BUILT_WASM_BYTES["$canister"]="$(byte_size "$raw_wasm")"
    BUILT_WASM_GZ_BYTES["$canister"]="$(byte_size "$raw_gz")"
    SHRUNK_WASM_BYTES["$canister"]="$(byte_size "$shrunk_wasm")"
    SHRUNK_WASM_GZ_BYTES["$canister"]="$(byte_size "$shrunk_gz")"
    SHRINK_DELTA_BYTES["$canister"]="$(( ${BUILT_WASM_BYTES[$canister]} - ${SHRUNK_WASM_BYTES[$canister]} ))"
    SHRINK_DELTA_PCT["$canister"]="$(normalize_delta_pct "${BUILT_WASM_BYTES[$canister]}" "${SHRUNK_WASM_BYTES[$canister]}")"

    built_info_out="$ARTIFACTS_DIR/$canister.built.ic-wasm-info.txt"
    shrunk_info_out="$ARTIFACTS_DIR/$canister.shrunk.ic-wasm-info.txt"

    if [ "$IC_WASM_AVAILABLE" -eq 1 ]; then
        ic-wasm "$raw_wasm" info >"$built_info_out"
        ic-wasm "$shrunk_wasm" info >"$shrunk_info_out"
        capture_info_metrics "$canister" built "$built_info_out"
        capture_info_metrics "$canister" shrunk "$shrunk_info_out"
    else
        printf 'BLOCKED: ic-wasm not installed\n' >"$built_info_out"
        printf 'BLOCKED: ic-wasm not installed\n' >"$shrunk_info_out"
        BUILT_FUNCTIONS["$canister"]="0"
        BUILT_DATA_SECTIONS["$canister"]="0"
        BUILT_DATA_BYTES["$canister"]="0"
        BUILT_EXPORTS["$canister"]="0"
        SHRUNK_FUNCTIONS["$canister"]="0"
        SHRUNK_DATA_SECTIONS["$canister"]="0"
        SHRUNK_DATA_BYTES["$canister"]="0"
        SHRUNK_EXPORTS["$canister"]="0"
    fi

    top_txt="$ARTIFACTS_DIR/$canister.twiggy-top.txt"
    top_csv="$ARTIFACTS_DIR/$canister.twiggy-top.csv"
    retained_csv="$ARTIFACTS_DIR/$canister.twiggy-retained.csv"
    dominators_txt="$ARTIFACTS_DIR/$canister.twiggy-dominators.txt"
    monos_txt="$ARTIFACTS_DIR/$canister.twiggy-monos.txt"

    if [ "$TWIGGY_AVAILABLE" -eq 1 ]; then
        twiggy top -n 40 "$analysis_wasm" >"$top_txt"
        twiggy top -n 40 -f csv "$analysis_wasm" >"$top_csv"
        twiggy top --retained -n 40 -f csv "$analysis_wasm" >"$retained_csv"
        twiggy dominators -d 8 -r 60 "$analysis_wasm" >"$dominators_txt"
        twiggy monos -m 20 -n 20 "$analysis_wasm" >"$monos_txt"
        capture_hotspot_summary "$canister" "$top_csv" "$retained_csv"
    else
        printf 'BLOCKED: twiggy not installed\n' >"$top_txt"
        printf 'Name,ShallowSize,ShallowSizePercent,RetainedSize,RetainedSizePercent\n' >"$top_csv"
        printf 'Name,ShallowSize,ShallowSizePercent,RetainedSize,RetainedSizePercent\n' >"$retained_csv"
        printf 'BLOCKED: twiggy not installed\n' >"$dominators_txt"
        printf 'BLOCKED: twiggy not installed\n' >"$monos_txt"
        HOTSPOT_NAME["$canister"]="N/A"
        HOTSPOT_SHALLOW["$canister"]="N/A"
        HOTSPOT_RETAINED["$canister"]="N/A"
    fi

    BASELINE_DELTA_BYTES["$canister"]="N/A"
    BASELINE_DELTA_PCT["$canister"]="N/A"

    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$canister" \
        "${CANISTER_KIND[$canister]}" \
        "${BUILT_WASM_BYTES[$canister]}" \
        "${BUILT_WASM_GZ_BYTES[$canister]}" \
        "${SHRUNK_WASM_BYTES[$canister]}" \
        "${SHRUNK_WASM_GZ_BYTES[$canister]}" \
        "${SHRINK_DELTA_BYTES[$canister]}" \
        "${SHRINK_DELTA_PCT[$canister]}" \
        "${BUILT_FUNCTIONS[$canister]}" \
        "${SHRUNK_FUNCTIONS[$canister]}" \
        "${BUILT_EXPORTS[$canister]}" \
        "${SHRUNK_EXPORTS[$canister]}" \
        >>"$SIZE_METRICS_TSV"

done

if [ "$BASELINE_PATH" != "N/A" ]; then
    BASELINE_SCOPE_STEM="$(basename "$BASELINE_PATH" .md)"
    BASELINE_TSV="$DAY_DIR/artifacts/$BASELINE_SCOPE_STEM/size-metrics.tsv"
    if [ -f "$BASELINE_TSV" ]; then
        record_verification "baseline size-metrics.tsv comparison" "PASS" "baseline deltas calculated from \`$BASELINE_TSV\`"
        for canister in "${CANISTERS[@]}"; do
            baseline_shrunk="$(awk -F'\t' -v c="$canister" 'NR > 1 && $1 == c {print $5}' "$BASELINE_TSV")"
            if [ -n "$baseline_shrunk" ]; then
                BASELINE_DELTA_BYTES["$canister"]="$(( ${SHRUNK_WASM_BYTES[$canister]} - baseline_shrunk ))"
                BASELINE_DELTA_PCT["$canister"]="$(normalize_baseline_delta_pct "$baseline_shrunk" "${SHRUNK_WASM_BYTES[$canister]}")"
            fi
        done
    else
        COMPARABILITY="non-comparable"
        record_verification "baseline size-metrics.tsv comparison" "BLOCKED" "baseline artifact table not found at \`$BASELINE_TSV\`"
    fi
else
    record_verification "baseline size-metrics.tsv comparison" "BLOCKED" "first run of day; no baseline comparison available"
fi

write_aggregate_json "$ARTIFACTS_DIR/size-report.json"
run_top_summary "$ARTIFACTS_DIR/size-summary.md"

for canister in "${CANISTERS[@]}"; do
    write_per_canister_json "$canister" "$ARTIFACTS_DIR/$canister.size-report.json"
    write_per_canister_markdown "$canister" "$ARTIFACTS_DIR/$canister.md"
done

determine_risk_score
render_report "$REPORT_PATH"

printf 'wrote %s\n' "$REPORT_PATH"
