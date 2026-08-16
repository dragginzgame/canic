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
    "${operator_docs[@]}"

status_line_count="$(wc -l <"$STATUS")"
[ "$status_line_count" -le 200 ] || {
    echo "current status is no longer a compact handoff: $status_line_count lines" >&2
    exit 1
}

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

historical_backlog="$ROOT/docs/design/archive/post-46-backlog"
[ "$(find "$historical_backlog" -type f | wc -l)" -le 11 ] || {
    echo "historical post-46 design collection has grown" >&2
    exit 1
}
[ "$(find "$historical_backlog" -mindepth 1 -maxdepth 1 -type d | wc -l)" -le 5 ] || {
    echo "historical post-46 design topics have grown" >&2
    exit 1
}

design_ideas="$ROOT/docs/design/ideas"
[ "$(find "$design_ideas" -type f ! -path "$design_ideas/saltz/*" | wc -l)" -le 16 ] || {
    echo "optional design-idea collection has grown without explicit approval" >&2
    exit 1
}
[ "$(find "$design_ideas" -mindepth 1 -maxdepth 1 -type d ! -name saltz | wc -l)" -le 9 ] || {
    echo "optional design-idea topics have grown without explicit approval" >&2
    exit 1
}

for idea_dir in "$design_ideas"/*; do
    [ -d "$idea_dir" ] || continue
    [ "${idea_dir##*/}" != "saltz" ] || continue

    max_idea_files=1
    case "${idea_dir##*/}" in
        coordinator-workers|cross-subnet-data-transport-groundwork|declarative-authentication-profiles|optional-encrypted-canister-snapshot-archives|standalone-blob-service-extraction)
            ;;
        framework-neutral-synchronous-lifecycle-composition|language-neutral-managed-guest-feasibility)
            max_idea_files=2
            ;;
        *)
            echo "unapproved optional design-idea topic: $(guard_path "$idea_dir")" >&2
            exit 1
            ;;
    esac

    [ "$(find "$idea_dir" -type f | wc -l)" -le "$max_idea_files" ] || {
        echo "optional design-idea topic exceeds its approved file boundary: $(guard_path "$idea_dir")" >&2
        exit 1
    }

    [ -f "$idea_dir/design.md" ] || {
        echo "optional design-idea topic is missing design.md: $(guard_path "$idea_dir")" >&2
        exit 1
    }
    while IFS= read -r idea_file; do
        case "${idea_file##*/}" in
            design.md | exploration.md | status.md) ;;
            *)
                echo "optional design-idea topic has an unsupported file: $(guard_path "$idea_file")" >&2
                exit 1
                ;;
        esac
    done < <(find "$idea_dir" -maxdepth 1 -type f | sort)
done

saltz_idea="$design_ideas/saltz"
saltz_waveform="$saltz_idea/saltz_24h_waveform_floor_100B_860.csv"
saltz_waveform_sha256="11fd75eb8fd0fed4f075d324051cc880db50619837bfe6c889fe9d654647d911"
[ "$(find "$saltz_idea" -type f | wc -l)" -eq 2 ] || {
    echo "Saltz idea must retain exactly its design and numeric waveform: $(guard_path "$saltz_idea")" >&2
    exit 1
}
require_files "$GUARD_LABEL" \
    "$saltz_idea/design.md" \
    "$saltz_waveform"
require_text "$saltz_idea/design.md" \
    "$saltz_waveform_sha256" \
    "$GUARD_LABEL"
bash "$ROOT/scripts/ci/verify-file-checksum.sh" \
    sha256 \
    "$saltz_waveform_sha256" \
    "$saltz_waveform"
[ ! -d "$ROOT/apps/saltz/profile_compiler" ] || {
    echo "removed Saltz image-authoring package has returned" >&2
    exit 1
}
saltz_image_file="$(find "$saltz_idea" "$ROOT/apps/saltz" -type f \( \
    -iname '*.avif' -o \
    -iname '*.gif' -o \
    -iname '*.jpeg' -o \
    -iname '*.jpg' -o \
    -iname '*.png' -o \
    -iname '*.svg' -o \
    -iname '*.webp' \
    \) -print -quit)"
[ -z "$saltz_image_file" ] || {
    echo "Saltz source or raster image has returned: $(guard_path "$saltz_image_file")" >&2
    exit 1
}

for evidence_root in \
    "$ROOT/docs/audits/working" \
    "$ROOT/docs/audits/release-lines/supporting"; do
    [ -d "$evidence_root" ] || continue

    loose_evidence_count="$(find "$evidence_root" -maxdepth 1 -type f | wc -l)"
    [ "$loose_evidence_count" -eq 0 ] || {
        echo "audit evidence must live in one bounded topic directory: $(guard_path "$evidence_root")" >&2
        exit 1
    }

    for evidence_dir in "$evidence_root"/*; do
        [ -d "$evidence_dir" ] || continue

        max_evidence_files=8
        case "${evidence_dir#"$ROOT"/}" in
            docs/audits/working/0.102-diagnostic-inventory)
                max_evidence_files=66
                ;;
            docs/audits/release-lines/supporting/0.82-boundary-hardening)
                max_evidence_files=73
                ;;
        esac

        evidence_file_count="$(find "$evidence_dir" -type f | wc -l)"
        [ "$evidence_file_count" -le "$max_evidence_files" ] || {
            echo "audit evidence bundle exceeds its file boundary: $(guard_path "$evidence_dir")" >&2
            exit 1
        }
    done
done

workspace_version="$(
    sed -n 's/^version = "\([^"]*\)"$/\1/p' "$ROOT/Cargo.toml" | sed -n '1p'
)"
[ -n "$workspace_version" ] || {
    echo "unable to derive the workspace package version" >&2
    exit 1
}

require_texts "$STATUS" "$GUARD_LABEL" \
    "Workspace package version: \`$workspace_version\`." \
    "## Current Decision" \
    "## Next Action"

forbid_text "$STATUS" "## Historical Release Detail" "$GUARD_LABEL"

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
