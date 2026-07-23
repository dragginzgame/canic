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
required_common_method_fields=(
    "Source repository or local source identifier"
    "Source commit SHA"
    "Source file path"
    "Mode"
    "Candid signature"
    "Request DTO shape"
    "Response DTO shape"
    "Unauthorized behavior"
    "Production-vs-local differences"
)
blob_root_hash_toko_field="Mapping from Toko blob identity into Canic \`BlobRootHash\`"
required_toko_fields=(
    "Local source identifier"
    "Source commit SHA"
    "$blob_root_hash_toko_field"
    "Migration/read-through strategy"
)

require_command() {
    local command_name="$1"

    if command -v "$command_name" >/dev/null 2>&1; then
        return 0
    fi

    echo "missing required tool: $command_name" >&2
    echo "run 'make install-dev' or 'make update-dev' to install the shared Canic toolchain" >&2
    exit 1
}

if [[ ! -f "$inventory" ]]; then
    echo "blob-storage inventory is missing: $inventory" >&2
    exit 1
fi

require_command rg

status="$(
    sed -n 's/^Status: \*\*\(.*\)\*\*$/\1/p' "$inventory" \
        | head -n 1
)"

if [[ "$status" == "Complete" ]]; then
    failed=0

    require_method_field() {
        local method="$1"
        local section="$2"
        local field="$3"

        if ! grep -Eq "^- ${field}: .+$" <<<"$section"; then
            echo "blob-storage inventory method missing required field: $method: $field" >&2
            failed=1
        fi
    }

    require_method_source_commit_sha() {
        local method="$1"
        local section="$2"

        if ! grep -Eq "^- Source commit SHA: [0-9a-fA-F]{40}([0-9a-fA-F]{24})?$" <<<"$section"; then
            echo "blob-storage inventory method has invalid source commit SHA: $method" >&2
            failed=1
        fi
    }

    validate_no_placeholder_method_evidence() {
        local method="$1"
        local section="$2"

        if grep -Eiq "^- [^:]+: *(TODO|unknown|unresolved|missing source|placeholder|source-backed evidence)([[:space:].,;]|$)" <<<"$section"; then
            echo "blob-storage inventory method still has placeholder evidence: $method" >&2
            failed=1
        fi
    }

    validate_method_specific_fields() {
        local method="$1"
        local section="$2"

        case "$method" in
            "_immutableObjectStorageBlobsAreLive")
                require_method_field "$method" "$section" "Malformed input behavior"
                require_method_field "$method" "$section" "Batch ordering semantics"
                require_method_field "$method" "$section" "Duplicate-input semantics"
                require_method_field "$method" "$section" "Absent-hash behavior"
                require_method_field "$method" "$section" "Maximum batch size"
                ;;
            "_immutableObjectStorageBlobsToDelete")
                require_method_field "$method" "$section" "Result ordering"
                require_method_field "$method" "$section" "Maximum batch size"
                require_method_field "$method" "$section" "Repeat-return behavior until confirmation"
                require_method_field "$method" "$section" "Empty pending-deletion behavior"
                ;;
            "_immutableObjectStorageConfirmBlobDeletion")
                require_method_field "$method" "$section" "Unknown blob behavior"
                require_method_field "$method" "$section" "Live-but-not-pending behavior"
                require_method_field "$method" "$section" "Already-confirmed behavior"
                require_method_field "$method" "$section" "Idempotency semantics"
                ;;
            "_immutableObjectStorageCreateCertificate")
                require_method_field "$method" "$section" "Certificate material source"
                require_method_field "$method" "$section" "Mutation-before-certificate behavior"
                require_method_field "$method" "$section" "Rollback or no-rollback behavior"
                require_method_field "$method" "$section" "Repeated create behavior"
                require_method_field "$method" "$section" "Metadata conflict/enrichment behavior"
                require_method_field "$method" "$section" "Malformed request behavior"
                ;;
            "_immutableObjectStorageUpdateGatewayPrincipals")
                require_method_field "$method" "$section" "Cashier dependency"
                ;;
            "_immutableObjectStorageFundFromProjectCycles")
                require_method_field "$method" "$section" "Cycle attachment requirements"
                require_method_field "$method" "$section" "Funding success/failure behavior"
                ;;
        esac
    }

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

        validate_no_placeholder_method_evidence "$method" "$section"

        for field in "${required_common_method_fields[@]}"; do
            require_method_field "$method" "$section" "$field"
        done
        require_method_source_commit_sha "$method" "$section"
        validate_method_specific_fields "$method" "$section"
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
            echo "blob-storage inventory missing Toko interoperability section" >&2
            failed=1
            return
        fi

        if ! grep -q "^Status: \*\*Complete\*\*$" <<<"$section"; then
            echo "blob-storage Toko interoperability notes are not complete" >&2
            failed=1
        fi

        if grep -q "TBD" <<<"$section"; then
            echo "blob-storage Toko interoperability notes still have TBD fields" >&2
            failed=1
        fi

        for field in "${required_toko_fields[@]}"; do
            if ! grep -Eq "^- ${field}: .+$" <<<"$section"; then
                echo "blob-storage Toko interoperability notes missing required field: $field" >&2
                failed=1
            fi
        done

        if grep -Eiq "^- [^:]+: *(TODO|unknown|unresolved|missing source|placeholder|source-backed evidence)([[:space:].,;]|$)" <<<"$section"; then
            echo "blob-storage Toko interoperability notes still have placeholder evidence" >&2
            failed=1
        fi

        if ! grep -Eq "^- Source commit SHA: [0-9a-fA-F]{40}([0-9a-fA-F]{24})?$" <<<"$section"; then
            echo "blob-storage Toko interoperability notes have invalid source commit SHA" >&2
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

mapfile -t cargo_files < <(find Cargo.toml crates canisters apps -name Cargo.toml -print)

check_forbidden \
    "blob-storage feature or dependency metadata before inventory completion" \
    rg -n "blob-storage|blob_storage" "${cargo_files[@]}"

check_forbidden \
    "blob_storage source/module path before inventory completion" \
    find crates canisters apps -path "*/blob_storage*" -print -quit

check_forbidden \
    "gateway method literal before inventory completion" \
    rg -n "_immutableObjectStorage" crates canisters apps \
        --glob "*.rs" \
        --glob "*.did" \
        --glob "*.toml"

check_forbidden \
    "internal blob-storage API/model type before inventory completion" \
    rg -n "BlobStorageApi|BlobRootHash|BlobStorage" crates canisters apps \
        --glob "*.rs" \
        --glob "*.did"

check_forbidden \
    "blob-storage billing/Cashier implementation surface before inventory completion" \
    rg -n "blob-storage-billing|blob_storage_billing|cashier|Cashier|account_balance_get_v1|account_top_up_v1|storage_gateway_principal_list_v1|get_blob_storage_status" \
        crates canisters apps \
        --glob "*.rs" \
        --glob "*.did" \
        --glob "*.toml"

if (( failed != 0 )); then
    echo "complete $inventory before adding blob-storage implementation files" >&2
    exit 1
fi
