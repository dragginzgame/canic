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
# decode -> verify material -> bind subject -> check scope -> consume update use.
if ! awk '
    /fn verify_token\(/ { in_fn = 1 }
    in_fn && /fn consume_update_token_once\(/ { in_fn = 0 }
    in_fn && /AuthOps::verify_token/ && !verify { verify = NR }
    in_fn && /enforce_subject_binding/ && !subject { subject = NR }
    in_fn && /enforce_required_scope/ && !scope { scope = NR }
    in_fn && /consume_update_token_once/ && !consume { consume = NR }
    END {
        if (!(verify && subject && scope && consume && verify < subject && subject < scope && scope < consume)) {
            exit 1
        }
    }
' crates/canic-core/src/access/auth/token.rs
then
    reject "delegated endpoint guard ordering changed"
fi

# Role-attestation verification may refresh key material only for unknown key
# IDs. Broad refresh-on-any-failure can hide real verifier failures.
if ! awk '
    /AttestationUnknownKeyId/ && !unknown { unknown = NR }
    /refresh\(\)/ && !refresh { refresh = NR }
    /Err\(err\) => Err\(RoleAttestationVerifyFlowError::Initial\(err\)\)/ && !fallback { fallback = NR }
    END {
        if (!(unknown && refresh && fallback && unknown < refresh && refresh < fallback)) {
            exit 1
        }
    }
' crates/canic-core/src/api/auth/verify_flow.rs
then
    reject "role-attestation refresh path is no longer narrowed to unknown key IDs"
fi

exit "$fail"
