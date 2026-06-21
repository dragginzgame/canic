use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use candid::types::internal::TypeContainer;
use candid::{decode_one, encode_one};
use candid_parser::utils::CandidSource;
#[cfg(feature = "blob-storage-billing")]
use canic::dto::blob_storage::{
    BlobStorageBillingWarning, BlobStorageCashierAccountBalanceGetError,
    BlobStorageCashierAccountBalanceGetOk, BlobStorageCashierAccountBalanceGetRequest,
    BlobStorageCashierAccountBalanceGetResult, BlobStorageCashierAccountCycleBalances,
    BlobStorageCashierAccountTopUpError, BlobStorageCashierAccountTopUpOk,
    BlobStorageCashierAccountTopUpRequest, BlobStorageCashierAccountTopUpResult,
    BlobStorageCashierDebtTarget, BlobStorageFundingStatus, BlobStorageGatewayPrincipalSyncAction,
    BlobStoragePaymentModelStatus, BlobStorageReadinessBlocker, BlobStorageStatusRequest,
    BlobStorageStatusResponse,
};
use canic::{
    cdk::types::Principal,
    dto::auth::{
        ActiveDelegationProofStatus, ActiveDelegationProofStatusResponse, AuthRequestMetadata,
        DelegatedRoleGrant, DelegationAudience, DelegationCert, DelegationProof,
        IcCanisterSignatureProofV1, IssuerProofAlgorithm, IssuerProofBinding,
        RootDelegationProofBatchEntry, RootDelegationProofBatchGetRequest,
        RootDelegationProofBatchGetResponse, RootDelegationProofBatchInstallRequest,
        RootDelegationProofBatchInstallResponse, RootDelegationProofBatchInstallResult,
        RootDelegationProofBatchPrepareEntry, RootDelegationProofBatchPrepareRequest,
        RootDelegationProofBatchPrepareResponse, RootDelegationProofBatchProof,
        RootDelegationProofBatchProofRef, RootDelegationProofInstallOutcome,
        RootIssuerPolicyResponse, RootIssuerPolicyUpsertRequest, RootIssuerPolicyView, RootProof,
    },
    dto::blob_storage::{BlobStorageLocalCounters, CreateCertificateResult},
    dto::memory::MemoryLedgerResponse,
    ids::CanisterRole,
};

// Returns the repository root so wire-surface fixtures can be read from disk.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory should have a parent")
        .parent()
        .expect("workspace root should exist")
        .to_path_buf()
}

// Reads a checked-in protocol artifact so the test can pin the public surface.
fn read_text(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn assert_candid_roundtrip<T>(value: T)
where
    T: candid::CandidType + for<'de> candid::Deserialize<'de> + Eq + Debug,
{
    let encoded = encode_one(&value).expect("encode Candid value");
    let decoded = decode_one::<T>(&encoded).expect("decode Candid value");
    assert_eq!(decoded, value);
}

fn candid_type_env<T: candid::CandidType>() -> String {
    let mut types = TypeContainer::new();
    types.add::<T>();
    types.env.to_string()
}

fn preceding_attribute<'a>(source: &'a str, signature: &str) -> &'a str {
    source
        .split(signature)
        .next()
        .unwrap_or_else(|| panic!("source should contain {signature}"))
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with("#["))
        .unwrap_or_else(|| panic!("{signature} should have a preceding attribute"))
}

#[test]
fn wasm_store_exposes_standard_cycle_tracker() {
    let did_path = workspace_root().join("crates/canic-wasm-store/wasm_store.did");
    let did = read_text(&did_path);

    assert!(
        did.contains("type PageRequest = record { offset : nat64; limit : nat64 };")
            && did.contains("  canic_cycle_tracker : (PageRequest) -> ("),
        "missing `canic_cycle_tracker` method in {}",
        did_path.display()
    );
    assert!(
        did.contains("type CycleTopupEvent = record")
            && did.contains("  canic_cycle_topups : (PageRequest) -> ("),
        "missing `canic_cycle_topups` method in {}",
        did_path.display()
    );
}

#[test]
fn wasm_store_excludes_default_memory_diagnostics() {
    let did_path = workspace_root().join("crates/canic-wasm-store/wasm_store.did");
    let did = read_text(&did_path);

    assert!(
        !did.contains("type MemoryLedgerResponse = record")
            && !did.contains("  canic_memory_ledger :"),
        "unexpected default `canic_memory_ledger` method in {}",
        did_path.display()
    );
    assert!(
        !did.contains("  canic_memory_registry :"),
        "unexpected `canic_memory_registry` method in {}",
        did_path.display()
    );
}

#[test]
fn wasm_store_canonical_did_parses() {
    let did_path = workspace_root().join("crates/canic-wasm-store/wasm_store.did");
    let did = read_text(&did_path);
    let (env, actor) = CandidSource::Text(&did)
        .load()
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", did_path.display()));

    let actor = actor.unwrap_or_else(|| panic!("missing service in {}", did_path.display()));
    let service = env
        .as_service(&actor)
        .unwrap_or_else(|err| panic!("invalid service in {}: {err}", did_path.display()));

    assert!(
        service
            .iter()
            .all(|(name, _)| name != "canic_memory_ledger"),
        "parsed default wasm_store service must not include canic_memory_ledger"
    );
}

#[test]
fn public_protocol_reexports_wasm_store_root_update_manifest() {
    assert_eq!(
        canic::protocol::CANIC_WASM_STORE_ROOT_UPDATE_METHODS,
        canic_core::protocol::CANIC_WASM_STORE_ROOT_UPDATE_METHODS
    );
    assert_eq!(
        canic::protocol::CANIC_WASM_STORE_STRUCTURAL_QUERY_METHODS,
        canic_core::protocol::CANIC_WASM_STORE_STRUCTURAL_QUERY_METHODS
    );

    for method in canic::protocol::CANIC_WASM_STORE_ROOT_UPDATE_METHODS {
        assert!(!canic::protocol::canic_wasm_store_method_requires_internal_proof(method));
    }
    for method in canic::protocol::CANIC_WASM_STORE_STRUCTURAL_QUERY_METHODS {
        assert!(!canic::protocol::canic_wasm_store_method_requires_internal_proof(method));
    }
}

#[test]
fn blob_storage_gateway_protocol_surface_is_pinned() {
    assert_eq!(
        canic::protocol::BLOB_STORAGE_BLOBS_ARE_LIVE,
        canic_core::protocol::BLOB_STORAGE_BLOBS_ARE_LIVE
    );
    assert_eq!(
        canic::protocol::BLOB_STORAGE_069_GATEWAY_METHODS,
        [
            "_immutableObjectStorageBlobsAreLive",
            "_immutableObjectStorageBlobsToDelete",
            "_immutableObjectStorageConfirmBlobDeletion",
            "_immutableObjectStorageCreateCertificate",
        ]
    );
    let did_path = workspace_root().join("crates/canic/tests/fixtures/blob_storage_gateway.did");
    let did = read_text(&did_path);
    let (env, actor) = CandidSource::Text(&did)
        .load()
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", did_path.display()));
    let actor = actor.unwrap_or_else(|| panic!("missing service in {}", did_path.display()));
    let service = env
        .as_service(&actor)
        .unwrap_or_else(|err| panic!("invalid service in {}: {err}", did_path.display()));

    for method in canic::protocol::BLOB_STORAGE_069_GATEWAY_METHODS {
        assert!(
            service.iter().any(|(name, _)| name == method),
            "blob-storage fixture missing 0.69 method: {method}"
        );
    }
}

#[test]
fn blob_storage_cashier_protocol_surface_is_pinned() {
    assert_eq!(
        canic::protocol::BLOB_STORAGE_CASHIER_ACCOUNT_BALANCE_GET_V1,
        canic_core::protocol::BLOB_STORAGE_CASHIER_ACCOUNT_BALANCE_GET_V1
    );
    assert_eq!(
        canic::protocol::BLOB_STORAGE_CASHIER_ACCOUNT_TOP_UP_V1,
        canic_core::protocol::BLOB_STORAGE_CASHIER_ACCOUNT_TOP_UP_V1
    );
    assert_eq!(
        canic::protocol::BLOB_STORAGE_CASHIER_STORAGE_GATEWAY_PRINCIPAL_LIST_V1,
        canic_core::protocol::BLOB_STORAGE_CASHIER_STORAGE_GATEWAY_PRINCIPAL_LIST_V1
    );
    assert_eq!(
        canic::protocol::BLOB_STORAGE_070_CASHIER_METHODS,
        [
            "account_balance_get_v1",
            "account_top_up_v1",
            "storage_gateway_principal_list_v1",
        ]
    );

    let did_path = workspace_root().join("crates/canic/tests/fixtures/blob_storage_cashier.did");
    let did = read_text(&did_path);
    let (env, actor) = CandidSource::Text(&did)
        .load()
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", did_path.display()));
    let actor = actor.unwrap_or_else(|| panic!("missing service in {}", did_path.display()));
    let service = env
        .as_service(&actor)
        .unwrap_or_else(|err| panic!("invalid service in {}: {err}", did_path.display()));

    for method in canic::protocol::BLOB_STORAGE_070_CASHIER_METHODS {
        assert!(
            service.iter().any(|(name, _)| name == method),
            "Cashier fixture missing method: {method}"
        );
    }
    assert!(
        did.contains("account_top_up_v1 : (\n      opt record")
            && did.contains("storage_gateway_principal_list_v1 : () -> (vec principal);"),
        "Cashier fixture must pin optional top-up request and gateway list response"
    );
}

#[test]
fn blob_storage_billing_gateway_protocol_names_are_pinned() {
    assert_eq!(
        canic::protocol::BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
        canic_core::protocol::BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS
    );
    assert_eq!(
        canic::protocol::BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
        canic_core::protocol::BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES
    );
    assert_eq!(
        canic::protocol::BLOB_STORAGE_STATUS,
        canic_core::protocol::BLOB_STORAGE_STATUS
    );
    assert_eq!(
        canic::protocol::BLOB_STORAGE_070_GATEWAY_METHODS,
        [
            "_immutableObjectStorageUpdateGatewayPrincipals",
            "_immutableObjectStorageFundFromProjectCycles",
        ]
    );
    assert_eq!(
        canic::protocol::BLOB_STORAGE_STATUS,
        "get_blob_storage_status"
    );

    let macro_path =
        workspace_root().join("crates/canic/src/macros/endpoints/blob_storage_billing.rs");
    let source = read_text(&macro_path);
    assert!(
        source.contains("macro_rules! canic_emit_blob_storage_billing_endpoints")
            && source.contains("requires the canic facade feature")
            && source.contains("blob-storage-billing"),
        "blob-storage billing endpoint macro should be opt-in"
    );
    assert!(
        source.contains("name = \"_immutableObjectStorageUpdateGatewayPrincipals\"")
            && source.contains("requires($sync_guard)")
            && source.contains("name = \"_immutableObjectStorageFundFromProjectCycles\"")
            && source.contains("requires($fund_guard)")
            && source.contains("name = \"get_blob_storage_status\"")
            && source.contains("requires($status_guard)"),
        "0.70 billing endpoints must stay update endpoints with separate guards"
    );
}

#[test]
fn blob_storage_gateway_dtos_roundtrip_through_candid() {
    assert_candid_roundtrip(CreateCertificateResult {
        method: "upload".to_string(),
        blob_hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(),
    });
    let create_env = candid_type_env::<CreateCertificateResult>();
    assert!(
        create_env.contains("type CreateCertificateResult = record")
            && create_env.contains("method : text")
            && create_env.contains("blob_hash : text"),
        "CreateCertificateResult Candid changed:\n{create_env}"
    );

    assert_candid_roundtrip(BlobStorageLocalCounters::new(1, 2, 3));
    let counters_env = candid_type_env::<BlobStorageLocalCounters>();
    assert!(
        counters_env.contains("type BlobStorageLocalCounters = record")
            && counters_env.contains("stored_blobs : nat64")
            && counters_env.contains("pending_deletions : nat64")
            && counters_env.contains("gateway_principals : nat64"),
        "BlobStorageLocalCounters Candid changed:\n{counters_env}"
    );
}

#[cfg(feature = "blob-storage-billing")]
fn cashier_balance(total: i64) -> BlobStorageCashierAccountCycleBalances {
    BlobStorageCashierAccountCycleBalances {
        total: candid::Int::from(total),
        cycles_prepaid: candid::Int::from(total),
        cycles_promo: candid::Int::from(0),
        debt_target: BlobStorageCashierDebtTarget::Prepaid,
        cycles_ledger: candid::Int::from(0),
    }
}

#[cfg(feature = "blob-storage-billing")]
#[test]
fn blob_storage_cashier_dtos_roundtrip_through_candid() {
    let account = Principal::from_slice(&[1, 2, 3]);
    assert_candid_roundtrip(BlobStorageCashierAccountBalanceGetRequest { account });
    assert_candid_roundtrip(BlobStorageCashierAccountBalanceGetResult::Ok(
        BlobStorageCashierAccountBalanceGetOk {
            account_cycle_balances: cashier_balance(10),
            account,
        },
    ));
    assert_candid_roundtrip(BlobStorageCashierAccountBalanceGetResult::Err(
        BlobStorageCashierAccountBalanceGetError::AccountNotFound,
    ));

    assert_candid_roundtrip(Some(BlobStorageCashierAccountTopUpRequest {
        target_balance: Some(candid::Nat::from(100_u64)),
        account: Some(account),
    }));
    assert_candid_roundtrip(BlobStorageCashierAccountTopUpResult::Ok(
        BlobStorageCashierAccountTopUpOk {
            balance: cashier_balance(100),
            message: "top-up accepted".to_string(),
        },
    ));
    assert_candid_roundtrip(BlobStorageCashierAccountTopUpResult::Err(
        BlobStorageCashierAccountTopUpError::TopUpWithoutCycles,
    ));
}

#[cfg(feature = "blob-storage-billing")]
#[test]
fn blob_storage_cashier_dto_candid_shapes_are_pinned() {
    let balance_env = candid_type_env::<BlobStorageCashierAccountCycleBalances>();
    assert!(
        balance_env.contains("total : int")
            && balance_env.contains("cycles_prepaid : int")
            && balance_env.contains("debt_target : BlobStorageCashierDebtTarget"),
        "Cashier balance DTO Candid changed:\n{balance_env}"
    );

    let top_up_env = candid_type_env::<BlobStorageCashierAccountTopUpRequest>();
    assert!(
        top_up_env.contains("target_balance : opt nat")
            && top_up_env.contains("account : opt principal"),
        "Cashier top-up request DTO Candid changed:\n{top_up_env}"
    );
}

#[cfg(feature = "blob-storage-billing")]
#[test]
fn blob_storage_status_dtos_roundtrip_through_candid() {
    let cashier = Principal::from_slice(&[4, 5, 6]);
    let project = Principal::from_slice(&[7, 8, 9]);

    assert_candid_roundtrip(BlobStorageStatusRequest {
        sync_gateway_principals: true,
    });
    assert_candid_roundtrip(BlobStorageStatusResponse {
        payment_model: BlobStoragePaymentModelStatus::ProjectAsPaymentAccount,
        cashier_canister_id: Some(cashier),
        payment_account: Some(project),
        cashier_balance: Some(candid::Nat::from(100_u64)),
        min_upload_balance: Some(candid::Nat::from(10_u64)),
        target_upload_balance: Some(candid::Nat::from(100_u64)),
        project_cycles_reserve: Some(candid::Nat::from(1_u64)),
        project_cycles_available: candid::Nat::from(1_000_u64),
        gateway_principal_count: 1,
        last_gateway_principal_sync_at_ns: Some(123),
        gateway_principal_sync_action: BlobStorageGatewayPrincipalSyncAction::SkippedReadOnlyStatus,
        funding_status: BlobStorageFundingStatus::NotNeeded,
        ready: true,
        blockers: Vec::new(),
        warnings: Vec::new(),
    });
    assert_candid_roundtrip(BlobStorageFundingStatus::BalanceMalformed);
    assert_candid_roundtrip(BlobStorageReadinessBlocker::CashierBalanceMalformed);
    assert_candid_roundtrip(BlobStorageBillingWarning::CashierBalanceMalformed);
}

#[cfg(feature = "blob-storage-billing")]
#[test]
fn blob_storage_status_dto_candid_shapes_are_pinned() {
    let status_env = candid_type_env::<BlobStorageStatusResponse>();
    assert!(
        status_env.contains("type BlobStorageStatusResponse = record")
            && status_env.contains("payment_model : BlobStoragePaymentModelStatus")
            && status_env
                .contains("gateway_principal_sync_action : BlobStorageGatewayPrincipalSyncAction")
            && status_env.contains("funding_status : BlobStorageFundingStatus")
            && status_env.contains("blockers : vec BlobStorageReadinessBlocker"),
        "blob-storage status response DTO Candid changed:\n{status_env}"
    );

    let request_env = candid_type_env::<BlobStorageStatusRequest>();
    assert!(
        request_env.contains("sync_gateway_principals : bool"),
        "blob-storage status request DTO Candid changed:\n{request_env}"
    );

    let blocker_env = candid_type_env::<BlobStorageReadinessBlocker>();
    assert!(
        blocker_env.contains("NotConfigured")
            && blocker_env.contains("GatewayPrincipalsMissing")
            && blocker_env.contains("CashierBalanceMalformed")
            && blocker_env.contains("ReserveWouldBeViolated"),
        "blob-storage readiness blocker DTO Candid changed:\n{blocker_env}"
    );

    let funding_env = candid_type_env::<BlobStorageFundingStatus>();
    assert!(
        funding_env.contains("BalanceUnavailable")
            && funding_env.contains("BalanceMalformed")
            && funding_env.contains("ReserveWouldBeViolated"),
        "blob-storage funding status DTO Candid changed:\n{funding_env}"
    );

    let warning_env = candid_type_env::<BlobStorageBillingWarning>();
    assert!(
        warning_env.contains("CashierBalanceUnavailable")
            && warning_env.contains("CashierBalanceMalformed")
            && warning_env.contains("SyncRequestedButStatusIsReadOnly"),
        "blob-storage billing warning DTO Candid changed:\n{warning_env}"
    );
}

#[test]
fn blob_storage_endpoint_macro_emits_only_non_billing_gateway_methods() {
    let endpoint_path = workspace_root().join("crates/canic/src/macros/endpoints/blob_storage.rs");
    let source = read_text(&endpoint_path);

    assert!(
        source.contains("macro_rules! canic_emit_blob_storage_endpoints")
            && source.contains("requires guard = <access expression>")
            && source.contains("requires the canic facade feature")
            && source.contains("blob-storage"),
        "blob-storage endpoint macro should be opt-in and require an explicit guard"
    );

    for method in canic::protocol::BLOB_STORAGE_069_GATEWAY_METHODS {
        assert!(
            source.contains(&format!("name = \"{method}\"")),
            "blob-storage macro must emit gateway method {method}"
        );
    }

    assert!(
        source.contains("canic_query(internal, name = \"_immutableObjectStorageBlobsAreLive\")")
            && source
                .contains("canic_query(internal, name = \"_immutableObjectStorageBlobsToDelete\")")
            && source.contains(
                "canic_update(internal, name = \"_immutableObjectStorageConfirmBlobDeletion\")"
            )
            && source.contains(
                "canic_update(requires($guard), name = \"_immutableObjectStorageCreateCertificate\")"
            ),
        "blob-storage endpoint modes/guards must match the 0.69 gateway contract"
    );

    let live_attr = preceding_attribute(&source, "fn canic_blob_storage_blobs_are_live(");
    let to_delete_attr = preceding_attribute(&source, "fn canic_blob_storage_blobs_to_delete(");
    let confirm_attr = preceding_attribute(&source, "fn canic_blob_storage_confirm_blob_deletion(");
    let create_attr = preceding_attribute(&source, "fn canic_blob_storage_create_certificate(");
    assert!(
        live_attr.contains("canic_query(internal")
            && !live_attr.contains("requires")
            && to_delete_attr.contains("canic_query(internal")
            && !to_delete_attr.contains("requires")
            && confirm_attr.contains("canic_update(internal")
            && !confirm_attr.contains("requires"),
        "liveness and gateway scrubber endpoints must not use the host create-certificate guard"
    );
    assert!(
        create_attr.contains("canic_update(requires($guard)") && !create_attr.contains("internal"),
        "create-certificate must remain the only host-guarded blob-storage endpoint"
    );
    assert!(
        source.contains("pending_deletion_hashes_for_gateway")
            && source.contains("confirm_deleted_by_gateway_hash_bytes_batch"),
        "gateway scrubber endpoints must keep delegating through gateway-aware API helpers"
    );

    assert!(
        !source.contains(concat!(
            "_immutableObjectStorage",
            "UpdateGatewayPrincipals"
        )) && !source.contains(concat!("_immutableObjectStorage", "FundFromProjectCycles")),
        "0.69 endpoint macro must not emit deferred billing/sync gateway methods"
    );
}

#[test]
fn active_delegation_proof_installer_surface_is_issuer_gated() {
    assert_eq!(
        canic::protocol::CANIC_ACTIVE_DELEGATION_PROOF_STATUS,
        canic_core::protocol::CANIC_ACTIVE_DELEGATION_PROOF_STATUS
    );
    assert_eq!(
        canic::protocol::CANIC_INSTALL_ACTIVE_DELEGATION_PROOF,
        canic_core::protocol::CANIC_INSTALL_ACTIVE_DELEGATION_PROOF
    );
    assert_eq!(
        canic::protocol::CANIC_INSTALL_ACTIVE_DELEGATION_PROOF,
        "canic_install_active_delegation_proof"
    );

    let bundle_path = workspace_root().join("crates/canic/src/macros/endpoints/bundles.rs");
    let bundle = read_text(&bundle_path);
    assert!(
        bundle.contains("#[cfg(canic_delegated_token_issuer)]\n        $crate::canic_emit_nonroot_auth_attestation_endpoints!();"),
        "non-root issuer provisioning endpoints must be gated by canic_delegated_token_issuer"
    );

    let endpoint_path = workspace_root().join("crates/canic/src/macros/endpoints/nonroot.rs");
    let endpoint_source = read_text(&endpoint_path);
    let endpoint = endpoint_source
        .split("fn canic_install_active_delegation_proof(")
        .nth(1)
        .expect("non-root auth endpoint should emit active proof installer");
    let prefix = endpoint_source
        .split("fn canic_install_active_delegation_proof(")
        .next()
        .expect("source should have endpoint prefix");
    let preceding_attribute = prefix
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with("#["))
        .expect("active proof installer endpoint should have an attribute");

    assert!(
        preceding_attribute.contains("canic_update")
            && preceding_attribute.contains("caller::is_controller()"),
        "active proof installer must be a controller-gated update endpoint"
    );
    assert!(
        endpoint.contains("InstallActiveDelegationProofRequest")
            && endpoint.contains("InstallActiveDelegationProofResponse")
            && endpoint.contains("AuthApi::install_active_delegation_proof"),
        "active proof installer must call the auth API with the install DTOs"
    );
    assert!(
        endpoint_source.contains("fn canic_active_delegation_proof_status(")
            && endpoint_source.contains("ActiveDelegationProofStatusResponse")
            && endpoint_source.contains("AuthApi::active_delegation_proof_status"),
        "delegated-token issuer bundle must expose active proof status"
    );

    let did_path = workspace_root().join("crates/canic-wasm-store/wasm_store.did");
    let did = read_text(&did_path);
    assert!(
        !did.contains("canic_install_active_delegation_proof")
            && !did.contains("canic_active_delegation_proof_status")
            && !did.contains("canic_prepare_delegated_token")
            && !did.contains("canic_get_delegated_token")
            && !did.contains("type InstallActiveDelegationProofRequest = record")
            && !did.contains("type DelegatedTokenPrepareRequest = record"),
        "canonical wasm_store DID must not expose delegated-token issuer provisioning"
    );
}

#[test]
fn root_delegation_proof_batch_surface_is_pinned() {
    assert_eq!(
        canic::protocol::CANIC_UPSERT_ROOT_ISSUER_POLICY,
        canic_core::protocol::CANIC_UPSERT_ROOT_ISSUER_POLICY
    );
    assert_eq!(
        canic::protocol::CANIC_PREPARE_DELEGATION_PROOF_BATCH,
        canic_core::protocol::CANIC_PREPARE_DELEGATION_PROOF_BATCH
    );
    assert_eq!(
        canic::protocol::CANIC_GET_DELEGATION_PROOF_BATCH,
        canic_core::protocol::CANIC_GET_DELEGATION_PROOF_BATCH
    );
    assert_eq!(
        canic::protocol::CANIC_INSTALL_DELEGATION_PROOF_BATCH,
        canic_core::protocol::CANIC_INSTALL_DELEGATION_PROOF_BATCH
    );
    assert_eq!(
        canic::protocol::CANIC_UPSERT_ROOT_ISSUER_POLICY,
        "canic_upsert_root_issuer_policy"
    );
    assert_eq!(
        canic::protocol::CANIC_PREPARE_DELEGATION_PROOF_BATCH,
        "canic_prepare_delegation_proof_batch"
    );
    assert_eq!(
        canic::protocol::CANIC_GET_DELEGATION_PROOF_BATCH,
        "canic_get_delegation_proof_batch"
    );
    assert_eq!(
        canic::protocol::CANIC_INSTALL_DELEGATION_PROOF_BATCH,
        "canic_install_delegation_proof_batch"
    );

    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/root.rs");
    let source = read_text(&macro_path);
    assert!(
        !source.contains("fn canic_prepare_delegation_proof(")
            && !source.contains("fn canic_get_delegation_proof("),
        "legacy single-proof root delegation endpoints must stay removed"
    );
    let upsert_attr = preceding_attribute(&source, "fn canic_upsert_root_issuer_policy(");
    let prepare_attr = preceding_attribute(&source, "fn canic_prepare_delegation_proof_batch(");
    let get_attr = preceding_attribute(&source, "fn canic_get_delegation_proof_batch(");
    let install_attr = preceding_attribute(&source, "fn canic_install_delegation_proof_batch(");
    assert!(
        upsert_attr.contains("canic_update")
            && upsert_attr.contains("caller::is_controller()")
            && !upsert_attr.contains("internal"),
        "root issuer policy upsert must remain a public controller-gated update"
    );
    assert!(
        prepare_attr.contains("canic_update")
            && prepare_attr.contains("caller::is_controller()")
            && !prepare_attr.contains("internal"),
        "root batch prepare must remain a public controller-gated update"
    );
    assert!(
        get_attr.contains("canic_query")
            && get_attr.contains("caller::is_controller()")
            && !get_attr.contains("internal")
            && !get_attr.contains("caller::is_registered_to_subnet()"),
        "root batch get must remain a direct controller-gated root query"
    );
    assert!(
        install_attr.contains("canic_update")
            && install_attr.contains("caller::is_controller()")
            && !install_attr.contains("internal"),
        "root batch install must remain a public controller-gated update"
    );
    assert!(
        source.contains("fn canic_upsert_root_issuer_policy(")
            && source.contains("RootIssuerPolicyUpsertRequest")
            && source.contains("RootIssuerPolicyResponse")
            && source.contains("AuthApi::upsert_root_issuer_policy_root"),
        "root auth endpoint bundle must expose issuer policy upsert"
    );
    assert!(
        source.contains("fn canic_prepare_delegation_proof_batch(")
            && source.contains("RootDelegationProofBatchPrepareRequest")
            && source.contains("RootDelegationProofBatchPrepareResponse")
            && source.contains("AuthApi::prepare_delegation_proof_batch_root"),
        "root auth endpoint bundle must expose batch prepare"
    );
    assert!(
        source.contains("fn canic_get_delegation_proof_batch(")
            && source.contains("RootDelegationProofBatchGetRequest")
            && source.contains("RootDelegationProofBatchGetResponse")
            && source.contains("AuthApi::get_delegation_proof_batch_root"),
        "root auth endpoint bundle must expose batch get"
    );
    assert!(
        source.contains("fn canic_install_delegation_proof_batch(")
            && source.contains("RootDelegationProofBatchInstallRequest")
            && source.contains("RootDelegationProofBatchInstallResponse")
            && source.contains("AuthApi::install_delegation_proof_batch_root"),
        "root auth endpoint bundle must expose batch install"
    );
}

#[test]
fn root_delegation_proof_batch_dtos_roundtrip_through_candid() {
    let issuer_pid = Principal::from_slice(&[17; 29]);
    let root_pid = Principal::from_slice(&[18; 29]);
    let batch_id = [19; 32];
    let cert_hash = [20; 32];
    let grant = test_delegated_role_grant();
    let audience = DelegationAudience::Project("test".to_string());
    let issuer_policy_request =
        root_issuer_policy_upsert_request(issuer_pid, audience.clone(), grant.clone());
    let issuer_policy_response =
        root_issuer_policy_response(issuer_pid, audience.clone(), grant.clone());
    let prepare_entry =
        root_delegation_proof_batch_prepare_entry(issuer_pid, audience.clone(), grant.clone());
    let prepare_request = root_delegation_proof_batch_prepare_request(batch_id, &prepare_entry);
    let prepare_response =
        root_delegation_proof_batch_prepare_response(batch_id, issuer_pid, cert_hash);
    let proof_ref = RootDelegationProofBatchProofRef {
        issuer_pid,
        cert_hash,
    };
    let get_request = RootDelegationProofBatchGetRequest {
        batch_id,
        entries: vec![proof_ref],
    };
    let proof = root_delegation_proof(root_pid, issuer_pid, audience, grant);
    let batch_proof = RootDelegationProofBatchProof {
        issuer_pid,
        cert_hash,
        proof,
    };
    let get_response = RootDelegationProofBatchGetResponse {
        batch_id,
        proofs: vec![batch_proof.clone()],
    };
    let install_request = RootDelegationProofBatchInstallRequest {
        batch_id,
        proofs: vec![batch_proof],
    };
    let install_response = RootDelegationProofBatchInstallResponse {
        batch_id,
        outcomes: vec![RootDelegationProofBatchInstallResult {
            issuer_pid,
            cert_hash,
            outcome: RootDelegationProofInstallOutcome::Installed,
        }],
    };
    let status = ActiveDelegationProofStatusResponse {
        status: ActiveDelegationProofStatus::RefreshNeeded,
        root_pid: Some(root_pid),
        issuer_pid: Some(issuer_pid),
        cert_hash: Some(cert_hash),
        expires_at_ns: Some(90),
        refresh_after_ns: Some(72),
    };

    assert_candid_roundtrip(issuer_policy_request);
    assert_candid_roundtrip(issuer_policy_response);
    assert_candid_roundtrip(prepare_entry);
    assert_candid_roundtrip(prepare_request);
    assert_candid_roundtrip(prepare_response);
    assert_candid_roundtrip(get_request);
    assert_candid_roundtrip(get_response);
    assert_candid_roundtrip(install_request);
    assert_candid_roundtrip(install_response);
    assert_candid_roundtrip(status);
}

fn test_delegated_role_grant() -> DelegatedRoleGrant {
    DelegatedRoleGrant {
        target: CanisterRole::new("test"),
        scopes: vec!["verify".to_string()],
    }
}

fn root_issuer_policy_upsert_request(
    issuer_pid: Principal,
    audience: DelegationAudience,
    grant: DelegatedRoleGrant,
) -> RootIssuerPolicyUpsertRequest {
    RootIssuerPolicyUpsertRequest {
        issuer_pid,
        enabled: true,
        allowed_audiences: vec![audience],
        allowed_grants: vec![grant],
        max_cert_ttl_ns: 60,
        refresh_after_ratio_bps: 8_000,
    }
}

fn root_issuer_policy_response(
    issuer_pid: Principal,
    audience: DelegationAudience,
    grant: DelegatedRoleGrant,
) -> RootIssuerPolicyResponse {
    RootIssuerPolicyResponse {
        issuer: RootIssuerPolicyView {
            issuer_pid,
            enabled: true,
            allowed_audiences: vec![audience],
            allowed_grants: vec![grant],
            max_cert_ttl_ns: 60,
            refresh_after_ratio_bps: 8_000,
        },
    }
}

fn root_delegation_proof_batch_prepare_entry(
    issuer_pid: Principal,
    audience: DelegationAudience,
    grant: DelegatedRoleGrant,
) -> RootDelegationProofBatchPrepareEntry {
    RootDelegationProofBatchPrepareEntry {
        issuer_pid,
        aud: audience,
        grants: vec![grant],
        cert_ttl_ns: 60,
    }
}

fn root_delegation_proof_batch_prepare_request(
    batch_id: [u8; 32],
    entry: &RootDelegationProofBatchPrepareEntry,
) -> RootDelegationProofBatchPrepareRequest {
    RootDelegationProofBatchPrepareRequest {
        metadata: Some(AuthRequestMetadata {
            request_id: batch_id,
            ttl_ns: 30,
        }),
        entries: vec![entry.clone()],
    }
}

fn root_delegation_proof_batch_prepare_response(
    batch_id: [u8; 32],
    issuer_pid: Principal,
    cert_hash: [u8; 32],
) -> RootDelegationProofBatchPrepareResponse {
    RootDelegationProofBatchPrepareResponse {
        batch_id,
        entries: vec![RootDelegationProofBatchEntry {
            issuer_pid,
            cert_hash,
            expires_at_ns: 90,
            refresh_after_ns: 72,
        }],
        retrieval_expires_at_ns: 45,
    }
}

fn root_delegation_proof(
    root_pid: Principal,
    issuer_pid: Principal,
    audience: DelegationAudience,
    grant: DelegatedRoleGrant,
) -> DelegationProof {
    DelegationProof {
        cert: DelegationCert {
            root_pid,
            issuer_pid,
            issuer_proof_alg: IssuerProofAlgorithm::IcCanisterSignatureV1,
            issuer_proof_binding_hash: [21; 32],
            issuer_proof_binding: IssuerProofBinding::IcCanisterSignatureV1 {
                seed_hash: [22; 32],
            },
            issued_at_ns: 1,
            not_before_ns: 1,
            expires_at_ns: 90,
            max_token_ttl_ns: 10,
            aud: audience,
            grants: vec![grant],
        },
        root_proof: RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
            signature_cbor: vec![1, 2, 3],
            public_key_der: vec![4, 5, 6],
        }),
    }
}

#[test]
fn root_role_attestation_prepare_get_surface_is_pinned() {
    assert_eq!(
        canic::protocol::CANIC_PREPARE_ROLE_ATTESTATION,
        canic_core::protocol::CANIC_PREPARE_ROLE_ATTESTATION
    );
    assert_eq!(
        canic::protocol::CANIC_GET_ROLE_ATTESTATION,
        canic_core::protocol::CANIC_GET_ROLE_ATTESTATION
    );
    assert_eq!(
        canic::protocol::CANIC_PREPARE_ROLE_ATTESTATION,
        "canic_prepare_role_attestation"
    );
    assert_eq!(
        canic::protocol::CANIC_GET_ROLE_ATTESTATION,
        "canic_get_role_attestation"
    );

    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/root.rs");
    let source = read_text(&macro_path);
    assert!(
        source.contains("fn canic_prepare_role_attestation(")
            && source.contains("RoleAttestationPrepareResponse")
            && source.contains("AuthApi::prepare_role_attestation_root"),
        "root auth endpoint bundle must expose role-attestation prepare"
    );
    assert!(
        source.contains("fn canic_get_role_attestation(")
            && source.contains("RoleAttestationGetRequest")
            && source.contains("AuthApi::get_role_attestation_root"),
        "root auth endpoint bundle must expose role-attestation get"
    );
}

#[test]
fn memory_ledger_diagnostic_bypasses_normal_dispatch() {
    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/shared.rs");
    let source = read_text(&macro_path);
    let endpoint = source
        .split("fn canic_memory_ledger()")
        .nth(1)
        .expect("memory ledger endpoint should exist");
    let prefix = source
        .split("fn canic_memory_ledger()")
        .next()
        .expect("source should have endpoint prefix");
    let preceding_attribute = prefix
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with("#["))
        .expect("memory ledger endpoint should have an attribute");

    assert!(
        preceding_attribute.contains("$crate::cdk::query"),
        "memory ledger diagnostic must use a raw query attribute in {}",
        macro_path.display()
    );
    assert!(
        !preceding_attribute.contains("canic_query"),
        "memory ledger diagnostic must not use normal Canic query dispatch in {}",
        macro_path.display()
    );
    assert!(
        endpoint.contains("$crate::cdk::api::is_controller")
            && endpoint.contains("MemoryQuery::ledger()"),
        "memory ledger diagnostic must be controller-gated and read the restricted ledger path"
    );
}

#[test]
fn memory_ledger_dto_candid_shape_includes_backing_memory_size() {
    let ledger_env = candid_type_env::<MemoryLedgerResponse>();

    assert!(
        ledger_env.contains("memories : vec MemoryLedgerMemoryEntry")
            && ledger_env.contains("type MemoryLedgerMemoryEntry = record")
            && ledger_env.contains("memory_manager_id : nat8")
            && ledger_env.contains("stable_key : text")
            && ledger_env.contains("state : MemoryAllocationState")
            && ledger_env.contains("size : MemoryAllocationSizeEntry")
            && ledger_env.contains("memory_size : opt MemoryAllocationSizeEntry")
            && ledger_env.contains("type MemoryAllocationSizeEntry = record")
            && ledger_env.contains("wasm_pages : nat64")
            && ledger_env.contains("bytes : nat64"),
        "memory ledger DTO Candid changed:\n{ledger_env}"
    );
}

#[test]
fn memory_ledger_is_config_gated() {
    let bundle_path = workspace_root().join("crates/canic/src/macros/endpoints/bundles.rs");
    let bundles = read_text(&bundle_path);
    let shared_bundle = bundles
        .split("macro_rules! canic_bundle_shared_runtime_endpoints")
        .nth(1)
        .and_then(|rest| {
            rest.split("macro_rules! canic_bundle_root_only_endpoints")
                .next()
        })
        .expect("shared runtime bundle should exist");
    let wasm_store_bundle = bundles
        .split("macro_rules! canic_bundle_wasm_store_runtime_endpoints")
        .nth(1)
        .expect("wasm_store runtime bundle should exist");

    assert!(
        shared_bundle.contains("#[cfg(canic_memory_ledger_enabled)]")
            && shared_bundle.contains("canic_emit_memory_ledger_diagnostic_endpoint!"),
        "shared runtime bundle must config-gate the ABI ledger recovery endpoint"
    );
    assert!(
        wasm_store_bundle.contains("#[cfg(canic_memory_ledger_enabled)]")
            && wasm_store_bundle.contains("canic_emit_memory_ledger_diagnostic_endpoint!"),
        "wasm_store runtime bundle must config-gate the ABI ledger recovery endpoint"
    );
    assert!(
        !shared_bundle.contains("canic_emit_memory_observability_endpoints!"),
        "live memory registry diagnostics must not be in the default bundle"
    );
}

#[test]
fn missing_finish_marker_stays_actionable() {
    let macro_path = workspace_root().join("crates/canic/src/macros/start.rs");
    let source = read_text(&macro_path);
    let marker = "__canic_missing_finish_macro_add_canic_finish_at_end_after_all_endpoints";

    assert!(
        source.contains(&format!("const _: fn() = {marker};")),
        "lifecycle start macros must reference an actionable missing-finish marker"
    );
    assert!(
        source.contains(&format!("fn {marker}()")),
        "finish! must define the same missing-finish marker"
    );
    assert!(
        marker.contains("missing_finish_macro")
            && marker.contains("add_canic_finish")
            && marker.contains("after_all_endpoints"),
        "missing-finish marker should read like a compiler-error hint"
    );
}
