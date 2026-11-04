#!/usr/bin/env bash
# Explicit qualification of the retained Auth client against a disposable live Fleet.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
if [[ -z "${CANIC_TEST_SCRATCH:-}" ]]; then
    exec bash scripts/ci/run-with-test-scratch.sh bash "$0" "$@"
fi
case "${1:-transport}" in
    transport) auth_case=delivered_client_signs_admission_and_device_scoped_logout ;;
    durable|issuer)
        : "${CANIC_AUTH_BROWSER_EXECUTABLE:?Select a prepared Chromium executable}"
        [[ -x "$CANIC_AUTH_BROWSER_EXECUTABLE" ]]
        if [[ "$1" == issuer ]]; then
            auth_case=issuer_http::browser_reconciles_issuance_before_admission
        else
            auth_case=delivered_client_reopens_durable_logout
        fi ;;
    *) echo 'usage: qualify-auth-browser.sh [transport|durable|issuer]' >&2; exit 2 ;;
esac
export ICP_ENVIRONMENT=local CARGO_NET_OFFLINE=true
node --input-type=module -e '
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
const manifest = JSON.parse(readFileSync("crates/canic-host/examples/frontend-consumer/package.json", "utf8"));
assert.equal(process.versions.node, manifest.engines.node, "Use the consumer-selected Node runtime");
'
node crates/canic-host/examples/frontend-consumer/check-boundary.mjs
# shellcheck source=scripts/ci/native-icp-lib.sh
source scripts/ci/native-icp-lib.sh
# shellcheck source=scripts/ci/ic-tool-pins.sh
source scripts/ci/ic-tool-pins.sh
use_native_test_icp "$CANIC_TEST_SCRATCH" "$CANIC_ICP_CLI_VERSION" "$ROOT"
testkit_server="$(bash scripts/ci/testkit-server.sh --check)"
"$testkit_server" run --idle-ttl 7200 \
    --server-stdout "$CANIC_TEST_SCRATCH/server.stdout" \
    --server-stderr "$CANIC_TEST_SCRATCH/server.stderr" -- \
    cargo test --locked -p canic-tests --test native_agent_delegation \
    "browser_transport::$auth_case" \
    -- --exact --ignored --test-threads=1 --nocapture
