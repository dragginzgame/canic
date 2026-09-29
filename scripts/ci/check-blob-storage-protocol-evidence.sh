#!/usr/bin/env bash
set -euo pipefail

record="${1:-docs/contracts/blob-storage-protocol-evidence.json}"

# This evidence record is a JSON protocol artifact, not operator configuration.
# Inventory prose remains editable without changing executable release authority.
if ! jq -e '
    def exact_keys($expected): keys == ($expected | sort);
    def source_path: type == "string" and test("^[a-zA-Z0-9_/-]+\\.rs$") and (contains("..") | not);
    def gateway_methods: [
        "_immutableObjectStorageBlobsAreLive",
        "_immutableObjectStorageBlobsToDelete",
        "_immutableObjectStorageConfirmBlobDeletion",
        "_immutableObjectStorageCreateCertificate",
        "_immutableObjectStorageUpdateGatewayPrincipals",
        "_immutableObjectStorageFundFromProjectCycles"
    ];
    def cashier_methods: [
        "account_balance_get_v1", "account_top_up_v1", "storage_gateway_principal_list_v1"
    ];
    exact_keys(["schema", "methods", "gateway_principal_refresh"])
    and .schema == "canic.blob_storage_protocol_evidence.v1"
    and ([.methods[].method] | sort) == ((gateway_methods + cashier_methods) | sort)
    and all(.methods[];
        exact_keys(["method", "protocol", "mode", "source_repository", "source_commit",
                    "source_path", "source_kind", "inventory_document"])
        and .source_repository == "toko"
        and (.source_commit | type == "string" and test("^[0-9a-f]{40}$") and . != ("0" * 40))
        and (.source_path | source_path)
        and (if .protocol == "gateway" then
            (.method as $method | gateway_methods | index($method) != null)
            and .source_kind == "endpoint"
            and .inventory_document == "docs/contracts/BLOB_STORAGE_INVENTORY.md"
        elif .protocol == "cashier" then
            (.method as $method | cashier_methods | index($method) != null)
            and .source_kind == "consumer"
            and .inventory_document == "docs/contracts/BLOB_STORAGE_CASHIER_INVENTORY.md"
        else false end)
        and .mode == (if .method == "_immutableObjectStorageBlobsAreLive"
                         or .method == "_immutableObjectStorageBlobsToDelete"
                     then "query" else "update" end))
    and .gateway_principal_refresh == {"empty_response": "reject_preserve_previous"}
' -- "$record" >/dev/null; then
    echo "invalid blob protocol evidence: $record" >&2
    exit 1
fi

while IFS= read -r document; do
    [[ -f "$document" ]] || { echo "missing protocol source notes: $document" >&2; exit 1; }
done < <(jq -r '.methods[].inventory_document' -- "$record" | sort -u)
