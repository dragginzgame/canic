#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CANIC_BIN="${CANIC_BIN:-$ROOT/target/debug/canic}"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/canic-v1-readiness.XXXXXX")"

cleanup() {
    rm -rf "$TMP_ROOT"
}

trap cleanup EXIT

require_canic_bin() {
    if [ ! -x "$CANIC_BIN" ]; then
        echo "missing canic binary at $CANIC_BIN" >&2
        echo "run: cargo build -p canic-cli" >&2
        echo "or set CANIC_BIN=/path/to/canic" >&2
        exit 2
    fi
}

write_workspace_manifest() {
    cat > "$TMP_ROOT/Cargo.toml" <<'EOF'
[workspace]
members = []
resolver = "2"
EOF
}

write_policy_evidence_inputs() {
    cat > "$TMP_ROOT/policy.toml" <<'EOF'
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]
EOF

    cat > "$TMP_ROOT/envelope.json" <<'EOF'
{
  "envelope_schema": {
    "id": "canic.evidence_envelope.v1",
    "version": "1",
    "stability": "stable"
  },
  "canic_version": "0.55.0",
  "command": {
    "name": "canic fleet adoption report",
    "argv_normalized": ["canic", "fleet", "adoption", "report", "demo"],
    "argv_redactions": [],
    "format": "envelope-json"
  },
  "target": {
    "kind": "fleet_adoption",
    "deployment": null,
    "fleet": "demo",
    "role": null,
    "profile": "minimal",
    "network": null
  },
  "generated_at": "2026-06-01T00:00:00Z",
  "source_config": null,
  "inputs": [],
  "payload_schema": {
    "id": "canic.adoption_report.v1",
    "version": "1",
    "stability": "experimental"
  },
  "payload_sha256": "payloadpayloadpayloadpayloadpayloadpayloadpayloadpayloadpayloadpayloadpayl",
  "payload": {
    "report_id": "report-1"
  },
  "summary": {
    "warnings": [],
    "blocked_actions": [],
    "missing_or_stale_evidence": [],
    "evidence_conflicts": []
  },
  "exit_class": "success"
}
EOF
}

assert_contains() {
    local path="$1"
    local needle="$2"
    if ! grep -Fq "$needle" "$path"; then
        echo "expected $path to contain: $needle" >&2
        echo "actual content:" >&2
        sed -n '1,160p' "$path" >&2
        exit 1
    fi
}

main() {
    require_canic_bin
    write_workspace_manifest
    write_policy_evidence_inputs

    cd "$TMP_ROOT"

    "$CANIC_BIN" fleet create demo --yes > fleet-create.txt
    "$CANIC_BIN" scaffold canister demo store > scaffold-store.txt
    "$CANIC_BIN" fleet role inspect demo store > inspect-declared.txt
    "$CANIC_BIN" fleet role attach demo store --subnet prime > attach-store.txt
    "$CANIC_BIN" fleet role inspect demo store > inspect-attached.txt
    "$CANIC_BIN" deploy catalog list --format json --output catalog.json
    "$CANIC_BIN" evidence gate \
        --policy policy.toml \
        --envelope envelope.json \
        --format json \
        --output gate.json

    assert_contains fleet-create.txt "Created Canic fleet:"
    assert_contains scaffold-store.txt "state: declared"
    assert_contains inspect-declared.txt "state: declared"
    assert_contains inspect-declared.txt "deploy artifact: blocked"
    assert_contains attach-store.txt "state: attached"
    assert_contains inspect-attached.txt "state: attached"
    assert_contains inspect-attached.txt "deploy artifact: eligible"
    assert_contains catalog.json "\"entries\": []"
    assert_contains catalog.json "catalog.no_deployment_state"
    assert_contains gate.json "\"policy_status\": \"passed\""
    assert_contains gate.json "\"gate_exit_class\": \"success\""

    echo "v1 readiness smoke passed"
}

main "$@"
