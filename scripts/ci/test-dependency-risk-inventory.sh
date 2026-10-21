#!/usr/bin/env bash
set -euo pipefail

# shellcheck source=scripts/ci/require-jq.sh
source "$(dirname "${BASH_SOURCE[0]}")/../ci/require-jq.sh"

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

fail() {
    echo "dependency risk gate test failed: $1" >&2
    exit 1
}

classification_only=0
# The focused lane checks classification without creating a Git database fixture.
case "$#:${1:-}" in
0:) ;;
1:--classification-only) classification_only=1 ;;
*) fail "usage: $0 [--classification-only]" ;;
esac

command -v git >/dev/null 2>&1 || fail "git is unavailable"
require_jq
mkdir -p "$ROOT/.tmp"
tmp_dir="$(mktemp -d "$ROOT/.tmp/dependency-risk-test.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT
base="$tmp_dir/base.json"
audit_db="$tmp_dir/advisory-db"
fixture="$tmp_dir/workspace"
GATE="$fixture/scripts/ci/check-dependency-risk-inventory.sh"

# Exercise the real gate against a complete local graph. The separate production
# gate audits Canic's actual lockfile and current advisories; these classification
# tests must not download another database or depend on today's warning inventory.
mkdir -p "$fixture/scripts/ci" "$fixture/app/src" \
    "$fixture/direct/src" "$fixture/transitive/src"
cp "$ROOT/scripts/ci/check-dependency-risk-inventory.sh" "$GATE"
cp "$ROOT/scripts/ci/require-jq.sh" "$ROOT/scripts/ci/prepare-rustsec-db.sh" \
    "$ROOT/scripts/ci/verify-file-checksum.sh" "$fixture/scripts/ci/"
cp "$ROOT/tool-versions.env" "$fixture/tool-versions.env"
cat >"$fixture/Cargo.toml" <<'TOML'
[workspace]
resolver = "3"
members = ["app"]
exclude = ["direct", "transitive"]
TOML
cat >"$fixture/app/Cargo.toml" <<'TOML'
[package]
name = "canic-risk-gate-fixture"
version = "1.0.0"
edition = "2024"
[dependencies]
serde = { path = "../direct" }
TOML
cat >"$fixture/direct/Cargo.toml" <<'TOML'
[package]
name = "serde"
version = "1.0.0"
edition = "2024"
[dependencies]
canic-risk-transitive-fixture = { path = "../transitive" }
TOML
cat >"$fixture/transitive/Cargo.toml" <<'TOML'
[package]
name = "canic-risk-transitive-fixture"
version = "1.0.0"
edition = "2024"
TOML
touch "$fixture/app/src/lib.rs" "$fixture/direct/src/lib.rs" "$fixture/transitive/src/lib.rs"
cargo generate-lockfile --offline --manifest-path "$fixture/Cargo.toml"
checksum="$(bash "$ROOT/scripts/ci/verify-file-checksum.sh" --print sha256 "$fixture/transitive/Cargo.toml")"
printf 'RUSTSEC-2099-0001\tunmaintained\tcanic-risk-transitive-fixture\t1.0.0\t%s\tserde\n' \
    "$checksum" >"$fixture/scripts/ci/dependency-risk-inventory.tsv"
"$JQ_BIN" -n --arg checksum "$checksum" '{
    vulnerabilities: { found: false, count: 0, list: [] },
    warnings: { unmaintained: [{
        kind: "unmaintained",
        advisory: { id: "RUSTSEC-2099-0001" },
        package: { name: "canic-risk-transitive-fixture", version: "1.0.0", checksum: $checksum }
    }] }
}' >"$base"

# Exercise the production audit invocation without network or an advisory clone.
# A failed audit must expose its report, retain the bytes and preserve its status,
# even if replaying the report itself fails.
failure_fixture="$tmp_dir/audit-failure"
mkdir -p "$failure_fixture/scripts/ci" "$failure_fixture/bin" "$failure_fixture/evidence"
cp "$GATE" "$fixture/scripts/ci/require-jq.sh" "$failure_fixture/scripts/ci/"
cp "$fixture/tool-versions.env" "$failure_fixture/tool-versions.env"
touch "$failure_fixture/scripts/ci/dependency-risk-inventory.tsv"
printf '#!/usr/bin/env bash\nexit 0\n' >"$failure_fixture/scripts/ci/prepare-rustsec-db.sh"
printf '%s\n' '{"error":{"kind":"fixture","message":"synthetic audit failure"}}' >"$tmp_dir/audit-error.json"
export CANIC_AUDIT_TEST_TOOLS="$failure_fixture/tool-versions.env"
export CANIC_AUDIT_TEST_JSON="$tmp_dir/audit-error.json"
export CANIC_AUDIT_TEST_CAT
CANIC_AUDIT_TEST_CAT="$(command -v cat)"
cat >"$failure_fixture/bin/cargo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
    'audit --version')
        # shellcheck source=/dev/null
        source "$CANIC_AUDIT_TEST_TOOLS"
        printf 'cargo-audit %s\n' "$CANIC_CARGO_AUDIT_VERSION" ;;
    'fetch --locked') exit 0 ;;
    'audit --no-fetch --db '*)
        "$CANIC_AUDIT_TEST_CAT" "$CANIC_AUDIT_TEST_JSON"
        exit 37 ;;
    *) exit 99 ;;
esac
SH
cat >"$failure_fixture/bin/cat" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$CANIC_AUDIT_TEST_REPLAY_FAIL" == 1 ]]; then exit 73; fi
exec "$CANIC_AUDIT_TEST_CAT" "$@"
SH
chmod +x "$failure_fixture/bin/cargo" "$failure_fixture/bin/cat"
for replay_fail in 0 1; do
    status=0
    PATH="$failure_fixture/bin:$PATH" TMPDIR="$failure_fixture/evidence" \
        CANIC_AUDIT_TEST_REPLAY_FAIL="$replay_fail" \
        bash "$failure_fixture/scripts/ci/check-dependency-risk-inventory.sh" \
        >"$tmp_dir/audit-failure.stdout" 2>"$tmp_dir/audit-failure.stderr" || status=$?
    [[ "$status" == 37 ]] || fail "audit failure status was replaced: $status"
    retained="$(sed -n 's/^Dependency audit evidence retained: //p' "$tmp_dir/audit-failure.stderr")"
    [[ -n "$retained" ]] || fail "audit failure evidence path was not reported"
    cmp "$CANIC_AUDIT_TEST_JSON" "$retained/audit.json" || fail "audit failure report was not retained"
    if [[ "$replay_fail" == 0 ]]; then
        rg -F -x -f "$CANIC_AUDIT_TEST_JSON" "$tmp_dir/audit-failure.stderr" >/dev/null ||
            fail "audit failure report was hidden"
    fi
done

test_offline_database_isolation() {
    local tracked_advisory="crates/canic-risk-transitive-fixture/RUSTSEC-2099-0001.md"
    mkdir -p "$audit_db/crates/canic-risk-transitive-fixture"
    cat >"$audit_db/$tracked_advisory" <<'ADVISORY'
```toml
[advisory]
id = "RUSTSEC-2099-0001"
package = "canic-risk-transitive-fixture"
date = "2026-01-01"
informational = "unmaintained"

[versions]
patched = []
```

# Synthetic unmaintained dependency

Local gate fixture; no upstream advisory is represented.
ADVISORY
    git -C "$audit_db" init --quiet
    git -C "$audit_db" add -- "$tracked_advisory"
    git -C "$audit_db" -c user.name='Canic gate fixture' \
        -c user.email='fixture@example.invalid' -c core.hooksPath=/dev/null \
        -c commit.gpgsign=false commit --quiet -m 'Local advisory fixture'

    # A shared cache may retain an untracked copy after an upstream advisory is
    # renamed or moved. Offline isolation must copy only the selected tracked
    # database revision so the stale file cannot create a duplicate advisory ID.
    mkdir -p "$audit_db/crates/canic-stale-duplicate"
    cp "$audit_db/$tracked_advisory" \
        "$audit_db/crates/canic-stale-duplicate/${tracked_advisory##*/}"
    if (
        cd "$fixture"
        cargo audit --no-fetch --db "$audit_db" --json
    ) >"$tmp_dir/duplicate.json" 2>"$tmp_dir/duplicate.stderr"; then
        fail "unisolated duplicate advisory fixture was accepted"
    fi
    CANIC_CARGO_AUDIT_NO_FETCH=1 CANIC_CARGO_AUDIT_DB="$audit_db" \
        bash "$GATE" >"$tmp_dir/offline.log" 2>&1 || {
        cat "$tmp_dir/offline.log" >&2
        fail "tracked advisory isolation failed"
    }
}

bash "$GATE" --audit-json "$base" >/dev/null
if [ "$classification_only" -eq 0 ]; then
    test_offline_database_isolation
fi

vulnerability="$tmp_dir/vulnerability.json"
"$JQ_BIN" '.vulnerabilities.found = true | .vulnerabilities.count = 1 | .vulnerabilities.list = [{}]' \
    "$base" >"$vulnerability"
if bash "$GATE" --audit-json "$vulnerability" >/dev/null 2>&1; then
    fail "known vulnerability fixture was accepted"
fi

new_warning="$tmp_dir/new-warning.json"
"$JQ_BIN" '.warnings.unmaintained += [(.warnings.unmaintained[0]
    | .advisory.id = "RUSTSEC-2099-0002"
    | .package.name = "unexpected-package"
    | .package.version = "1.0.0"
    | .package.checksum = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")]' \
    "$base" >"$new_warning"
bash "$GATE" --audit-json "$new_warning" >/dev/null 2>&1 ||
    fail "new transitive informational advisory fixture was rejected"

missing_warning="$tmp_dir/missing-warning.json"
"$JQ_BIN" '.warnings.unmaintained |= .[1:]' "$base" >"$missing_warning"
bash "$GATE" --audit-json "$missing_warning" >/dev/null 2>&1 ||
    fail "removed transitive informational advisory fixture was rejected"

identity_drift="$tmp_dir/identity-drift.json"
"$JQ_BIN" '.warnings.unmaintained[0].package.version = "9.9.9"' "$base" >"$identity_drift"
bash "$GATE" --audit-json "$identity_drift" >/dev/null 2>&1 ||
    fail "transitive informational package identity drift fixture was rejected"

direct_warning="$tmp_dir/direct-warning.json"
"$JQ_BIN" '.warnings.unmaintained += [(.warnings.unmaintained[0]
    | .advisory.id = "RUSTSEC-2099-0003"
    | .package.name = "serde"
    | .package.version = "1.0.0")]' \
    "$base" >"$direct_warning"
if bash "$GATE" --audit-json "$direct_warning" >/dev/null 2>&1; then
    fail "unmaintained direct dependency fixture was accepted"
fi

yanked_warning="$tmp_dir/yanked-warning.json"
"$JQ_BIN" '.warnings.yanked = [(.warnings.unmaintained[0]
    | del(.advisory)
    | .kind = "yanked"
    | .package.name = "transitive-yanked-package")]' \
    "$base" >"$yanked_warning"
if bash "$GATE" --audit-json "$yanked_warning" >/dev/null 2>&1; then
    fail "yanked dependency fixture was accepted"
fi

missing_advisory="$tmp_dir/missing-advisory.json"
"$JQ_BIN" 'del(.warnings.unmaintained[0].advisory)' "$base" >"$missing_advisory"
bash "$GATE" --audit-json "$missing_advisory" >/dev/null 2>&1 ||
    fail "missing optional advisory shifted warning kind and package fields"

echo "dependency risk gate tests passed"
