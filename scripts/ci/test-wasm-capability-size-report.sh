#!/usr/bin/env bash

set -euo pipefail

# shellcheck source=scripts/ci/require-jq.sh
source "$(dirname "${BASH_SOURCE[0]}")/../ci/require-jq.sh"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CLASSIFIER="$ROOT/scripts/ci/wasm-capability-size-report.jq"
require_jq
FIXTURE="$(mktemp -d "${TMPDIR:-/tmp}/canic-wasm-capability-size-test.XXXXXX")"
cleanup() {
    local status=$?
    if [[ "$status" == 0 ]]; then
        rm -rf "$FIXTURE"
    else
        echo "Wasm size-report fixtures retained: $FIXTURE" >&2
    fi
}
trap cleanup EXIT

"$JQ_BIN" -n '{
  artifact: {file_name: "diagnostic.wasm", sha256: "fixture", bytes: 425},
  context: {
    role: "project_instance",
    build_profile: "debug",
    build_network: "ic",
    producer_identity: "fixture-a",
    role_capabilities: ["Runtime", "ChildProvisioning"],
    metrics_tiers: ["Core", "Runtime", "Security"],
    endpoint_exports: 273
  },
  tool: "twiggy fixture",
  items: [
    {name: "canic_core::ops::auth::verify", shallow_size: 100},
    {name: "k256::ecdsa::verify", shallow_size: 25},
    {name: "canic_metrics_core::encode", shallow_size: 80},
    {name: "canic_control_plane::child::create", shallow_size: 70},
    {name: "canic_core::workflow::status", shallow_size: 50},
    {name: "project_instance::endpoint", shallow_size: 40},
    {name: "code[12]", shallow_size: 30},
    {name: "data[0]", shallow_size: 15},
    {name: "type[1]: (i32) -> nil", shallow_size: 5},
    {name: "export \"canister_update endpoint\"", shallow_size: 10}
  ]
}' | "$JQ_BIN" -f "$CLASSIFIER" >"$FIXTURE/partial.json"

"$JQ_BIN" -e '
  .schema == "canic.wasm_capability_size.v1"
  and .analysis.artifact_bytes_match == true
  and .analysis.symbol_attribution == "partial"
  and .analysis.named_code_bytes == 365
  and .analysis.unattributed_code_bytes == 30
  and ([.categories[] | {key: .category, value: .shallow_bytes}] | from_entries) == {
    cryptography: 25,
    authentication_and_admission: 100,
    metrics: 80,
    child_provisioning: 70,
    canic_runtime: 50,
    application_and_upstream: 40,
    unattributed_code: 30,
    wasm_structural_and_abi: 30
  }
' "$FIXTURE/partial.json" >/dev/null

"$JQ_BIN" -n '{
  artifact: {file_name: "stripped.wasm", sha256: "fixture", bytes: 80},
  context: {
    role: "project_instance",
    build_profile: "release",
    build_network: "ic",
    producer_identity: "fixture-b",
    role_capabilities: [],
    metrics_tiers: [],
    endpoint_exports: null
  },
  tool: "twiggy fixture",
  items: [
    {name: "code[7]", shallow_size: 60},
    {name: "data[0]", shallow_size: 20}
  ]
}' | "$JQ_BIN" -f "$CLASSIFIER" >"$FIXTURE/stripped.json"

"$JQ_BIN" -e '
  .analysis.symbol_attribution == "unavailable"
  and .analysis.named_code_bytes == 0
  and .analysis.unattributed_code_bytes == 60
  and (.categories[] | select(.category == "canic_runtime") | .shallow_bytes) == 0
' "$FIXTURE/stripped.json" >/dev/null

# Exercise the report writer with controlled analysis output and real digest backends.
mkdir "$FIXTURE/common-bin"
for tool in bash dirname mktemp rm wc tr head basename mv; do
    ln -s "$(command -v "$tool")" "$FIXTURE/common-bin/$tool"
done
cat >"$FIXTURE/common-bin/twiggy" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == --version ]]; then
    printf 'twiggy test fixture\n'
else
    printf '[{"name":"data[0]","shallow_size":3}]\n'
fi
EOF
chmod +x "$FIXTURE/common-bin/twiggy"
report_args=(--role component --build-profile debug --build-network local --producer-identity fixture)
expected_sha256=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
backend_count=0
report_backend_path=""
for backend in sha256sum shasum; do
    command -v "$backend" >/dev/null || continue
    backend_count=$((backend_count + 1))
    mkdir "$FIXTURE/$backend-bin"
    ln -s "$(command -v "$backend")" "$FIXTURE/$backend-bin/$backend"
    report_backend_path="$FIXTURE/$backend-bin:$FIXTURE/common-bin"
    for name in 'file with spaces' '-leading-dash' 'back\slash' $'line\nbreak'; do
        printf abc >"$FIXTURE/$name"
        PATH="$report_backend_path" "$BASH" \
            "$ROOT/scripts/ci/wasm-capability-size-report.sh" \
            --wasm "$FIXTURE/$name" --output "$FIXTURE/report.json" \
            "${report_args[@]}" >"$FIXTURE/report.log" 2>&1
        "$JQ_BIN" -e --arg hash "$expected_sha256" --arg name "$name" '
            .artifact.sha256 == $hash and .artifact.file_name == $name
            and .artifact.bytes == 3 and .analysis.artifact_bytes_match == true
        ' "$FIXTURE/report.json" >/dev/null
    done
done
[[ "$backend_count" -gt 0 ]]

mkdir "$FIXTURE/failing-bin"
cat >"$FIXTURE/failing-bin/sha256sum" <<'EOF'
#!/usr/bin/env bash
exit 9
EOF
cat >"$FIXTURE/failing-bin/shasum" <<'EOF'
#!/usr/bin/env bash
printf 'fallback must not run\n' >"$REPORT_FALLBACK_MARKER"
exit 0
EOF
chmod +x "$FIXTURE/failing-bin/sha256sum" "$FIXTURE/failing-bin/shasum"
printf retained >"$FIXTURE/report.json"
status=0
PATH="$FIXTURE/failing-bin:$FIXTURE/common-bin" REPORT_FALLBACK_MARKER="$FIXTURE/fallback" \
    "$BASH" "$ROOT/scripts/ci/wasm-capability-size-report.sh" \
    --wasm "$FIXTURE/file with spaces" --output "$FIXTURE/report.json" \
    "${report_args[@]}" >"$FIXTURE/backend-failure.log" 2>&1 || status=$?
[[ "$status" == 9 && ! -e "$FIXTURE/fallback" ]]
[[ "$(cat "$FIXTURE/report.json")" == retained ]]
status=0
"$BASH" "$ROOT/scripts/ci/wasm-capability-size-report.sh" \
    --wasm "$FIXTURE/missing" --output "$FIXTURE/report.json" \
    "${report_args[@]}" >"$FIXTURE/missing-input.log" 2>&1 || status=$?
[[ "$status" != 0 && "$(cat "$FIXTURE/report.json")" == retained ]]
printf abc >"$FIXTURE/unreadable"
chmod 000 "$FIXTURE/unreadable"
if [[ ! -r "$FIXTURE/unreadable" ]]; then
    status=0
    PATH="$report_backend_path" "$BASH" \
        "$ROOT/scripts/ci/wasm-capability-size-report.sh" \
        --wasm "$FIXTURE/unreadable" --output "$FIXTURE/report.json" \
        "${report_args[@]}" >"$FIXTURE/unreadable-input.log" 2>&1 || status=$?
    [[ "$status" != 0 && "$(cat "$FIXTURE/report.json")" == retained ]]
fi
chmod u+r "$FIXTURE/unreadable"

echo "Wasm capability size report tests passed"
