#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=./scripts/ci/doc-guard-lib.sh
source "$ROOT/scripts/ci/doc-guard-lib.sh"

GUARD_LABEL="current document semantics"
STATUS="$ROOT/docs/status/current.md"
STATUS_ARCHIVE="$ROOT/docs/status/archive/2026-08-12-precompact.md"
AGENTS="$ROOT/AGENTS.md"
CI_GOVERNANCE="$ROOT/docs/governance/ci-deployment.md"
ARCHITECTURE="$ROOT/docs/contracts/ARCHITECTURE.md"
HYGIENE="$ROOT/docs/governance/code-hygiene/README.md"
AUTH_DESIGN="$ROOT/docs/architecture/authentication.md"
AUTH_CONTRACT="$ROOT/docs/contracts/AUTH_DELEGATED_SIGNATURES.md"
TIMER_DESIGN="$ROOT/docs/design/0.104-ic-timers-consumer-hard-cut/0.104-design.md"
TIMER_STATUS="$ROOT/docs/design/0.104-ic-timers-consumer-hard-cut/status.md"
TIMER_EVIDENCE="$ROOT/docs/audits/working/0.104-timer-ownership/README.md"
TIMER_CHANGELOG="$ROOT/docs/changelog/0.104.md"
TIMER_GUIDE="$ROOT/docs/features/runtime/native-timers.md"
ADMISSION_DESIGN="$ROOT/docs/design/0.109-fleet-wide-ingress-admission/0.109-design.md"
ADMISSION_STATUS="$ROOT/docs/design/0.109-fleet-wide-ingress-admission/status.md"
ESTATE_DESIGN="$ROOT/docs/design/0.110-fleet-subnet-canister-estates/0.110-design.md"
ESTATE_STATUS="$ROOT/docs/design/0.110-fleet-subnet-canister-estates/status.md"
COMPLEXITY_AUDIT="$ROOT/docs/audits/release-lines/0.109-post-implementation-complexity-audit.md"

operator_docs=(
    "$ROOT/INSTALLING.md"
    "$ROOT/apps/README.md"
    "$ROOT/scripts/app/README.md"
    "$ROOT/crates/canic-cli/README.md"
    "$ROOT/crates/canic-host/README.md"
    "$ROOT/docs/architecture/fleet-install-input.md"
    "$ROOT/docs/getting-started/local-academic-fleet.md"
    "$ROOT/docs/getting-started/minimal-managed-fleet.md"
)

require_files "$GUARD_LABEL" \
    "$STATUS" \
    "$STATUS_ARCHIVE" \
    "$AGENTS" \
    "$CI_GOVERNANCE" \
    "$ARCHITECTURE" \
    "$HYGIENE" \
    "$AUTH_DESIGN" \
    "$AUTH_CONTRACT" \
    "$TIMER_DESIGN" \
    "$TIMER_STATUS" \
    "$TIMER_EVIDENCE" \
    "$TIMER_CHANGELOG" \
    "$TIMER_GUIDE" \
    "$ADMISSION_DESIGN" \
    "$ADMISSION_STATUS" \
    "$ESTATE_DESIGN" \
    "$ESTATE_STATUS" \
    "$COMPLEXITY_AUDIT" \
    "${operator_docs[@]}"

for design_entry in "$ROOT"/docs/design/*; do
    [ -d "$design_entry" ] || continue
    case "$(basename "$design_entry")" in
        0.* | archive | ideas) ;;
        *)
            echo "unexpected top-level design collection: $(guard_path "$design_entry")" >&2
            exit 1
            ;;
    esac
done

for archive_entry in "$ROOT"/docs/design/archive/*; do
    [ -d "$archive_entry" ] || continue
    case "$(basename "$archive_entry")" in
        0.* | post-46-backlog) ;;
        *)
            echo "unexpected archived design collection: $(guard_path "$archive_entry")" >&2
            exit 1
            ;;
    esac
done

for design_dir in "$ROOT"/docs/design/0.* "$ROOT"/docs/design/archive/0.*; do
    [ -d "$design_dir" ] || continue

    max_files=2
    if [ "${design_dir#"$ROOT"/}" = \
        "docs/design/0.102-compact-diagnostic-codes" ]; then
        max_files=4
        require_files "$GUARD_LABEL" \
            "$design_dir/0.102-design.md" \
            "$design_dir/status.md" \
            "$design_dir/allocation-proposal.md" \
            "$design_dir/code-allocation-ledger.md"
    fi

    if [[ "$design_dir" == "$ROOT/docs/design/0."* ]]; then
        design_line="$(basename "$design_dir")"
        design_line="${design_line%%-*}"
        require_files "$GUARD_LABEL" \
            "$design_dir/$design_line-design.md" \
            "$design_dir/status.md"
    fi

    design_file_count="$(find "$design_dir" -maxdepth 1 -type f | wc -l)"
    design_subdir_count="$(find "$design_dir" -mindepth 1 -maxdepth 1 -type d | wc -l)"
    [ "$design_file_count" -le "$max_files" ] || {
        echo "design directory exceeds its compact file boundary: $(guard_path "$design_dir")" >&2
        exit 1
    }
    [ "$design_subdir_count" -eq 0 ] || {
        echo "numbered design directory contains a nested evidence directory: $(guard_path "$design_dir")" >&2
        exit 1
    }

    while IFS= read -r design_file; do
        case "$(basename "$design_file")" in
            *design.md | status.md | *-status.md) ;;
            allocation-proposal.md | code-allocation-ledger.md)
                [ "$max_files" -eq 4 ] || {
                    echo "unexpected design-directory authority: $(guard_path "$design_file")" >&2
                    exit 1
                }
                ;;
            *)
                echo "unexpected design-directory file: $(guard_path "$design_file")" >&2
                exit 1
                ;;
        esac
    done < <(find "$design_dir" -maxdepth 1 -type f | sort)
done

design_ideas="$ROOT/docs/design/ideas"
idea_index="$design_ideas/README.md"
require_files "$GUARD_LABEL" "$idea_index"

actual_idea_topics="$(
    find "$design_ideas" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort
)"
indexed_idea_topics="$(
    sed -n '/^## Current Topics$/,/^## /p' "$idea_index" |
        sed -n 's/^- `\([^`]*\)\/`$/\1/p' |
        sort
)"
if [[ "$actual_idea_topics" != "$indexed_idea_topics" ]]; then
    echo "design-idea index does not match its topic directories" >&2
    printf 'indexed:\n%s\nactual:\n%s\n' \
        "$indexed_idea_topics" "$actual_idea_topics" >&2
    exit 1
fi

for idea_dir in "$design_ideas"/*; do
    [ -d "$idea_dir" ] || continue

    [ -f "$idea_dir/design.md" ] || {
        echo "optional design-idea topic is missing design.md: $(guard_path "$idea_dir")" >&2
        exit 1
    }
    while IFS= read -r idea_file; do
        case "${idea_file##*/}" in
            design.md | exploration.md | status.md) ;;
            *)
                echo "optional design-idea topic has an unsupported authority document: $(guard_path "$idea_file")" >&2
                exit 1
                ;;
        esac
    done < <(find "$idea_dir" -maxdepth 1 -type f -name '*.md' | sort)
done

require_texts "$STATUS" "$GUARD_LABEL" \
    "## Current Decision" \
    "## Next Action"
require_texts "$COMPLEXITY_AUDIT" "$GUARD_LABEL" \
    "closeout_verdict: fail" \
    "CANIC-109-GOLIVE-001" \
    "CANIC-109-GOLIVE-002" \
    "CANIC-109-GOLIVE-003" \
    "B9 remediation must not begin" \
    "until B8 closes." \
    "Until then, 0.110 is not authorized."
require_texts "$ADMISSION_STATUS" "$GUARD_LABEL" \
    "../../audits/release-lines/0.109-post-implementation-complexity-audit.md" \
    "| B8 | Release and downstream go-live support |" \
    "| B9 | Post-adoption complexity contraction |" \
    "CANIC-109-GOLIVE-001" \
    "CANIC-109-GOLIVE-002" \
    "CANIC-109-GOLIVE-003"
require_texts "$ADMISSION_DESIGN" "$GUARD_LABEL" \
    "| B8 | Release and downstream go-live support |" \
    "| B9 | Post-adoption complexity contraction |" \
    '`CANIC-109-GOLIVE-001` through `003`'
require_texts "$ESTATE_STATUS" "$GUARD_LABEL" \
    "../../audits/release-lines/0.109-post-implementation-complexity-audit.md" \
    "No 0.110 mutation is authorized."
require_text "$ESTATE_DESIGN" \
    "../../audits/release-lines/0.109-post-implementation-complexity-audit.md" \
    "$GUARD_LABEL"
require_texts "$STATUS" "$GUARD_LABEL" \
    "binding post-implementation complexity audit" \
    "B8 owns Canic release and downstream" \
    "B9 owns"

forbid_text "$STATUS" "## Historical Release Detail" "$GUARD_LABEL"
forbid_texts "$STATUS" "$GUARD_LABEL" \
    "latest published release" \
    "latest published package" \
    "Release-truth warning:" \
    "not published or package-versioned"

forbid_texts "$TIMER_DESIGN" "$GUARD_LABEL" \
    "B7 is authorized" \
    "B8 remains blocked" \
    "Blocked on B7"
forbid_texts "$TIMER_STATUS" "$GUARD_LABEL" \
    "Keep the open 0.104.0" \
    "open 0.104.1 changelog" \
    "only remaining action is maintainer closeout review" \
    'Published `v0.104.1` contains that correction and closes the line.' \
    "not part of either published tag" \
    'bounded `0.104.2` corrective candidate' \
    "1.1750% raw" \
    "1.9779% gzip"
require_texts "$TIMER_STATUS" "$GUARD_LABEL" \
    '`v0.104.2` freezes exact registration actions' \
    "No 0.104 implementation work remains." \
    "not canonical release-identity evidence"
forbid_texts "$STATUS" "$GUARD_LABEL" \
    "Keep the open 0.104.0" \
    "bounded closeout correction are complete" \
    'open `0.104.2` corrective candidate' \
    "Existing B3 work is paused" \
    "1.1750% raw" \
    "1.9779% gzip"
require_texts "$STATUS" "$GUARD_LABEL" \
    'Published `v0.104.2` now freezes native registration actions' \
    "B2-B7 are complete" \
    "the 0.105 implementation batch is closed"
forbid_texts "$STATUS" "$GUARD_LABEL" \
    "B7 is active" \
    "B7 may proceed" \
    "final Canic-only recovery and residue qualification remains"
require_texts "$TIMER_EVIDENCE" "$GUARD_LABEL" \
    "not acceptance evidence" \
    "19,424,848" \
    "19,124,317" \
    "19,424,589" \
    "19,123,930" \
    "19,123,917" \
    "not canonical release-identity evidence" \
    "controlled causal percentage"
require_texts "$TIMER_GUIDE" "$GUARD_LABEL" \
    "Provider inventory is volatile." \
    "Provider inventory is not durable business demand." \
    "Provider inventory is not an application recovery record."
require_text "$TIMER_CHANGELOG" \
    "## [0.104.2] - 2026-08-19 - Closeout Audit Correction" \
    "$GUARD_LABEL"
require_text "$TIMER_CHANGELOG" \
    "## [0.104.1] - 2026-08-19 - Closeout Evidence Correction" \
    "$GUARD_LABEL"

for detailed_changelog in "$ROOT"/docs/changelog/*.md; do
    forbid_text "$detailed_changelog" "Release truth:" "$GUARD_LABEL"
done

for layer_doc in "$AGENTS" "$ARCHITECTURE" "$HYGIENE"; do
    forbid_text "$layer_doc" \
        "endpoints -> workflow -> policy -> ops -> model" \
        "$GUARD_LABEL"
done
require_texts "$AGENTS" "$GUARD_LABEL" \
    "workflow may call" \
    "Policy never calls ops."
require_text "$ARCHITECTURE" "policy does not call ops." "$GUARD_LABEL"
require_text "$HYGIENE" "Policy never calls ops." "$GUARD_LABEL"

require_text "$CI_GOVERNANCE" \
    "Automated agents must never change release version numbers directly." \
    "$GUARD_LABEL"
forbid_text "$AGENTS" \
    "unless the maintainer explicitly asks for a version bump" \
    "$GUARD_LABEL"

if rg -ni '\bproject\b' "$ROOT/docs/governance/code-hygiene/example-crate/src" "$HYGIENE" >/dev/null; then
    echo "code-hygiene examples reintroduce Project as a Canic-owned identity" >&2
    exit 1
fi
forbid_texts "$AUTH_DESIGN" "$GUARD_LABEL" \
    "local project id" \
    "local project accepts" \
    "local project does not accept"
forbid_text "$AUTH_CONTRACT" "local project" "$GUARD_LABEL"

echo "current document semantics guard passed"
