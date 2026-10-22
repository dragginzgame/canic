#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-current-document-guard.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT

expect_pass() {
    bash "$fixture/scripts/ci/$1" >"$fixture/output" 2>&1 || {
        cat "$fixture/output" >&2
        exit 1
    }
}

expect_failure() {
    local status=0
    bash "$fixture/scripts/ci/$1" >"$fixture/output" 2>&1 || status=$?
    if [ "$status" -ne 1 ]; then
        echo "expected guard rejection for $2; got exit $status" >&2
        cat "$fixture/output" >&2
        exit 1
    fi
}

# Exercise the current-document inventory with a find implementation that rejects
# GNU formatting, while preserving real traversal failures and advisory mismatch.
current_inputs=(
    scripts/ci/doc-guard-lib.sh docs/governance/ci-deployment.md
    AGENTS.md INSTALLING.md apps/README.md scripts/app/README.md
    crates/canic-cli/README.md crates/canic-host/README.md
    docs/status/current.md docs/contracts/ARCHITECTURE.md
    docs/governance/code-hygiene/README.md docs/architecture/authentication.md
    docs/contracts/AUTH_DELEGATED_SIGNATURES.md docs/features/runtime/native-timers.md
    docs/getting-started/local-academic-fleet.md
    docs/getting-started/minimal-managed-fleet.md
    docs/design/0.109-fleet-wide-ingress-admission/0.109-design.md
    docs/design/0.109-fleet-wide-ingress-admission/status.md
    docs/design/0.110-fleet-runtime-contraction/0.110-design.md
    docs/design/0.110-fleet-runtime-contraction/status.md
    scripts/ci/check-current-document-semantics.sh
)
for path in "${current_inputs[@]}"; do
    mkdir -p "$fixture/$(dirname "$path")"
    cp "$ROOT/$path" "$fixture/$path"
done
mkdir -p "$fixture/bin" "$fixture/docs/design/ideas"
export DOCUMENT_GUARD_REAL_FIND
DOCUMENT_GUARD_REAL_FIND="$(command -v find)"
cat >"$fixture/bin/find" <<'FIND'
#!/usr/bin/env bash
set -euo pipefail
for argument in "$@"; do
    if [[ "$argument" == -printf ]]; then exit 2; fi
done
if [[ "${DOCUMENT_GUARD_FAIL_FIND:-0}" == 1 && "$1" == */docs/design/ideas ]]; then
    exit 1
fi
exec "$DOCUMENT_GUARD_REAL_FIND" "$@"
FIND
chmod +x "$fixture/bin/find"
export PATH="$fixture/bin:$PATH"
: >"$fixture/docs/design/ideas/README.md"
expect_pass check-current-document-semantics.sh
for topic in 'space topic' alpha .hidden; do
    mkdir -p "$fixture/docs/design/ideas/$topic"
    : >"$fixture/docs/design/ideas/$topic/design.md"
    printf -- '- `%s/`\n' "$topic" >>"$fixture/docs/design/ideas/README.md"
done
expect_pass check-current-document-semantics.sh
if grep -q 'design-idea index does not match' "$fixture/output"; then
    cat "$fixture/output" >&2
    exit 1
fi
: >"$fixture/docs/design/ideas/README.md"
expect_pass check-current-document-semantics.sh
# Inventory differences remain advisory; actual discovery failure must fail.
DOCUMENT_GUARD_FAIL_FIND=1 expect_failure check-current-document-semantics.sh 'failed idea discovery'

echo "current document inventory tests passed"
