#!/usr/bin/env bash
set -euo pipefail

fail=0

reject() {
    echo "auth trust-chain guard: $*" >&2
    fail=1
}

# The public AuthApi must not expose a material-only token verifier. Endpoint
# auth also has to bind the verified subject to the caller and consume update
# tokens once.
if rg -n "pub(\\([^)]*\\))?\\s+(async\\s+)?fn\\s+verify_token\\b|pub(\\([^)]*\\))?\\s+(async\\s+)?fn\\s+verify_token_material\\b|AuthApi::verify_token\\b" \
    crates/canic-core/src/api/auth crates/canic/src --glob '*.rs'
then
    reject "public delegated-token verifier helper detected"
fi

# Auth DTOs are boundary data only. Verification, signing, key resolution,
# replay, and policy behavior belongs in ops/access/api code.
if sed '/#\[cfg(test)\]/,$d' crates/canic-core/src/dto/auth.rs \
    | rg -n "impl\\s+(DelegatedToken|DelegatedTokenClaims|RoleAttestation|SignedRoleAttestation)\\b|\\b(verify|sign|resolve|replay|consume|policy|validate)\\w*\\s*\\("
then
    reject "auth DTO behavior detected"
fi

# Preserve the delegated endpoint guard sequence:
# decode -> verify material -> bind subject -> check scope.
# 0.61 removes verifier-local delegated-token use consumption; replay-sensitive
# commands must use domain replay receipts instead of an auth-token use store.
if ! awk '
    /fn verify_token\(/ { in_fn = 1 }
    in_fn && /pub\(super\) fn enforce_subject_binding/ { in_fn = 0 }
    in_fn && /AuthOps::verify_token/ && !verify { verify = NR }
    in_fn && /enforce_subject_binding/ && !subject { subject = NR }
    in_fn && /enforce_required_scope/ && !scope { scope = NR }
    END {
        if (!(verify && subject && scope && verify < subject && subject < scope)) {
            exit 1
        }
    }
' crates/canic-core/src/access/auth/token.rs
then
    reject "delegated endpoint guard ordering changed"
fi

# Role-attestation verification uses embedded root canister-signature proofs.
# It must not reintroduce verifier-local ECDSA key refresh or the retired
# verify-flow wrapper.
if [[ -e crates/canic-core/src/api/auth/verify_flow.rs ]]; then
    reject "retired role-attestation verify_flow module detected"
fi

if rg -n "AttestationUnknownKeyId|RoleAttestationVerifyFlow|verify_keyed_proof_with_single_refresh|attestation_public_key|canic_attestation_key_set" \
    crates/canic-core/src crates/canic/src canisters/test fleets/test --glob '*.rs'
then
    reject "role-attestation key refresh or key-cache surface detected"
fi

# Delegated-token prepare derives its nonce deterministically from local inputs.
# It must remain synchronous and must not call the management canister for
# randomness or any other side effect.
if rg -n "raw_rand|management_canister|\\.await|Call::|ic_cdk::call" \
    crates/canic-core/src/ops/auth/token.rs \
    crates/canic-core/src/ops/auth/delegated/mint.rs
then
    reject "delegated-token prepare contains async or management-canister side effect"
fi

exit "$fail"
