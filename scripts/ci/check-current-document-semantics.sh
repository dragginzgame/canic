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
CONTRACTION_DESIGN="$ROOT/docs/design/0.110-fleet-runtime-contraction/0.110-design.md"
CONTRACTION_STATUS="$ROOT/docs/design/0.110-fleet-runtime-contraction/status.md"
COMPLEXITY_AUDIT="$ROOT/docs/audits/release-lines/0.109-post-implementation-complexity-audit.md"

operator_docs=(
    "$ROOT/INSTALLING.md"
    "$ROOT/apps/README.md"
    "$ROOT/scripts/app/README.md"
    "$ROOT/crates/canic-cli/README.md"
    "$ROOT/crates/canic-host/README.md"
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
    "$CONTRACTION_DESIGN" \
    "$CONTRACTION_STATUS" \
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
    sed -n 's/^- `\([^`]*\)\/`$/\1/p' "$idea_index" |
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

echo "current document semantics guard passed"
