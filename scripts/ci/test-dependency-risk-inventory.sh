#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

fail() {
    echo "dependency risk gate test failed: $1" >&2
    exit 1
}

command -v git >/dev/null 2>&1 || fail "git is unavailable"
command -v jq >/dev/null 2>&1 || fail "jq is unavailable"
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
checksum="$(sha256sum "$fixture/transitive/Cargo.toml" | cut -d ' ' -f1)"
printf 'RUSTSEC-2099-0001\tunmaintained\tcanic-risk-transitive-fixture\t1.0.0\t%s\tserde\n' \
    "$checksum" >"$fixture/scripts/ci/dependency-risk-inventory.tsv"
jq -n --arg checksum "$checksum" '{
    vulnerabilities: { found: false, count: 0, list: [] },
    warnings: { unmaintained: [{
        kind: "unmaintained",
        advisory: { id: "RUSTSEC-2099-0001" },
        package: { name: "canic-risk-transitive-fixture", version: "1.0.0", checksum: $checksum }
    }] }
}' >"$base"

mkdir -p "$audit_db/crates/canic-risk-transitive-fixture"
tracked_advisory="crates/canic-risk-transitive-fixture/RUSTSEC-2099-0001.md"
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

bash "$GATE" --audit-json "$base" >/dev/null

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

vulnerability="$tmp_dir/vulnerability.json"
jq '.vulnerabilities.found = true | .vulnerabilities.count = 1 | .vulnerabilities.list = [{}]' \
    "$base" >"$vulnerability"
if bash "$GATE" --audit-json "$vulnerability" >/dev/null 2>&1; then
    fail "known vulnerability fixture was accepted"
fi

new_warning="$tmp_dir/new-warning.json"
jq '.warnings.unmaintained += [(.warnings.unmaintained[0]
    | .advisory.id = "RUSTSEC-2099-0002"
    | .package.name = "unexpected-package"
    | .package.version = "1.0.0"
    | .package.checksum = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")]' \
    "$base" >"$new_warning"
bash "$GATE" --audit-json "$new_warning" >/dev/null 2>&1 ||
    fail "new transitive informational advisory fixture was rejected"

missing_warning="$tmp_dir/missing-warning.json"
jq '.warnings.unmaintained |= .[1:]' "$base" >"$missing_warning"
bash "$GATE" --audit-json "$missing_warning" >/dev/null 2>&1 ||
    fail "removed transitive informational advisory fixture was rejected"

identity_drift="$tmp_dir/identity-drift.json"
jq '.warnings.unmaintained[0].package.version = "9.9.9"' "$base" >"$identity_drift"
bash "$GATE" --audit-json "$identity_drift" >/dev/null 2>&1 ||
    fail "transitive informational package identity drift fixture was rejected"

direct_warning="$tmp_dir/direct-warning.json"
jq '.warnings.unmaintained += [(.warnings.unmaintained[0]
    | .advisory.id = "RUSTSEC-2099-0003"
    | .package.name = "serde"
    | .package.version = "1.0.0")]' \
    "$base" >"$direct_warning"
if bash "$GATE" --audit-json "$direct_warning" >/dev/null 2>&1; then
    fail "unmaintained direct dependency fixture was accepted"
fi

yanked_warning="$tmp_dir/yanked-warning.json"
jq '.warnings.yanked = [(.warnings.unmaintained[0]
    | .advisory.id = "RUSTSEC-2099-0004"
    | .kind = "yanked"
    | .package.name = "transitive-yanked-package")]' \
    "$base" >"$yanked_warning"
if bash "$GATE" --audit-json "$yanked_warning" >/dev/null 2>&1; then
    fail "yanked dependency fixture was accepted"
fi

echo "dependency risk gate tests passed"
