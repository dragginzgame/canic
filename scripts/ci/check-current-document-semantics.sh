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

for operator_doc in "${operator_docs[@]}"; do
    if rg -ni \
        '(current|in-progress).*0\.100|0\.100 (implementation|installer)' \
        "$operator_doc" >/dev/null; then
        echo "current operator document reintroduces a stale 0.100 boundary: $(guard_path "$operator_doc")" >&2
        exit 1
    fi
    forbid_texts "$operator_doc" "$GUARD_LABEL" \
        "canic --environment local fleet list" \
        "stops before Component creation" \
        "does not yet create the"
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
