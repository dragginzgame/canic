#!/usr/bin/env bash
set -euo pipefail

inventory="${BLOB_STORAGE_INVENTORY:-docs/contracts/BLOB_STORAGE_INVENTORY.md}"
required_methods=(
    "_immutableObjectStorageBlobsAreLive"
    "_immutableObjectStorageBlobsToDelete"
    "_immutableObjectStorageConfirmBlobDeletion"
    "_immutableObjectStorageCreateCertificate"
    "_immutableObjectStorageUpdateGatewayPrincipals"
    "_immutableObjectStorageFundFromProjectCycles"
)

if [[ ! -f "$inventory" ]]; then
    echo "blob-storage inventory is missing: $inventory" >&2
    exit 1
fi

status="$(
    sed -n 's/^Status: \*\*\(.*\)\*\*$/\1/p' "$inventory" \
        | head -n 1
)"

if [[ "$status" == "Complete" ]]; then
    failed=0

    validate_complete_method_section() {
        local method="$1"
        local section

        if ! rg -q "^### \`$method\`$" "$inventory"; then
            echo "blob-storage inventory missing method section: $method" >&2
            failed=1
            return
        fi

        section="$(
            awk -v method="$method" '
                $0 == "### `" method "`" { in_section = 1; print; next }
                in_section && /^### / { exit }
                in_section { print }
            ' "$inventory"
        )"

        if ! grep -q "^Status: \*\*Complete\*\*$" <<<"$section"; then
            echo "blob-storage inventory method is not complete: $method" >&2
            failed=1
        fi

        if grep -q "TBD" <<<"$section"; then
            echo "blob-storage inventory method still has TBD fields: $method" >&2
            failed=1
        fi
    }

    validate_complete_toko_section() {
        local section

        section="$(
            awk '
                $0 == "### Toko" { in_section = 1; print; next }
                in_section && /^## / { exit }
                in_section { print }
            ' "$inventory"
        )"

        if [[ -z "$section" ]]; then
            echo "blob-storage inventory missing Toko compatibility section" >&2
            failed=1
            return
        fi

        if ! grep -q "^Status: \*\*Complete\*\*$" <<<"$section"; then
            echo "blob-storage Toko compatibility notes are not complete" >&2
            failed=1
        fi

        if grep -q "TBD" <<<"$section"; then
            echo "blob-storage Toko compatibility notes still have TBD fields" >&2
            failed=1
        fi
    }

    for method in "${required_methods[@]}"; do
        validate_complete_method_section "$method"
    done
    validate_complete_toko_section

    if (( failed != 0 )); then
        echo "blob-storage inventory is marked Complete but required evidence is incomplete" >&2
        exit 1
    fi

    exit 0
fi

echo "blob-storage inventory status is '$status'; implementation remains gated" >&2

failed=0

check_forbidden() {
    local description="$1"
    local output
    local status
    shift

    if output="$("$@" 2>&1)"; then
        status=0
    else
        status=$?
    fi

    if (( status > 1 )); then
        printf '%s\n' "$output" >&2
        exit "$status"
    fi

    if [[ -n "$output" ]]; then
        printf '%s\n' "$output" >&2
        echo "forbidden blob-storage implementation surface found: $description" >&2
        failed=1
    fi
}

mapfile -t cargo_files < <(find Cargo.toml crates canisters fleets -name Cargo.toml -print)

check_forbidden \
    "blob-storage feature or dependency metadata before inventory completion" \
    rg -n "blob-storage|blob_storage" "${cargo_files[@]}"

check_forbidden \
    "blob_storage source/module path before inventory completion" \
    find crates canisters fleets -path "*/blob_storage*" -print -quit

check_forbidden \
    "gateway method literal before inventory completion" \
    rg -n "_immutableObjectStorage" crates canisters fleets \
        --glob "*.rs" \
        --glob "*.did" \
        --glob "*.toml"

check_forbidden \
    "internal blob-storage API/model type before inventory completion" \
    rg -n "BlobStorageApi|BlobRootHash|BlobStorage" crates canisters fleets \
        --glob "*.rs" \
        --glob "*.did"

check_forbidden \
    "blob-storage billing/Cashier implementation surface before inventory completion" \
    rg -n "blob-storage-billing|blob_storage_billing|cashier|Cashier|account_balance_get_v1|account_top_up_v1|storage_gateway_principal_list_v1|get_blob_storage_status" \
        crates canisters fleets \
        --glob "*.rs" \
        --glob "*.did" \
        --glob "*.toml"

if (( failed != 0 )); then
    echo "complete $inventory before adding blob-storage implementation files" >&2
    exit 1
fi
