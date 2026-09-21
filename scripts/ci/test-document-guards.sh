#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-document-guards.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT

# Copy only the recorded audit inputs and the release-document guard's inputs.
mapfile -t inputs < <(
    awk -F '|' '
        NF == 6 && $2 ~ /CANIC-/ { path = $5 }
        NF == 4 && $2 ~ /[0-9a-f]/ { path = $3 }
        path != "" {
            gsub(/^[[:space:]]*`?|`?[[:space:]]*$/, "", path)
            if (path ~ /^(docs|scripts|crates)\//) print path
            path = ""
        }
    ' "$ROOT/docs/audits/method-fingerprints-v1.md" | sort -u
)
inputs+=(
    docs/audits/method-fingerprints-v1.md
    docs/operations/release-validation-matrix.md
    docs/operations/release-package-install-validation.md
    docs/operations/README.md
    docs/governance/ci-deployment.md
    scripts/ci/check-release-validation-matrix.sh
    scripts/ci/doc-guard-lib.sh
    scripts/ci/verify-packaged-downstream-wasm-store.sh
    Makefile
)
for path in "${inputs[@]}"; do
    mkdir -p "$fixture/$(dirname "$path")"
    cp "$ROOT/$path" "$fixture/$path"
done

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

refresh_fingerprint_hash() {
    local path="$1"
    local hash
    hash="$(sha256sum "$fixture/$path" | awk '{print $1}')"
    awk -F '|' -v OFS='|' -v path="$path" -v hash="$hash" '
        NF == 4 || NF == 6 {
            path_column = NF - 1
            hash_column = NF - 2
            candidate = $path_column
            gsub(/^[[:space:]]*`?|`?[[:space:]]*$/, "", candidate)
            if (candidate == path) $hash_column = " `" hash "` "
        }
        { print }
    ' "$fixture/docs/audits/method-fingerprints-v1.md" >"$fixture/updated"
    mv "$fixture/updated" "$fixture/docs/audits/method-fingerprints-v1.md"
}

expect_pass check-audit-method-catalog.sh
expect_pass check-release-validation-matrix.sh

# Reviewed editorial edits can change headings, table spacing and comments.
for path in docs/audits/METHODS.md docs/audits/META-AUDIT.md; do
    sed -i 's/^## .*/## Revised editorial title/' "$fixture/$path"
    refresh_fingerprint_hash "$path"
done
sed -i -e 's/^## .*/## Rearranged ledger/' -e 's/ | /  |  /g' \
    "$fixture/docs/audits/method-fingerprints-v1.md"
expect_pass check-audit-method-catalog.sh

# Actual command execution belongs to the packaged proof, not source-text grep.
printf '#!/usr/bin/env bash\n# Command implementation may be refactored.\n' \
    >"$fixture/scripts/ci/verify-packaged-downstream-wasm-store.sh"
expect_pass check-release-validation-matrix.sh

printf '\n# unreviewed change\n' >>"$fixture/scripts/ci/wasm-audit-report.sh"
expect_failure check-audit-method-catalog.sh 'stale executable fingerprint'
refresh_fingerprint_hash scripts/ci/wasm-audit-report.sh
expect_pass check-audit-method-catalog.sh

cp "$fixture/docs/audits/method-fingerprints-v1.md" "$fixture/ledger-backup"
printf '| `%064d` | `scripts/ci/wasm-audit-report.sh` |\n' 0 \
    >>"$fixture/docs/audits/method-fingerprints-v1.md"
expect_failure check-audit-method-catalog.sh 'conflicting executable fingerprints'
mv "$fixture/ledger-backup" "$fixture/docs/audits/method-fingerprints-v1.md"
expect_pass check-audit-method-catalog.sh

definition=docs/audits/recurring/invariants/canonical-auth-boundary.md
cp "$fixture/$definition" "$fixture/definition-backup"
printf '\n- Audit ID: `CANIC-DUPLICATE-001`\n' >>"$fixture/$definition"
# Keep the hash valid so this case isolates identity validation.
refresh_fingerprint_hash "$definition"
expect_failure check-audit-method-catalog.sh 'duplicate method identity field'
mv "$fixture/definition-backup" "$fixture/$definition"
refresh_fingerprint_hash "$definition"
expect_pass check-audit-method-catalog.sh

cp "$fixture/docs/audits/METHODS.md" "$fixture/catalog-backup"
sed -i '/canonical-auth-boundary[.]md/d' "$fixture/docs/audits/METHODS.md"
refresh_fingerprint_hash docs/audits/METHODS.md
expect_failure check-audit-method-catalog.sh 'missing catalog registration'
mv "$fixture/catalog-backup" "$fixture/docs/audits/METHODS.md"
refresh_fingerprint_hash docs/audits/METHODS.md
expect_pass check-audit-method-catalog.sh

trace=docs/audits/mandatory-trace-protocol.md
cp "$fixture/$trace" "$fixture/trace-backup"
awk -F '|' '$2 ~ /TRACE-/ { print; exit }' "$fixture/trace-backup" >>"$fixture/$trace"
refresh_fingerprint_hash "$trace"
expect_failure check-audit-method-catalog.sh 'duplicate trace identity'
mv "$fixture/trace-backup" "$fixture/$trace"
refresh_fingerprint_hash "$trace"
expect_pass check-audit-method-catalog.sh

rm "$fixture/$definition"
expect_failure check-audit-method-catalog.sh 'missing active definition'
sed -i '/release-package-install-validation[.]md/d' "$fixture/docs/operations/README.md"
expect_failure check-release-validation-matrix.sh 'missing operator link'

echo "document guard behavior tests passed"
