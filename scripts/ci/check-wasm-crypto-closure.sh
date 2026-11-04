#!/usr/bin/env bash

set -euo pipefail

# Package identities are compared as sets, independent of the host's collation.
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

# The preparation owner supplies the locked graph before this offline check.

CANONICAL_PACKAGE_PROFILES=(
    canic/fleet-coordinator-canister:none
    canic/control-plane:none
    delegation_root_stub:root-sign
    canic/wasm-store-canister:none
    canister_app:none
    canister_index_child:none
    canister_index_hub:none
    canister_scale:none
    canister_scale_hub:none
    canister_test:delegated-verify
    canister_user_hub:none
    canister_user_shard:delegated-verify
)
CRYPTO_PACKAGES='^(base16ct|block-buffer|const-oid|crypto-bigint|crypto-common|der|digest|ecdsa|elliptic-curve|ff|group|hmac|ic-canister-sig-creation|ic-certification|ic-representation-independent-hash|ic-signature-verification|ic-verify-bls-signature|ic_bls12_381|k256|pairing|pkcs8|primefield|primeorder|rfc6979|sec1|sha2|signature|spki|subtle|wnaf|zeroize)$'
SIGNATURE_PACKAGES='^(crypto-bigint|ecdsa|elliptic-curve|hmac|ic-canister-sig-creation|ic-certification|ic-signature-verification|ic-verify-bls-signature|ic_bls12_381|k256|pairing|rfc6979|signature)$'

package_tree() {
    local selection="$1"
    local args=(--package "${selection%%/*}")
    if [[ "$selection" == */* ]]; then
        args+=(--no-default-features --features "${selection#*/}")
    fi
    cargo tree \
        --locked \
        --offline \
        --target wasm32-unknown-unknown \
        --edges normal \
        "${args[@]}" \
        --prefix none \
        --format '{p}'
}

crypto_identities() {
    awk -v accepted="$CRYPTO_PACKAGES" '$1 ~ accepted { sub(/ \(\*\)$/, ""); print }' | sort -u
}

expected_crypto_cohorts() {
    # Platform's IC hashes retain digest 0.10; canonical self-authenticating
    # principals use digest 0.11 even without a signature verifier. Admit one
    # full identity per hash cohort; signature packages remain role-bound below.
    case "$1" in
        none|root-sign|delegated-verify)
            printf '%s\n' \
                'block-buffer v0.10' 'block-buffer v0.12' \
                'crypto-common v0.1' 'crypto-common v0.2' \
                'digest v0.10' 'digest v0.11' \
                'sha2 v0.10' 'sha2 v0.11'
            ;;
        *) return 1 ;;
    esac
    if [[ "$1" == delegated-verify ]]; then
        printf '%s\n' 'ff v0.13' 'ff v0.14' 'group v0.13' 'group v0.14'
    fi
}

rejected_crypto_packages() {
    local cohorts
    cohorts="$(expected_crypto_cohorts "$1")" || return 1
    awk -v cohorts="$cohorts" '
        BEGIN {
            count = split(cohorts, rows, "\n")
            for (i = 1; i <= count; i++) {
                if (split(rows[i], fields, " ") != 2) continue
                reviewed[fields[1], fields[2]] = 1
                restricted[fields[1]] = 1
            }
        }
        {
            name = $1
            if (name in restricted) {
                split($2, version, ".")
                cohort = version[1] "." version[2]
                if (!reviewed[name, cohort] || ++seen_cohort[name, cohort] > 1) rejected[name] = 1
            } else if (++seen_name[name] > 1) {
                rejected[name] = 1
            }
        }
        END { for (name in rejected) print name }
    ' | sort -u
}

contains_package() {
    local package_name="$1"
    awk -v package_name="$package_name" '$1 == package_name { found = 1 } END { exit !found }'
}

signature_names() {
    awk -v accepted="$SIGNATURE_PACKAGES" '$1 ~ accepted { print $1 }' | sort -u
}

expected_signature_packages() {
    # Platform owns certification and BLS implementation inside its package.
    # Separate IC signature/certification packages remain detected above and are
    # unexpected in these normal Wasm profiles. Native Agent graphs are separate.
    case "$1" in
        none)
            ;;
        root-sign)
            # Root batch signing and proof validation need ECDSA, not BLS verification.
            printf '%s\n' \
                crypto-bigint \
                ecdsa \
                elliptic-curve \
                hmac \
                k256 \
                rfc6979 \
                signature
            ;;
        delegated-verify)
            # Certified receivers still require both Root ECDSA and canister signatures.
            printf '%s\n' \
                crypto-bigint \
                ecdsa \
                elliptic-curve \
                hmac \
                k256 \
                pairing \
                rfc6979 \
                signature
            ;;
        *)
            printf 'unknown canonical crypto profile: %s\n' "$1" >&2
            return 1
            ;;
    esac
}

failures=0
zero_auth_count=0
for package_profile in "${CANONICAL_PACKAGE_PROFILES[@]}"; do
    package="${package_profile%%:*}"
    profile="${package_profile#*:}"
    tree="$(package_tree "$package")"
    identities="$(printf '%s\n' "$tree" | crypto_identities)"
    rejected="$(printf '%s\n' "$identities" | rejected_crypto_packages "$profile")"
    if [[ -n "$rejected" ]]; then
        printf 'deployed Wasm package %s resolves crypto identities outside its reviewed cohorts: %s\n' \
            "$package" "$(printf '%s' "$rejected" | paste -sd, -)" >&2
        failures=$((failures + 1))
    fi
    if ! printf '%s\n' "$identities" | contains_package sha2; then
        printf 'deployed Wasm package %s does not resolve the canonical SHA-256 provider\n' \
            "$package" >&2
        failures=$((failures + 1))
    fi
    actual_signature_names="$(printf '%s\n' "$identities" | signature_names)"
    expected_signatures="$(expected_signature_packages "$profile" | sort -u)"
    if [[ "$actual_signature_names" != "$expected_signatures" ]]; then
        printf 'deployed Wasm package %s has crypto profile %s but resolves signature libraries [%s], expected [%s]\n' \
            "$package" \
            "$profile" \
            "$(printf '%s' "$actual_signature_names" | paste -sd, -)" \
            "$(printf '%s' "$expected_signatures" | paste -sd, -)" >&2
        failures=$((failures + 1))
    fi
    if [[ "$profile" == none ]]; then
        zero_auth_count=$((zero_auth_count + 1))
    fi
done

if ((failures > 0)); then
    exit 1
fi

printf 'Wasm crypto closure passed: %d canonical roles; one identity per reviewed crypto cohort; %d zero-auth roles contain no signature stack\n' \
    "${#CANONICAL_PACKAGE_PROFILES[@]}" "$zero_auth_count"
