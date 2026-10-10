#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/canic-crypto-closure.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/bin"
cat >"$fixture/bin/cargo" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
[[ "$LC_ALL" == C ]]
case "$1" in
    fetch) exit 0 ;;
    tree) shift ;;
    *) exit 2 ;;
esac
package=""
features=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --package) package="$2"; shift ;;
        --features) features="$2"; shift ;;
    esac
    shift
done
[[ -n "$package" ]]
printf '%s v0.0.0\n' "$package"
if [[ "$CRYPTO_TEST_CASE" != missing-sha256 ]]; then
    printf 'sha2 v0.10.9\nsha2 v0.10.9\n'
fi
if [[ "$CRYPTO_TEST_CASE" == duplicate-version ]]; then
    printf 'sha2 v0.11.0\n'
fi
case "$package/$features" in
    delegation_root_stub/|canister_test/|canister_user_shard/)
        # Deliberately unordered and repeated identities must compare as a set.
        printf '%s v0.1.0\n' signature rfc6979 k256 ic_bls12_381 \
            ic-verify-bls-signature ic-signature-verification ic-certification \
            ic-canister-sig-creation hmac elliptic-curve ecdsa crypto-bigint signature
        if [[ "$CRYPTO_TEST_CASE" != missing-signature ]]; then
            printf 'pairing v0.1.0\n'
        fi
        ;;
    canister_app/)
        if [[ "$CRYPTO_TEST_CASE" == unexpected-signature ]]; then
            printf 'k256 v0.1.0\n'
        fi
        ;;
esac
STUB
chmod +x "$fixture/bin/cargo"

check_case() {
    local selection="$1"
    local expected="$2"
    local collation="$3"
    local status=0
    PATH="$fixture/bin:$PATH" LC_ALL="$collation" CRYPTO_TEST_CASE="$selection" \
        bash "$ROOT/scripts/ci/check-wasm-crypto-closure.sh" >"$fixture/output" 2>&1 || status=$?
    if [[ "$status" != "$expected" ]]; then
        cat "$fixture/output" >&2
        printf 'crypto closure fixture %s returned %s, expected %s\n' \
            "$selection" "$status" "$expected" >&2
        exit 1
    fi
}

check_case valid 0 C
host_collation="$(locale -a | awk 'tolower($0) ~ /^en_us\./ { print; exit }')"
if [[ -n "$host_collation" ]]; then
    check_case valid 0 "$host_collation"
fi
for selection in missing-sha256 duplicate-version missing-signature unexpected-signature; do
    check_case "$selection" 1 C
done
echo 'Wasm crypto closure fixtures passed: collation, package sets, versions and SHA-256 (Cargo stubs)'
