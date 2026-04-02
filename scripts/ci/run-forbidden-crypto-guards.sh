#!/usr/bin/env bash
set -euo pipefail

if rg -n "sign_with_ecdsa|verify_ecdsa|ecdsa_public_key" crates --glob '!crates/canic-core/src/ops/ic/ecdsa.rs'; then
    echo "forbidden ECDSA API detected" >&2
    exit 1
fi
