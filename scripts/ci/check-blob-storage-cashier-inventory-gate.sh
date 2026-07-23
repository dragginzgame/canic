#!/usr/bin/env bash
set -euo pipefail

inventory="${BLOB_STORAGE_CASHIER_INVENTORY:-docs/contracts/BLOB_STORAGE_CASHIER_INVENTORY.md}"
required_methods=(
    "account_balance_get_v1"
    "account_top_up_v1"
    "storage_gateway_principal_list_v1"
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
    echo "blob-storage Cashier inventory is missing: $inventory" >&2
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
            echo "blob-storage Cashier inventory method missing required field: $method: $field" >&2
            failed=1
        fi
    }

    validate_method_specific_fields() {
        local method="$1"
        local section="$2"

        case "$method" in
            "storage_gateway_principal_list_v1")
                require_method_field "$method" "$section" "Empty-list behavior"
                if ! grep -Eiq "^- Empty-list behavior: .*malformed.*preserv" <<<"$section"; then
                    echo "blob-storage Cashier inventory method has invalid empty-list behavior: $method" >&2
                    failed=1
                fi
                require_method_field "$method" "$section" "Duplicate-principal behavior"
                require_method_field "$method" "$section" "Anonymous-principal behavior"
                require_method_field "$method" "$section" "Management-canister-principal behavior"
                require_method_field "$method" "$section" "Malformed response behavior expected from Canic wrappers"
                ;;
        esac
    }

    validate_complete_method_section() {
        local method="$1"
        local section

        if ! rg -q "^### \`$method\`$" "$inventory"; then
            echo "blob-storage Cashier inventory missing method section: $method" >&2
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
            echo "blob-storage Cashier inventory method is not complete: $method" >&2
            failed=1
        fi

        if grep -q "TBD" <<<"$section"; then
            echo "blob-storage Cashier inventory method still has TBD fields: $method" >&2
            failed=1
        fi

        validate_method_specific_fields "$method" "$section"
    }

    validate_complete_optional_section() {
        local section

        section="$(
            awk '
                $0 == "## Optional Cashier Methods" { in_section = 1; print; next }
                in_section && /^## / { exit }
                in_section { print }
            ' "$inventory"
        )"

        if [[ -z "$section" ]]; then
            echo "blob-storage Cashier inventory missing optional methods section" >&2
            failed=1
            return
        fi

        if ! grep -q "^Status: \*\*Complete\*\*$" <<<"$section"; then
            echo "blob-storage Cashier optional methods section is not complete" >&2
            failed=1
        fi

        if grep -q "TBD" <<<"$section"; then
            echo "blob-storage Cashier optional methods section still has TBD fields" >&2
            failed=1
        fi
    }

    for method in "${required_methods[@]}"; do
        validate_complete_method_section "$method"
    done
    validate_complete_optional_section

    if (( failed != 0 )); then
        echo "blob-storage Cashier inventory is marked Complete but required evidence is incomplete" >&2
        exit 1
    fi

    exit 0
fi

echo "blob-storage Cashier inventory status is '$status'; billing implementation remains gated" >&2

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
        echo "forbidden blob-storage Cashier implementation surface found: $description" >&2
        failed=1
    fi
}

mapfile -t cargo_files < <(find Cargo.toml crates canisters apps -name Cargo.toml -print)

check_forbidden \
    "blob-storage billing feature or dependency metadata before Cashier inventory completion" \
    rg -n "blob-storage-billing|blob_storage_billing" "${cargo_files[@]}"

check_forbidden \
    "blob-storage billing or Cashier source/module path before Cashier inventory completion" \
    find crates canisters apps \( -path "*/blob_storage_billing*" -o -path "*/cashier*" \) \
        -print \
        -quit

check_forbidden \
    "Cashier method literal before Cashier inventory completion" \
    rg -n "account_balance_get_v1|account_top_up_v1|storage_gateway_principal_list_v1" \
        crates canisters apps \
        --glob "*.rs" \
        --glob "*.did" \
        --glob "*.toml"

check_forbidden \
    "billing endpoint literal before Cashier inventory completion" \
    rg -n "get_blob_storage_status|_immutableObjectStorageUpdateGatewayPrincipals|_immutableObjectStorageFundFromProjectCycles" \
        crates canisters apps \
        --glob "*.rs" \
        --glob "*.did" \
        --glob "*.toml"

check_forbidden \
    "public Cashier/billing API or model type before Cashier inventory completion" \
    rg -n "Cashier|BlobStorageBilling|BlobStorageStatus|BlobStorageFunding|GatewayPrincipalSync" \
        crates canisters apps \
        --glob "*.rs" \
        --glob "*.did"

if (( failed != 0 )); then
    echo "complete $inventory before adding blob-storage billing implementation files" >&2
    exit 1
fi
