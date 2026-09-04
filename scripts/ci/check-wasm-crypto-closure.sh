#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

CANONICAL_PACKAGE_PROFILES=(
    canic-fleet-coordinator:none
    canic-wasm-store:none
    canister_app:none
    canister_index_child:none
    canister_index_hub:none
    canister_root:root-sign
    canister_scale:none
    canister_scale_hub:none
    canister_test:delegated-verify
    canister_user_hub:none
    canister_user_shard:delegated-verify
)
CRYPTO_PACKAGES='^(base16ct|block-buffer|const-oid|crypto-bigint|crypto-common|der|digest|ecdsa|elliptic-curve|ff|group|hmac|ic-canister-sig-creation|ic-certification|ic-representation-independent-hash|ic-signature-verification|ic-verify-bls-signature|ic_bls12_381|k256|pairing|pkcs8|primeorder|rfc6979|sec1|sha2|signature|spki|subtle|zeroize)$'
SIGNATURE_PACKAGES='^(crypto-bigint|ecdsa|elliptic-curve|hmac|ic-canister-sig-creation|ic-certification|ic-signature-verification|ic-verify-bls-signature|ic_bls12_381|k256|pairing|rfc6979|signature)$'

package_tree() {
    cargo tree \
        --locked \
        --offline \
        --target wasm32-unknown-unknown \
        --edges normal \
        --package "$1" \
        --prefix none \
        --format '{p}'
}

crypto_identities() {
    awk -v accepted="$CRYPTO_PACKAGES" '$1 ~ accepted { print $1, $2 }' | sort -u
}

duplicate_crypto_packages() {
    awk '
        previous_name == $1 && previous_version != $2 { print $1 }
        { previous_name = $1; previous_version = $2 }
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
    case "$1" in
        none)
            ;;
        root-sign)
            printf '%s\n' \
                crypto-bigint \
                ecdsa \
                elliptic-curve \
                hmac \
                ic-canister-sig-creation \
                ic-certification \
                k256 \
                rfc6979 \
                signature
            ;;
        delegated-verify)
            printf '%s\n' \
                crypto-bigint \
                ecdsa \
                elliptic-curve \
                hmac \
                ic-canister-sig-creation \
                ic-certification \
                ic-signature-verification \
                ic-verify-bls-signature \
                ic_bls12_381 \
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
    duplicates="$(printf '%s\n' "$identities" | duplicate_crypto_packages)"
    if [[ -n "$duplicates" ]]; then
        printf 'deployed Wasm package %s resolves duplicate crypto libraries: %s\n' \
            "$package" "$(printf '%s' "$duplicates" | paste -sd, -)" >&2
        failures=$((failures + 1))
    fi
    if ! printf '%s\n' "$identities" | contains_package sha2; then
        printf 'deployed Wasm package %s does not resolve the canonical SHA-256 provider\n' \
            "$package" >&2
        failures=$((failures + 1))
    fi
    actual_signature_names="$(printf '%s\n' "$identities" | signature_names)"
    expected_signatures="$(expected_signature_packages "$profile")"
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

printf 'Wasm crypto closure passed: %d canonical roles; no duplicate crypto package versions; %d zero-auth roles contain no signature stack\n' \
    "${#CANONICAL_PACKAGE_PROFILES[@]}" "$zero_auth_count"
