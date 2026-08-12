use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use candid::types::internal::TypeContainer;
use candid::{Principal, decode_one, encode_one};
use candid_parser::utils::CandidSource;
#[cfg(feature = "blob-storage-billing")]
use canic::dto::blob_storage::{
    BlobProjectCyclesTopUpReport, BlobStorageBillingConfig, BlobStorageBillingWarning,
    BlobStorageCashierAccountBalanceGetError, BlobStorageCashierAccountBalanceGetOk,
    BlobStorageCashierAccountBalanceGetRequest, BlobStorageCashierAccountBalanceGetResult,
    BlobStorageCashierAccountCycleBalances, BlobStorageCashierAccountTopUpError,
    BlobStorageCashierAccountTopUpOk, BlobStorageCashierAccountTopUpRequest,
    BlobStorageCashierAccountTopUpResult, BlobStorageCashierDebtTarget, BlobStorageFundingStatus,
    BlobStorageGatewayPrincipalSyncAction, BlobStoragePaymentModelStatus,
    BlobStorageReadinessBlocker, BlobStorageStatusRequest, BlobStorageStatusResponse,
};
use canic::{
    api::protocol::icrc21::Icrc21Dispatcher,
    dto::auth::{
        ActiveDelegationProofStatus, ActiveDelegationProofStatusResponse, ChainKeyAlgorithm,
        ChainKeyBatchHeaderV1, ChainKeyBatchWitnessStepV1, ChainKeyBatchWitnessV1,
        ChainKeyDelegationCertV1, ChainKeyKeyId, ChainKeyRootSignatureV1, DelegatedRoleGrant,
        DelegationAudience, DelegationCert, DelegationProof, IcChainKeyBatchSignatureProofV1,
        IssuerProofAlgorithm, IssuerProofBinding, RootDelegationProofBatchProof,
        RootIssuerPolicyResponse, RootIssuerPolicyUpsertRequest, RootIssuerPolicyView,
        RootIssuerRenewalBatchStatus, RootIssuerRenewalBatchView, RootIssuerRenewalStateView,
        RootIssuerRenewalStatusRequest, RootIssuerRenewalStatusResponse,
        RootIssuerRenewalTemplateResponse, RootIssuerRenewalTemplateUpsertRequest,
        RootIssuerRenewalTemplateView, RootProof,
    },
    dto::blob_storage::{BlobStorageLocalCounters, CreateCertificateResult},
    dto::cascade::StateSnapshotInput,
    dto::component_provisioning::{
        FleetComponentProvisioningAdvanceRequest, FleetComponentProvisioningPrepareRequest,
        FleetComponentProvisioningStatusResponse, RootComponentActivationRequest,
        RootComponentPublicationRequest,
    },
    dto::cycles::Cycles,
    dto::env::{EnvBootstrapArgs, EnvSnapshotResponse},
    dto::fleet_activation::FleetActivationStatusResponse,
    dto::fleet_registry::{
        FleetSubnetRootDrainingReservationRequest, FleetSubnetRootDrainingReservationResponse,
        FleetSubnetRootDrainingReservationStatusRequest,
    },
    dto::icp_refill::{IcpRefillDryRun, IcpRefillRequest},
    dto::icrc21::{
        ConsentInfo, ConsentMessage, ConsentMessageMetadata, ConsentMessageRequest,
        ConsentMessageResponse, ConsentMessageSpec, DisplayMessageType,
    },
    dto::memory::MemoryLedgerResponse,
    dto::rpc::Response as RootRpcResponse,
    dto::runtime::{
        CanicHealthStatus, CanicReadinessStatus, CanicRuntimeStatus, RecentFailure,
        RuntimeFieldVisibility,
    },
    dto::state::{FleetCommand, FleetCommandResponse, FleetMode, FleetStateResponse},
    ids::{CanisterRole, CanonicalNetworkId, FleetId, FleetKey},
};

fn test_fleet() -> FleetKey {
    FleetKey {
        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
        fleet_id: FleetId::from_generated_bytes([1; 32]),
    }
}

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

#[test]
fn fleet_state_and_internal_cascade_candid_shapes_use_the_current_contract() {
    assert_eq!(canic::protocol::CANIC_FLEET_ADMIN, "canic_fleet_admin");
    assert_eq!(canic::protocol::CANIC_FLEET_STATE, "canic_fleet_state");

    let command_env = candid_type_env::<FleetCommand>();
    assert!(command_env.contains("FleetCommand"));
    let response_env = candid_type_env::<FleetCommandResponse>();
    assert!(response_env.contains("FleetCommandResponse"));
    let state_env = candid_type_env::<FleetStateResponse>();
    assert!(state_env.contains("FleetStateResponse"));
    assert!(state_env.contains("FleetMode"));

    let cascade_env = candid_type_env::<StateSnapshotInput>();
    assert!(
        cascade_env.contains("fleet_state"),
        "state cascade Candid must contain fleet_state"
    );

    assert_candid_roundtrip(FleetMode::Readonly);
}

#[test]
fn environment_candid_shapes_use_fleet_subnet_root_and_component_spec() {
    for env in [
        candid_type_env::<EnvBootstrapArgs>(),
        candid_type_env::<EnvSnapshotResponse>(),
    ] {
        assert!(
            env.contains("fleet_subnet_root_pid : opt principal")
                && env.contains("component_spec : opt text"),
            "environment Candid must expose Fleet Subnet Root and Component Spec identity:\n{env}"
        );
    }
}

#[test]
fn root_rpc_commands_without_result_data_use_unit_variants() {
    for response in [
        RootRpcResponse::AcknowledgePlacementReceipt,
        RootRpcResponse::RecycleCanister,
    ] {
        let encoded = encode_one(&response).expect("encode root RPC response");
        let decoded =
            decode_one::<RootRpcResponse>(&encoded).expect("decode root RPC unit response");
        assert_eq!(
            std::mem::discriminant(&decoded),
            std::mem::discriminant(&response)
        );
    }

    let env = candid_type_env::<RootRpcResponse>();
    assert!(env.contains("AcknowledgePlacementReceipt"));
    assert!(env.contains("RecycleCanister"));
}

#[test]
fn root_capability_surface_uses_component_registry_authority() {
    assert_eq!(
        canic::protocol::CANIC_RESPONSE_CAPABILITY_V1,
        "canic_response_capability_v1"
    );

    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/shared.rs");
    let source = read_text(&macro_path);
    let attribute = preceding_attribute_context(&source, "fn canic_response_capability_v1(");
    assert!(attribute.contains("canic_update(internal, requires(custom("));
    assert!(attribute.contains("RootCapabilityCallerPredicate"));
    assert!(
        source.contains("ComponentRpcApi::response_capability_v1_root(envelope)"),
        "root capability endpoint must delegate through the control-plane authority facade"
    );
}

fn consent_message_request(method: &str) -> ConsentMessageRequest {
    ConsentMessageRequest {
        method: method.to_string(),
        arg: vec![1, 2, 3],
        user_preferences: ConsentMessageSpec {
            metadata: ConsentMessageMetadata {
                language: "en".to_string(),
                utc_offset_minutes: Some(60),
            },
            device_spec: Some(DisplayMessageType::GenericDisplay),
        },
    }
}

#[test]
fn semantic_protocol_and_cycle_types_are_public() {
    assert_candid_roundtrip(consent_message_request("transfer"));

    let cycles = Cycles::new(42);
    assert_eq!(cycles.to_u128(), 42);
}

#[test]
fn icrc21_dispatcher_uses_the_registered_typed_handler() {
    let method = "protocol_surface_transfer";
    Icrc21Dispatcher::register(method, |request| {
        ConsentMessageResponse::Ok(ConsentInfo {
            consent_message: ConsentMessage::GenericDisplayMessage(request.method),
            metadata: request.user_preferences.metadata,
        })
    });

    let ConsentMessageResponse::Ok(info) =
        Icrc21Dispatcher::consent_message(consent_message_request(method))
    else {
        panic!("registered handler should return consent information");
    };
    assert_eq!(
        info.consent_message,
        ConsentMessage::GenericDisplayMessage(method.to_string())
    );
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

fn preceding_attribute_context(source: &str, signature: &str) -> String {
    let before = source
        .split(signature)
        .next()
        .unwrap_or_else(|| panic!("source should contain {signature}"));
    let mut lines = before.lines().rev().take(6).collect::<Vec<_>>();
    lines.reverse();
    lines.join("\n")
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
    assert!(
        did.contains("type FleetKey = record {")
            && did.contains("canonical_network_id : text")
            && did.contains("fleet_id : text")
            && !did.contains("type FleetKey = record { network : text;"),
        "canonical Wasm-store DID must expose the exact FleetKey member names"
    );
    assert!(
        did.contains("type CanisterInitAuthority = variant")
            && did.contains("authority : CanisterInitAuthority;")
            && did.contains(
                "type StateSnapshotInput = record { fleet_state : opt FleetStateInput };"
            )
            && !did.contains("FleetDirectoryInput")
            && !did.contains("fleet_directory"),
        "canonical Wasm-store DID must expose current managed init and state-cascade authority"
    );
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
    assert!(
        service
            .iter()
            .any(|(name, _)| name == canic::protocol::CANIC_FLEET_ACTIVATION_STATUS),
        "parsed default wasm_store service must include the canonical Fleet activation status query"
    );
    for endpoint in [
        canic::protocol::CANIC_PREPARE_FLEET_CREDENTIAL_GENERATION,
        canic::protocol::CANIC_ACTIVATE_FLEET,
    ] {
        assert!(
            service.iter().any(|(name, _)| name == endpoint),
            "parsed default wasm_store service must include {endpoint}"
        );
    }

    let status_env = candid_type_env::<FleetActivationStatusResponse>();
    assert!(status_env.contains("FleetActivationStatusResponse"));
    assert!(status_env.contains("FleetActivationIdentity"));
    assert!(status_env.contains("FleetCascadeActivationEvidence"));
    assert!(status_env.contains("FleetCredentialManifest"));
}

#[test]
fn fleet_coordinator_canonical_did_parses() {
    let did_path = workspace_root().join("crates/canic-fleet-coordinator/fleet_coordinator.did");
    let did = read_text(&did_path);
    let (env, actor) = CandidSource::Text(&did)
        .load()
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", did_path.display()));

    let actor = actor.unwrap_or_else(|| panic!("missing service in {}", did_path.display()));
    let service = env
        .as_service(&actor)
        .unwrap_or_else(|err| panic!("invalid service in {}: {err}", did_path.display()));

    for endpoint in [
        canic::protocol::CANIC_FLEET_COMPONENT_PROVISIONING_PREPARE,
        canic::protocol::CANIC_FLEET_COMPONENT_PROVISIONING_STATUS,
        canic::protocol::CANIC_FLEET_REGISTRY,
        canic::protocol::CANIC_FLEET_REGISTRY_MANIFEST,
        canic::protocol::CANIC_FLEET_REGISTRY_VERSION,
        canic::protocol::CANIC_FLEET_SUBNET_ROOT_JOIN,
    ] {
        assert!(
            service.iter().any(|(name, _)| name == endpoint),
            "parsed Fleet Coordinator service must include {endpoint}"
        );
    }
}

#[test]
fn fleet_activation_status_is_a_controller_query_on_the_shared_runtime_surface() {
    assert_eq!(
        canic::protocol::CANIC_FLEET_ACTIVATION_STATUS,
        "canic_fleet_activation_status"
    );

    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/shared.rs");
    let source = read_text(&macro_path);
    let attribute =
        preceding_attribute_context(&source, "async fn canic_fleet_activation_status()");

    assert!(
        attribute.contains("canic_query(requires(caller::is_controller()))"),
        "Fleet activation status must remain a controller-guarded query"
    );
}

#[test]
fn fleet_subnet_root_authority_is_a_controller_query_on_the_root_surface() {
    assert_eq!(
        canic::protocol::CANIC_FLEET_SUBNET_ROOT_AUTHORITY,
        "canic_fleet_subnet_root_authority"
    );

    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/root.rs");
    let source = read_text(&macro_path);
    let attribute =
        preceding_attribute_context(&source, "async fn canic_fleet_subnet_root_authority(");

    assert!(
        attribute.contains("canic_query(requires(caller::is_controller()))"),
        "Fleet Subnet Root authority must remain a controller-guarded query"
    );
}

#[test]
fn fleet_subnet_wasm_store_authority_is_a_controller_query_on_the_store_surface() {
    assert_eq!(
        canic::protocol::CANIC_FLEET_SUBNET_WASM_STORE_AUTHORITY,
        "canic_fleet_subnet_wasm_store_authority"
    );

    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/wasm_store.rs");
    let source = read_text(&macro_path);
    let attribute =
        preceding_attribute_context(&source, "async fn canic_fleet_subnet_wasm_store_authority(");

    assert!(
        attribute.contains("canic_query(requires(caller::is_controller()))"),
        "Fleet Subnet Wasm Store authority must remain a controller-guarded query"
    );
}

#[test]
fn fleet_subnet_root_canister_summary_is_a_controller_query_on_the_root_surface() {
    assert_eq!(
        canic::protocol::CANIC_FLEET_SUBNET_ROOT_CANISTER_SUMMARY,
        canic_core::protocol::CANIC_FLEET_SUBNET_ROOT_CANISTER_SUMMARY
    );
    assert_eq!(
        canic::protocol::CANIC_FLEET_SUBNET_ROOT_CANISTER_SUMMARY,
        "canic_fleet_subnet_root_canister_summary"
    );

    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/root.rs");
    let source = read_text(&macro_path);
    let attribute = preceding_attribute_context(
        &source,
        "async fn canic_fleet_subnet_root_canister_summary(",
    );

    assert!(
        attribute.contains("canic_query(requires(caller::is_controller()))"),
        "Fleet Subnet Root Canister summary must remain a controller-guarded query"
    );
}

#[test]
fn canister_pool_inventory_and_admin_are_controller_guarded_root_surfaces() {
    assert_eq!(canic::protocol::CANIC_POOL_LIST, "canic_pool_list");
    assert_eq!(canic::protocol::CANIC_POOL_ADMIN, "canic_pool_admin");

    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/root.rs");
    let source = read_text(&macro_path);
    for endpoint in ["async fn canic_pool_list(", "async fn canic_pool_admin("] {
        let attribute = preceding_attribute_context(&source, endpoint);
        assert!(
            attribute.contains("requires(caller::is_controller())"),
            "{endpoint} must remain controller guarded",
        );
    }
}

#[test]
fn fleet_subnet_root_draining_is_controller_guarded_on_the_root_surface() {
    assert_eq!(
        canic::protocol::CANIC_FLEET_SUBNET_ROOT_DRAINING_BEGIN,
        canic_core::protocol::CANIC_FLEET_SUBNET_ROOT_DRAINING_BEGIN
    );
    assert_eq!(
        canic::protocol::CANIC_FLEET_SUBNET_ROOT_DRAINING_INVENTORY_FINALIZE,
        canic_core::protocol::CANIC_FLEET_SUBNET_ROOT_DRAINING_INVENTORY_FINALIZE
    );
    assert_eq!(
        canic::protocol::CANIC_FLEET_SUBNET_ROOT_DRAINING_INVENTORY_STATUS,
        canic_core::protocol::CANIC_FLEET_SUBNET_ROOT_DRAINING_INVENTORY_STATUS
    );
    assert_eq!(
        canic::protocol::CANIC_FLEET_SUBNET_ROOT_DRAINING_STATUS,
        canic_core::protocol::CANIC_FLEET_SUBNET_ROOT_DRAINING_STATUS
    );
    assert_eq!(
        canic::protocol::CANIC_FLEET_SUBNET_ROOT_REMOVAL_PUBLISH,
        canic_core::protocol::CANIC_FLEET_SUBNET_ROOT_REMOVAL_PUBLISH
    );
    assert_eq!(
        canic::protocol::CANIC_FLEET_SUBNET_ROOT_REMOVAL_STATUS,
        canic_core::protocol::CANIC_FLEET_SUBNET_ROOT_REMOVAL_STATUS
    );
    assert_eq!(
        canic::protocol::CANIC_FLEET_SUBNET_ROOT_STORE_RECLAIM,
        canic_core::protocol::CANIC_FLEET_SUBNET_ROOT_STORE_RECLAIM
    );
    assert_eq!(
        canic::protocol::CANIC_FLEET_SUBNET_ROOT_STORE_RECLAMATION_STATUS,
        canic_core::protocol::CANIC_FLEET_SUBNET_ROOT_STORE_RECLAMATION_STATUS
    );
    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/root.rs");
    let source = read_text(&macro_path);
    let begin =
        preceding_attribute_context(&source, "async fn canic_fleet_subnet_root_draining_begin(");
    let status =
        preceding_attribute_context(&source, "async fn canic_fleet_subnet_root_draining_status(");
    let finalize = preceding_attribute_context(
        &source,
        "async fn canic_fleet_subnet_root_draining_inventory_finalize(",
    );
    let inventory_status = preceding_attribute_context(
        &source,
        "async fn canic_fleet_subnet_root_draining_inventory_status(",
    );
    let removal =
        preceding_attribute_context(&source, "async fn canic_fleet_subnet_root_removal_publish(");
    let removal_status =
        preceding_attribute_context(&source, "async fn canic_fleet_subnet_root_removal_status(");
    let store_reclaim =
        preceding_attribute_context(&source, "async fn canic_fleet_subnet_root_store_reclaim(");
    let store_reclamation_status = preceding_attribute_context(
        &source,
        "async fn canic_fleet_subnet_root_store_reclamation_status(",
    );
    assert!(
        begin.contains("canic_update(requires(caller::is_controller()))"),
        "Fleet Subnet Root draining begin must remain a controller-guarded update"
    );
    assert!(
        status.contains("canic_query(requires(caller::is_controller()))"),
        "Fleet Subnet Root draining status must remain a controller-guarded query"
    );
    assert!(
        finalize.contains("canic_update(requires(caller::is_controller()))"),
        "Fleet Subnet Root final inventory must remain a controller-guarded update"
    );
    assert!(
        inventory_status.contains("canic_query(requires(caller::is_controller()))"),
        "Fleet Subnet Root final inventory status must remain a controller-guarded query"
    );
    assert!(
        removal.contains("canic_update(requires(caller::is_controller()))"),
        "Fleet Subnet Root removal publication must remain a controller-guarded update"
    );
    assert!(
        removal_status.contains("canic_query(requires(caller::is_controller()))"),
        "Fleet Subnet Root removal status must remain a controller-guarded query"
    );
    assert!(
        store_reclaim.contains("canic_update(requires(caller::is_controller()))"),
        "Fleet Subnet Root Store reclamation must remain a controller-guarded update"
    );
    assert!(
        store_reclamation_status.contains("canic_query(requires(caller::is_controller()))"),
        "Fleet Subnet Root Store reclamation status must remain a controller-guarded query"
    );
}

#[test]
fn fleet_subnet_root_store_binding_finalization_is_controller_guarded() {
    assert_eq!(
        canic::protocol::CANIC_FLEET_SUBNET_ROOT_STORE_BINDING_FINALIZE,
        canic_core::protocol::CANIC_FLEET_SUBNET_ROOT_STORE_BINDING_FINALIZE
    );
    assert_eq!(
        canic::protocol::CANIC_FLEET_SUBNET_ROOT_STORE_BINDING_FINALIZATION_STATUS,
        canic_core::protocol::CANIC_FLEET_SUBNET_ROOT_STORE_BINDING_FINALIZATION_STATUS
    );

    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/root.rs");
    let source = read_text(&macro_path);
    let finalize = preceding_attribute_context(
        &source,
        "async fn canic_fleet_subnet_root_store_binding_finalize(",
    );
    let status = preceding_attribute_context(
        &source,
        "async fn canic_fleet_subnet_root_store_binding_finalization_status(",
    );

    assert!(
        finalize.contains("canic_update(requires(caller::is_controller()))"),
        "Fleet Subnet Root Store binding finalization must remain a controller-guarded update"
    );
    assert!(
        status.contains("canic_query(requires(caller::is_controller()))"),
        "Fleet Subnet Root Store binding finalization status must remain a controller-guarded query"
    );
}

#[test]
fn fleet_subnet_root_store_deletion_is_controller_guarded() {
    assert_eq!(
        canic::protocol::CANIC_FLEET_SUBNET_ROOT_STORE_DELETE,
        canic_core::protocol::CANIC_FLEET_SUBNET_ROOT_STORE_DELETE
    );
    assert_eq!(
        canic::protocol::CANIC_FLEET_SUBNET_ROOT_STORE_DELETION_STATUS,
        canic_core::protocol::CANIC_FLEET_SUBNET_ROOT_STORE_DELETION_STATUS
    );

    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/root.rs");
    let source = read_text(&macro_path);
    let deletion =
        preceding_attribute_context(&source, "async fn canic_fleet_subnet_root_store_delete(");
    let status = preceding_attribute_context(
        &source,
        "async fn canic_fleet_subnet_root_store_deletion_status(",
    );

    assert!(
        deletion.contains("canic_update(requires(caller::is_controller()))"),
        "Fleet Subnet Root Store deletion must remain a controller-guarded update"
    );
    assert!(
        status.contains("canic_query(requires(caller::is_controller()))"),
        "Fleet Subnet Root Store deletion status must remain a controller-guarded query"
    );
}

#[test]
fn fleet_subnet_root_physical_deletion_handoff_has_exact_authority_guards() {
    for (facade, core) in [
        (
            canic::protocol::CANIC_FLEET_SUBNET_ROOT_DELETION_PREPARE,
            canic_core::protocol::CANIC_FLEET_SUBNET_ROOT_DELETION_PREPARE,
        ),
        (
            canic::protocol::CANIC_FLEET_SUBNET_ROOT_DELETION_PREPARATION_STATUS,
            canic_core::protocol::CANIC_FLEET_SUBNET_ROOT_DELETION_PREPARATION_STATUS,
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_ROOT_DELETION_READINESS_PREPARE,
            canic_core::protocol::CANIC_FLEET_REGISTRY_ROOT_DELETION_READINESS_PREPARE,
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_ROOT_DELETION_READY,
            canic_core::protocol::CANIC_FLEET_REGISTRY_ROOT_DELETION_READY,
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_ROOT_DELETION_EXECUTION_BEGIN,
            canic_core::protocol::CANIC_FLEET_REGISTRY_ROOT_DELETION_EXECUTION_BEGIN,
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_ROOT_DELETION_EXECUTION_STATUS,
            canic_core::protocol::CANIC_FLEET_REGISTRY_ROOT_DELETION_EXECUTION_STATUS,
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_ROOT_DELETION_COMPLETE,
            canic_core::protocol::CANIC_FLEET_REGISTRY_ROOT_DELETION_COMPLETE,
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_ROOT_DELETION_STATUS,
            canic_core::protocol::CANIC_FLEET_REGISTRY_ROOT_DELETION_STATUS,
        ),
    ] {
        assert_eq!(facade, core);
    }

    let root = read_text(&workspace_root().join("crates/canic/src/macros/endpoints/root.rs"));
    for signature in [
        "async fn canic_fleet_subnet_root_deletion_prepare(",
        "async fn canic_fleet_subnet_root_deletion_preparation_status(",
    ] {
        assert!(
            preceding_attribute_context(&root, signature)
                .contains("requires(caller::is_controller())"),
            "{signature} must remain controller guarded"
        );
    }

    let coordinator =
        read_text(&workspace_root().join("crates/canic/src/macros/endpoints/fleet_coordinator.rs"));
    for signature in [
        "async fn canic_fleet_registry_root_deletion_readiness_prepare(",
        "async fn canic_fleet_registry_root_deletion_ready(",
    ] {
        assert!(
            preceding_attribute_context(&coordinator, signature).contains("canic_update(public)"),
            "{signature} must remain callable by the authenticated root"
        );
    }
    for signature in [
        "async fn canic_fleet_registry_root_deletion_execution_begin(",
        "async fn canic_fleet_registry_root_deletion_execution_status(",
        "async fn canic_fleet_registry_root_deletion_complete(",
        "async fn canic_fleet_registry_root_deletion_status(",
    ] {
        assert!(
            preceding_attribute_context(&coordinator, signature)
                .contains("requires(caller::is_controller())"),
            "{signature} must remain controller guarded"
        );
    }
}

#[test]
fn fleet_subnet_root_join_is_a_controller_update_on_the_coordinator_surface() {
    assert_eq!(
        canic::protocol::CANIC_FLEET_SUBNET_ROOT_JOIN,
        "canic_fleet_subnet_root_join"
    );
    assert_eq!(
        canic::protocol::CANIC_FLEET_SUBNET_ROOT_JOIN,
        canic_core::protocol::CANIC_FLEET_SUBNET_ROOT_JOIN
    );

    let macro_path =
        workspace_root().join("crates/canic/src/macros/endpoints/fleet_coordinator.rs");
    let source = read_text(&macro_path);
    let attribute = preceding_attribute_context(&source, "async fn canic_fleet_subnet_root_join(");

    assert!(
        attribute.contains("requires(caller::is_controller())")
            && attribute.contains("payload(max_bytes"),
        "Fleet Subnet Root join must remain a controller-guarded update"
    );
}

#[test]
fn fleet_component_provisioning_plan_surface_is_controller_guarded_and_bounded() {
    assert_eq!(
        canic::protocol::CANIC_FLEET_COMPONENT_PROVISIONING_PREPARE,
        "canic_fleet_component_provisioning_prepare"
    );
    assert_eq!(
        canic::protocol::CANIC_FLEET_COMPONENT_PROVISIONING_STATUS,
        "canic_fleet_component_provisioning_status"
    );
    assert_eq!(
        canic::protocol::CANIC_FLEET_COMPONENT_PROVISIONING_ADVANCE,
        "canic_fleet_component_provisioning_advance"
    );
    assert_fleet_component_provisioning_endpoint_guards();
    assert_fleet_component_provisioning_candid_surface();
}

fn assert_fleet_component_provisioning_endpoint_guards() {
    let source =
        read_text(&workspace_root().join("crates/canic/src/macros/endpoints/fleet_coordinator.rs"));
    let prepare = preceding_attribute_context(
        &source,
        "async fn canic_fleet_component_provisioning_prepare(",
    );
    assert!(
        prepare.contains("requires(caller::is_controller())")
            && prepare.contains("payload(max_bytes"),
        "Fleet Component plan preparation must remain controller guarded and payload bounded"
    );
    assert!(
        preceding_attribute_context(
            &source,
            "async fn canic_fleet_component_provisioning_advance(",
        )
        .contains("canic_update(requires(caller::is_controller()))"),
        "Fleet Component plan advance must remain a controller-guarded update"
    );
    let root_source =
        read_text(&workspace_root().join("crates/canic/src/macros/endpoints/root.rs"));
    let root_accept = preceding_attribute_context(
        &root_source,
        "async fn canic_root_component_provisioning_accept(",
    );
    assert!(
        root_acceptance_is_bounded_public_update(&root_accept),
        "root Component plan acceptance must remain an internal public and payload-bounded update"
    );
    let root_publish = preceding_attribute_context(
        &root_source,
        "async fn canic_root_component_provisioning_publish(",
    );
    assert!(
        root_publication_is_bounded_public_update(&root_publish),
        "root Component Directory publication must remain an internal public and payload-bounded update"
    );
    let root_synchronization = preceding_attribute_context(
        &root_source,
        "async fn canic_root_component_directories_synchronize(",
    );
    assert!(
        root_publication_is_bounded_public_update(&root_synchronization),
        "root scale-out Directory synchronization must remain an internal public and payload-bounded update"
    );
    let root_activation = preceding_attribute_context(
        &root_source,
        "async fn canic_root_component_provisioning_activate(",
    );
    assert!(
        root_activation_is_bounded_public_update(&root_activation),
        "root Component runtime activation must remain an internal public and payload-bounded update"
    );
    assert!(
        preceding_attribute_context(
            &source,
            "async fn canic_fleet_component_provisioning_status(",
        )
        .contains("canic_query(requires(caller::is_controller()))"),
        "Fleet Component plan status must remain a controller-guarded query"
    );
}

fn assert_fleet_component_provisioning_candid_surface() {
    let prepare_env = candid_type_env::<FleetComponentProvisioningPrepareRequest>();
    for field in [
        "operation_id",
        "plan",
        "directory_confirmation_roots",
        "batches",
    ] {
        assert!(
            prepare_env.contains(field),
            "Fleet Component preparation Candid is missing {field}:\n{prepare_env}"
        );
    }
    let advance_env = candid_type_env::<FleetComponentProvisioningAdvanceRequest>();
    for field in [
        "expected_accepted_root_count",
        "expected_phase",
        "expected_provisioned_root_count",
        "expected_current_root",
        "registry_committed_component_count",
        "expected_directory_confirmed_root_count",
        "expected_current_synchronization",
        "expected_current_publication",
        "expected_runtime_activated_root_count",
        "expected_current_activation",
    ] {
        assert!(
            advance_env.contains(field),
            "Fleet Component advance Candid is missing {field}:\n{advance_env}"
        );
    }
    let status_env = candid_type_env::<FleetComponentProvisioningStatusResponse>();
    for field in [
        "operation_id",
        "plan_hash",
        "phase",
        "accepted_root_count",
        "acceptance_in_flight_root",
        "provisioned_root_count",
        "current_root",
        "provisioning_in_flight_root",
        "component_count",
        "planned_at_ns",
        "roots_accepted_at_ns",
        "components_provisioned_at_ns",
        "published_fleet_registry",
        "service_topology_published_at_ns",
        "directory_confirmed_root_count",
        "current_synchronization",
        "current_publication",
        "publication_in_flight_root",
        "directories_confirmed_at_ns",
        "runtime_activated_root_count",
        "current_activation",
        "activation_in_flight_root",
        "runtimes_activated_at_ns",
    ] {
        assert!(
            status_env.contains(field),
            "Fleet Component status Candid is missing {field}:\n{status_env}"
        );
    }
    let publication_env = candid_type_env::<RootComponentPublicationRequest>();
    for field in [
        "operation_id",
        "plan_hash",
        "published_fleet_registry",
        "expected_published_component_count",
    ] {
        assert!(
            publication_env.contains(field),
            "root Component publication Candid is missing {field}:\n{publication_env}"
        );
    }
    let activation_env = candid_type_env::<RootComponentActivationRequest>();
    for field in [
        "operation_id",
        "plan_hash",
        "expected_activated_component_count",
        "expected_root_runtime_active",
    ] {
        assert!(
            activation_env.contains(field),
            "root Component activation Candid is missing {field}:\n{activation_env}"
        );
    }
}

#[test]
fn fleet_registry_snapshot_synchronization_protocol_and_guards_are_pinned() {
    assert_fleet_registry_protocol_constants();

    let coordinator_path =
        workspace_root().join("crates/canic/src/macros/endpoints/fleet_coordinator.rs");
    let coordinator = read_text(&coordinator_path);
    for signature in [
        "async fn canic_fleet_registry_snapshot_for_root(",
        "async fn canic_fleet_registry_acknowledge_root(",
    ] {
        assert!(
            preceding_attribute_context(&coordinator, signature).contains("canic_update(public)"),
            "{signature} must remain inter-canister callable"
        );
    }
    assert!(
        preceding_attribute_context(
            &coordinator,
            "async fn canic_fleet_registry_root_acknowledgements(",
        )
        .contains("canic_query(requires(caller::is_controller()))"),
        "root acknowledgement inventory must remain controller-guarded"
    );
    assert!(
        preceding_attribute_context(&coordinator, "async fn canic_fleet_registry_activate(")
            .contains("canic_update(requires(caller::is_controller()))"),
        "Registry activation must remain a controller-guarded update"
    );
    assert!(
        preceding_attribute_context(
            &coordinator,
            "async fn canic_fleet_registry_publish_root_draining(",
        )
        .contains("canic_update(requires(caller::is_controller()))"),
        "root Draining publication must remain a controller-guarded update"
    );
    assert!(
        preceding_attribute_context(
            &coordinator,
            "async fn canic_fleet_registry_publish_root_removed(",
        )
        .contains("canic_update(public)"),
        "root Removed publication must remain inter-canister callable"
    );
    assert_root_draining_reservation_protocol(&coordinator);

    let coordinator_api_path =
        workspace_root().join("crates/canic-control-plane/src/api/fleet_coordinator.rs");
    let coordinator_api = read_text(&coordinator_api_path);
    assert!(
        coordinator_api.contains("FleetCoordinatorWorkflow::snapshot_for_root(msg_caller())",)
            && coordinator_api.contains(
                "FleetCoordinatorWorkflow::acknowledge_root_snapshot(msg_caller(), request)",
            )
            && coordinator_api
                .contains("FleetCoordinatorWorkflow::publish_root_removed(msg_caller(), request)",),
        "public Coordinator transports must authenticate the exact calling root in the API facade"
    );
    assert!(
        coordinator_api.contains("FleetCoordinatorWorkflow::root_draining_reservation_status(")
            && coordinator_api.contains("is_controller(&caller)"),
        "root-draining reservation status must authenticate the controller or exact target root"
    );

    let root_path = workspace_root().join("crates/canic/src/macros/endpoints/root.rs");
    let root = read_text(&root_path);
    assert!(
        preceding_attribute_context(&root, "async fn canic_fleet_registry_synchronize(")
            .contains("canic_update(requires(caller::is_controller()))"),
        "root Registry synchronization must remain a controller-guarded update"
    );
    assert!(
        preceding_attribute_context(&root, "async fn canic_fleet_registry_sync_status(")
            .contains("canic_query(composite, requires(caller::is_controller()))"),
        "root Registry synchronization status must remain a controller-guarded composite query"
    );
    assert_root_registry_mirror_guards(&root);
}

fn assert_root_draining_reservation_protocol(coordinator: &str) {
    assert!(
        preceding_attribute_context(
            coordinator,
            "async fn canic_fleet_registry_root_draining_reservation_prepare(",
        )
        .contains("canic_update(requires(caller::is_controller()))"),
        "root-draining reservation preparation must remain controller guarded"
    );
    assert!(
        preceding_attribute_context(
            coordinator,
            "async fn canic_fleet_registry_root_draining_reservation_status(",
        )
        .contains("canic_update(public)"),
        "root-draining reservation status must remain inter-Canister callable by the exact target root"
    );
    for (name, env) in [
        (
            "request",
            candid_type_env::<FleetSubnetRootDrainingReservationRequest>(),
        ),
        (
            "status request",
            candid_type_env::<FleetSubnetRootDrainingReservationStatusRequest>(),
        ),
        (
            "response",
            candid_type_env::<FleetSubnetRootDrainingReservationResponse>(),
        ),
    ] {
        assert!(
            env.contains("operation_id") && env.contains("fleet_subnet_root"),
            "root-draining reservation {name} must bind operation and root identity:\n{env}"
        );
    }
}

fn assert_fleet_registry_protocol_constants() {
    for (facade, core, expected) in [
        (
            canic::protocol::CANIC_FLEET_REGISTRY,
            canic_core::protocol::CANIC_FLEET_REGISTRY,
            "canic_fleet_registry",
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_MANIFEST,
            canic_core::protocol::CANIC_FLEET_REGISTRY_MANIFEST,
            "canic_fleet_registry_manifest",
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_VERSION,
            canic_core::protocol::CANIC_FLEET_REGISTRY_VERSION,
            "canic_fleet_registry_version",
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_SNAPSHOT_FOR_ROOT,
            canic_core::protocol::CANIC_FLEET_REGISTRY_SNAPSHOT_FOR_ROOT,
            "canic_fleet_registry_snapshot_for_root",
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_ACKNOWLEDGE_ROOT,
            canic_core::protocol::CANIC_FLEET_REGISTRY_ACKNOWLEDGE_ROOT,
            "canic_fleet_registry_acknowledge_root",
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_ACTIVATE,
            canic_core::protocol::CANIC_FLEET_REGISTRY_ACTIVATE,
            "canic_fleet_registry_activate",
        ),
        (
            canic::protocol::CANIC_FLEET_COMPONENT_PROVISIONING_ADVANCE,
            canic_core::protocol::CANIC_FLEET_COMPONENT_PROVISIONING_ADVANCE,
            "canic_fleet_component_provisioning_advance",
        ),
        (
            canic::protocol::CANIC_FLEET_COMPONENT_PROVISIONING_PREPARE,
            canic_core::protocol::CANIC_FLEET_COMPONENT_PROVISIONING_PREPARE,
            "canic_fleet_component_provisioning_prepare",
        ),
        (
            canic::protocol::CANIC_FLEET_COMPONENT_PROVISIONING_STATUS,
            canic_core::protocol::CANIC_FLEET_COMPONENT_PROVISIONING_STATUS,
            "canic_fleet_component_provisioning_status",
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_PUBLISH_ROOT_DRAINING,
            canic_core::protocol::CANIC_FLEET_REGISTRY_PUBLISH_ROOT_DRAINING,
            "canic_fleet_registry_publish_root_draining",
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_PUBLISH_ROOT_REMOVED,
            canic_core::protocol::CANIC_FLEET_REGISTRY_PUBLISH_ROOT_REMOVED,
            "canic_fleet_registry_publish_root_removed",
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_ROOT_ACKNOWLEDGEMENTS,
            canic_core::protocol::CANIC_FLEET_REGISTRY_ROOT_ACKNOWLEDGEMENTS,
            "canic_fleet_registry_root_acknowledgements",
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_ROOT_DRAINING_RESERVATION_PREPARE,
            canic_core::protocol::CANIC_FLEET_REGISTRY_ROOT_DRAINING_RESERVATION_PREPARE,
            "canic_fleet_registry_root_draining_reservation_prepare",
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_ROOT_DRAINING_RESERVATION_STATUS,
            canic_core::protocol::CANIC_FLEET_REGISTRY_ROOT_DRAINING_RESERVATION_STATUS,
            "canic_fleet_registry_root_draining_reservation_status",
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_SYNCHRONIZE,
            canic_core::protocol::CANIC_FLEET_REGISTRY_SYNCHRONIZE,
            "canic_fleet_registry_synchronize",
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_SYNC_STATUS,
            canic_core::protocol::CANIC_FLEET_REGISTRY_SYNC_STATUS,
            "canic_fleet_registry_sync_status",
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_ACTIVATE_MIRROR,
            canic_core::protocol::CANIC_FLEET_REGISTRY_ACTIVATE_MIRROR,
            "canic_fleet_registry_activate_mirror",
        ),
        (
            canic::protocol::CANIC_FLEET_REGISTRY_MIRROR_STATUS,
            canic_core::protocol::CANIC_FLEET_REGISTRY_MIRROR_STATUS,
            "canic_fleet_registry_mirror_status",
        ),
    ] {
        assert_eq!(facade, core);
        assert_eq!(facade, expected);
    }
    assert_component_registry_protocol_constants();
}

#[expect(
    clippy::too_many_lines,
    reason = "one table pins the complete Component Registry protocol family"
)]
fn assert_component_registry_protocol_constants() {
    for (facade, core, expected) in [
        (
            canic::protocol::CANIC_ROOT_COMPONENT_REGISTRY_PREPARE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_REGISTRY_PREPARE,
            "canic_root_component_registry_prepare",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
            canic_core::protocol::CANIC_ROOT_COMPONENT_REGISTRY_STATUS,
            "canic_root_component_registry_status",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_PROVISIONING_ACCEPT,
            canic_core::protocol::CANIC_ROOT_COMPONENT_PROVISIONING_ACCEPT,
            "canic_root_component_provisioning_accept",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_PROVISIONING_ADVANCE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_PROVISIONING_ADVANCE,
            "canic_root_component_provisioning_advance",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_PROVISIONING_PUBLISH,
            canic_core::protocol::CANIC_ROOT_COMPONENT_PROVISIONING_PUBLISH,
            "canic_root_component_provisioning_publish",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_DIRECTORIES_SYNCHRONIZE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_DIRECTORIES_SYNCHRONIZE,
            "canic_root_component_directories_synchronize",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_PROVISIONING_ACTIVATE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_PROVISIONING_ACTIVATE,
            "canic_root_component_provisioning_activate",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_PROVISIONING_STATUS,
            canic_core::protocol::CANIC_ROOT_COMPONENT_PROVISIONING_STATUS,
            "canic_root_component_provisioning_status",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_ALLOCATE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_ALLOCATE,
            "canic_root_component_allocate",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_ALLOCATION_STATUS,
            canic_core::protocol::CANIC_ROOT_COMPONENT_ALLOCATION_STATUS,
            "canic_root_component_allocation_status",
        ),
        (
            canic::protocol::CANIC_ROOT_PEER_COMPONENT_ALLOCATE,
            canic_core::protocol::CANIC_ROOT_PEER_COMPONENT_ALLOCATE,
            "canic_root_peer_component_allocate",
        ),
        (
            canic::protocol::CANIC_ROOT_PEER_COMPONENT_ALLOCATION_STATUS,
            canic_core::protocol::CANIC_ROOT_PEER_COMPONENT_ALLOCATION_STATUS,
            "canic_root_peer_component_allocation_status",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_CHILD_ALLOCATE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_CHILD_ALLOCATE,
            "canic_root_component_child_allocate",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_CHILD_ALLOCATION_STATUS,
            canic_core::protocol::CANIC_ROOT_COMPONENT_CHILD_ALLOCATION_STATUS,
            "canic_root_component_child_allocation_status",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_DRAINING_BEGIN,
            canic_core::protocol::CANIC_ROOT_COMPONENT_DRAINING_BEGIN,
            "canic_root_component_draining_begin",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_DRAINING_STATUS,
            canic_core::protocol::CANIC_ROOT_COMPONENT_DRAINING_STATUS,
            "canic_root_component_draining_status",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_QUIESCE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_QUIESCE,
            "canic_root_component_quiesce",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_QUIESCENCE_STATUS,
            canic_core::protocol::CANIC_ROOT_COMPONENT_QUIESCENCE_STATUS,
            "canic_root_component_quiescence_status",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_DRAINING_ADVANCE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_DRAINING_ADVANCE,
            "canic_root_component_draining_advance",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_DRAINING_INVENTORY_FINALIZE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_DRAINING_INVENTORY_FINALIZE,
            "canic_root_component_draining_inventory_finalize",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_DELETE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_DELETE,
            "canic_root_component_delete",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_MEMBERSHIP_REMOVE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_MEMBERSHIP_REMOVE,
            "canic_root_component_membership_remove",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_DELETION_STATUS,
            canic_core::protocol::CANIC_ROOT_COMPONENT_DELETION_STATUS,
            "canic_root_component_deletion_status",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_BEGIN,
            canic_core::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_BEGIN,
            "canic_root_component_subtree_removal_begin",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_ADVANCE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_ADVANCE,
            "canic_root_component_subtree_removal_advance",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STOP_PREPARE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STOP_PREPARE,
            "canic_root_component_subtree_removal_stop_prepare",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STOP,
            canic_core::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STOP,
            "canic_root_component_subtree_removal_stop",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_DELETE_PREPARE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_DELETE_PREPARE,
            "canic_root_component_subtree_removal_delete_prepare",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_DELETE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_DELETE,
            "canic_root_component_subtree_removal_delete",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_MEMBERSHIP_REMOVE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_MEMBERSHIP_REMOVE,
            "canic_root_component_subtree_removal_membership_remove",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_DIRECTORY_SYNCHRONIZE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_DIRECTORY_SYNCHRONIZE,
            "canic_root_component_subtree_removal_directory_synchronize",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_LEAF_FINALIZE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_LEAF_FINALIZE,
            "canic_root_component_subtree_removal_leaf_finalize",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STATUS,
            canic_core::protocol::CANIC_ROOT_COMPONENT_SUBTREE_REMOVAL_STATUS,
            "canic_root_component_subtree_removal_status",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_CHILD_CREATE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_CHILD_CREATE,
            "canic_root_component_child_create",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_CHILD_INSTALL,
            canic_core::protocol::CANIC_ROOT_COMPONENT_CHILD_INSTALL,
            "canic_root_component_child_install",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_CHILD_COMMIT,
            canic_core::protocol::CANIC_ROOT_COMPONENT_CHILD_COMMIT,
            "canic_root_component_child_commit",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_CHILD_DIRECTORY_PREPARE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_CHILD_DIRECTORY_PREPARE,
            "canic_root_component_child_directory_prepare",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_CHILD_RUNTIME_ACTIVATE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_CHILD_RUNTIME_ACTIVATE,
            "canic_root_component_child_runtime_activate",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_CHILD_MEMBERSHIP_ACTIVATE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_CHILD_MEMBERSHIP_ACTIVATE,
            "canic_root_component_child_membership_activate",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_CREATE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_CREATE,
            "canic_root_component_create",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_INSTALL,
            canic_core::protocol::CANIC_ROOT_COMPONENT_INSTALL,
            "canic_root_component_install",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_COMMIT,
            canic_core::protocol::CANIC_ROOT_COMPONENT_COMMIT,
            "canic_root_component_commit",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_DIRECTORY_PREPARE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_DIRECTORY_PREPARE,
            "canic_root_component_directory_prepare",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_RUNTIME_ACTIVATE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_RUNTIME_ACTIVATE,
            "canic_root_component_runtime_activate",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_MEMBERSHIP_ACTIVATE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_MEMBERSHIP_ACTIVATE,
            "canic_root_component_membership_activate",
        ),
        (
            canic::protocol::CANIC_ROOT_PEER_COMPONENT_CREATE,
            canic_core::protocol::CANIC_ROOT_PEER_COMPONENT_CREATE,
            "canic_root_peer_component_create",
        ),
        (
            canic::protocol::CANIC_ROOT_PEER_COMPONENT_INSTALL,
            canic_core::protocol::CANIC_ROOT_PEER_COMPONENT_INSTALL,
            "canic_root_peer_component_install",
        ),
        (
            canic::protocol::CANIC_ROOT_PEER_COMPONENT_COMMIT,
            canic_core::protocol::CANIC_ROOT_PEER_COMPONENT_COMMIT,
            "canic_root_peer_component_commit",
        ),
        (
            canic::protocol::CANIC_ROOT_PEER_COMPONENT_DIRECTORY_PREPARE,
            canic_core::protocol::CANIC_ROOT_PEER_COMPONENT_DIRECTORY_PREPARE,
            "canic_root_peer_component_directory_prepare",
        ),
        (
            canic::protocol::CANIC_ROOT_PEER_COMPONENT_RUNTIME_ACTIVATE,
            canic_core::protocol::CANIC_ROOT_PEER_COMPONENT_RUNTIME_ACTIVATE,
            "canic_root_peer_component_runtime_activate",
        ),
        (
            canic::protocol::CANIC_ROOT_PEER_COMPONENT_MEMBERSHIP_ACTIVATE,
            canic_core::protocol::CANIC_ROOT_PEER_COMPONENT_MEMBERSHIP_ACTIVATE,
            "canic_root_peer_component_membership_activate",
        ),
        (
            canic::protocol::CANIC_COMPONENT_RUNTIME_DIRECTORY_PREPARE,
            canic_core::protocol::CANIC_COMPONENT_RUNTIME_DIRECTORY_PREPARE,
            "canic_component_runtime_directory_prepare",
        ),
        (
            canic::protocol::CANIC_COMPONENT_RUNTIME_DIRECTORY_SYNCHRONIZE,
            canic_core::protocol::CANIC_COMPONENT_RUNTIME_DIRECTORY_SYNCHRONIZE,
            "canic_component_runtime_directory_synchronize",
        ),
        (
            canic::protocol::CANIC_COMPONENT_RUNTIME_STATUS,
            canic_core::protocol::CANIC_COMPONENT_RUNTIME_STATUS,
            "canic_component_runtime_status",
        ),
        (
            canic::protocol::CANIC_COMPONENT_RUNTIME_ACTIVATE,
            canic_core::protocol::CANIC_COMPONENT_RUNTIME_ACTIVATE,
            "canic_component_runtime_activate",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_REGISTRY_PARTITION,
            canic_core::protocol::CANIC_ROOT_COMPONENT_REGISTRY_PARTITION,
            "canic_root_component_registry_partition",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_DIRECTORY_HEAD,
            canic_core::protocol::CANIC_ROOT_COMPONENT_DIRECTORY_HEAD,
            "canic_root_component_directory_head",
        ),
        (
            canic::protocol::CANIC_ROOT_COMPONENT_DIRECTORY_PAGE,
            canic_core::protocol::CANIC_ROOT_COMPONENT_DIRECTORY_PAGE,
            "canic_root_component_directory_page",
        ),
    ] {
        assert_eq!(facade, core);
        assert_eq!(facade, expected);
    }
}

fn assert_root_registry_mirror_guards(root: &str) {
    assert!(
        preceding_attribute_context(root, "async fn canic_fleet_registry_activate_mirror(")
            .contains("canic_update(requires(caller::is_controller()))"),
        "root Registry mirror activation must remain a controller-guarded update"
    );
    assert!(
        preceding_attribute_context(root, "async fn canic_fleet_registry_mirror_status(")
            .contains("canic_query(composite, requires(caller::is_controller()))"),
        "root Registry mirror status must remain a controller-guarded composite query"
    );
    assert!(
        preceding_attribute_context(root, "async fn canic_root_component_registry_prepare(")
            .contains("canic_update(requires(caller::is_controller()))"),
        "root Component Registry preparation must remain a controller-guarded update"
    );
    assert!(
        preceding_attribute_context(root, "async fn canic_root_component_registry_status(")
            .contains("canic_query(composite, requires(caller::is_controller()))"),
        "root Component Registry status must remain a controller-guarded composite query"
    );
    assert_root_component_provisioning_guards(root);
    assert!(
        preceding_attribute_context(root, "async fn canic_root_component_allocate(")
            .contains("canic_update(requires(caller::is_controller()))"),
        "root Component allocation must remain a controller-guarded update"
    );
    assert_peer_component_guards(root);
    assert!(
        preceding_attribute_context(root, "async fn canic_root_component_allocation_status(")
            .contains("canic_query(requires(caller::is_controller()))"),
        "root Component allocation status must remain a controller-guarded query"
    );
    for endpoint in [
        "async fn canic_root_component_child_allocate(",
        "async fn canic_root_component_child_create(",
        "async fn canic_root_component_child_install(",
        "async fn canic_root_component_child_commit(",
        "async fn canic_root_component_child_directory_prepare(",
        "async fn canic_root_component_child_runtime_activate(",
        "async fn canic_root_component_child_membership_activate(",
    ] {
        assert!(
            preceding_attribute_context(root, endpoint).contains("canic_update(internal, public)"),
            "{endpoint} must remain a public update authenticated by workflow"
        );
    }
    assert!(
        preceding_attribute_context(
            root,
            "async fn canic_root_component_child_allocation_status("
        )
        .contains("canic_query(internal, public)"),
        "root Component Child allocation status must remain a public query authenticated by workflow"
    );
    assert_component_draining_guards(root);
    assert_subtree_removal_guards(root);
    for endpoint in [
        "async fn canic_root_component_create(",
        "async fn canic_root_component_install(",
        "async fn canic_root_component_commit(",
        "async fn canic_root_component_directory_prepare(",
        "async fn canic_root_component_runtime_activate(",
        "async fn canic_root_component_membership_activate(",
    ] {
        assert!(
            preceding_attribute_context(root, endpoint)
                .contains("canic_update(requires(caller::is_controller()))"),
            "{endpoint} must remain a controller-guarded update"
        );
    }
    for endpoint in [
        "async fn canic_root_component_registry_partition(",
        "async fn canic_root_component_directory_head(",
    ] {
        assert!(
            preceding_attribute_context(root, endpoint)
                .contains("canic_query(requires(caller::is_controller()))"),
            "{endpoint} must remain a controller-guarded query"
        );
    }
    assert!(
        preceding_attribute_context(root, "async fn canic_root_component_directory_page(")
            .contains("canic_query(internal, public)"),
        "root Component Directory pages must remain public queries authenticated by workflow"
    );
}

fn assert_root_component_provisioning_guards(root: &str) {
    let acceptance =
        preceding_attribute_context(root, "async fn canic_root_component_provisioning_accept(");
    assert!(
        root_acceptance_is_bounded_public_update(&acceptance),
        "root Component provisioning acceptance must remain a public update authenticated by workflow"
    );
    assert!(
        preceding_attribute_context(root, "async fn canic_root_component_provisioning_advance(")
            .contains("canic_update(internal, public)"),
        "root Component provisioning advance must remain a public update authenticated by workflow"
    );
    let activation =
        preceding_attribute_context(root, "async fn canic_root_component_provisioning_activate(");
    assert!(
        root_activation_is_bounded_public_update(&activation),
        "root Component provisioning activation must remain a bounded public update authenticated by workflow"
    );
    assert!(
        preceding_attribute_context(root, "async fn canic_root_component_provisioning_status(")
            .contains("canic_query(internal, public)"),
        "root Component provisioning status must remain a public query authenticated by workflow"
    );
}

fn root_acceptance_is_bounded_public_update(attribute: &str) -> bool {
    is_bounded_public_update(
        attribute,
        "MAX_FLEET_SUBNET_ROOT_PROVISIONING_ACCEPTANCE_PAYLOAD_BYTES",
    )
}

fn root_publication_is_bounded_public_update(attribute: &str) -> bool {
    is_bounded_public_update(
        attribute,
        "MAX_FLEET_SUBNET_ROOT_COMPONENT_PUBLICATION_PAYLOAD_BYTES",
    )
}

fn root_activation_is_bounded_public_update(attribute: &str) -> bool {
    is_bounded_public_update(
        attribute,
        "MAX_FLEET_SUBNET_ROOT_COMPONENT_ACTIVATION_PAYLOAD_BYTES",
    )
}

fn is_bounded_public_update(attribute: &str, exact_envelope: &str) -> bool {
    let is_internal_public = attribute.contains("internal") && attribute.contains("public");
    let has_exact_envelope =
        attribute.contains("payload(max_bytes") && attribute.contains(exact_envelope);
    attribute.contains("canic_update(") && is_internal_public && has_exact_envelope
}

fn assert_peer_component_guards(root: &str) {
    assert!(
        preceding_attribute_context(root, "async fn canic_root_peer_component_allocate(")
            .contains("canic_update(internal, public)"),
        "peer Component allocation must remain a public update authenticated by workflow"
    );
    assert!(
        preceding_attribute_context(
            root,
            "async fn canic_root_peer_component_allocation_status("
        )
        .contains("canic_query(internal, public)"),
        "peer Component allocation status must remain a public query authenticated by workflow"
    );
    for endpoint in [
        "async fn canic_root_peer_component_create(",
        "async fn canic_root_peer_component_install(",
        "async fn canic_root_peer_component_commit(",
        "async fn canic_root_peer_component_directory_prepare(",
        "async fn canic_root_peer_component_runtime_activate(",
        "async fn canic_root_peer_component_membership_activate(",
    ] {
        assert!(
            preceding_attribute_context(root, endpoint).contains("canic_update(internal, public)"),
            "{endpoint} must remain a public update authenticated by workflow"
        );
    }
}

fn assert_component_draining_guards(root: &str) {
    for endpoint in [
        "async fn canic_root_component_draining_begin(",
        "async fn canic_root_component_quiesce(",
        "async fn canic_root_component_draining_advance(",
        "async fn canic_root_component_draining_inventory_finalize(",
        "async fn canic_root_component_delete(",
        "async fn canic_root_component_membership_remove(",
    ] {
        assert!(
            preceding_attribute_context(root, endpoint)
                .contains("canic_update(requires(caller::is_controller()))"),
            "{endpoint} must remain a controller-guarded update"
        );
    }
    for endpoint in [
        "async fn canic_root_component_draining_status(",
        "async fn canic_root_component_quiescence_status(",
        "async fn canic_root_component_deletion_status(",
    ] {
        assert!(
            preceding_attribute_context(root, endpoint)
                .contains("canic_query(requires(caller::is_controller()))"),
            "{endpoint} must remain a controller-guarded query"
        );
    }
}

fn assert_subtree_removal_guards(root: &str) {
    assert!(
        preceding_attribute_context(root, "async fn canic_root_component_subtree_removal_begin(")
            .contains("canic_update(requires(caller::is_controller()))"),
        "root Component subtree-removal fencing must remain a controller-guarded update"
    );
    assert!(
        preceding_attribute_context(
            root,
            "async fn canic_root_component_subtree_removal_advance("
        )
        .contains("canic_update(requires(caller::is_controller()))"),
        "root Component subtree-removal traversal must remain a controller-guarded update"
    );
    assert!(
        preceding_attribute_context(
            root,
            "async fn canic_root_component_subtree_removal_stop_prepare("
        )
        .contains("canic_update(requires(caller::is_controller()))"),
        "root Component subtree stop preparation must remain a controller-guarded update"
    );
    assert!(
        preceding_attribute_context(root, "async fn canic_root_component_subtree_removal_stop(")
            .contains("canic_update(requires(caller::is_controller()))"),
        "root Component subtree stop must remain a controller-guarded update"
    );
    assert!(
        preceding_attribute_context(
            root,
            "async fn canic_root_component_subtree_removal_delete_prepare("
        )
        .contains("canic_update(requires(caller::is_controller()))"),
        "root Component subtree deletion preparation must remain a controller-guarded update"
    );
    assert!(
        preceding_attribute_context(
            root,
            "async fn canic_root_component_subtree_removal_delete("
        )
        .contains("canic_update(requires(caller::is_controller()))"),
        "root Component subtree deletion must remain a controller-guarded update"
    );
    assert!(
        preceding_attribute_context(
            root,
            "async fn canic_root_component_subtree_removal_membership_remove("
        )
        .contains("canic_update(requires(caller::is_controller()))"),
        "root Component subtree membership removal must remain a controller-guarded update"
    );
    assert!(
        preceding_attribute_context(
            root,
            "async fn canic_root_component_subtree_removal_directory_synchronize("
        )
        .contains("canic_update(requires(caller::is_controller()))"),
        "root Component subtree Directory synchronization must remain a controller-guarded update"
    );
    assert!(
        preceding_attribute_context(
            root,
            "async fn canic_root_component_subtree_removal_leaf_finalize("
        )
        .contains("canic_update(requires(caller::is_controller()))"),
        "root Component subtree leaf finalization must remain a controller-guarded update"
    );
    assert!(
        preceding_attribute_context(
            root,
            "async fn canic_root_component_subtree_removal_status("
        )
        .contains("canic_query(requires(caller::is_controller()))"),
        "root Component subtree-removal status must remain a controller-guarded query"
    );
}

#[test]
fn root_store_bootstrap_protocol_and_guards_are_pinned() {
    assert_eq!(
        canic::protocol::CANIC_ROOT_STORE_BOOTSTRAP,
        "canic_root_store_bootstrap"
    );
    assert_eq!(
        canic::protocol::CANIC_ROOT_STORE_BOOTSTRAP_STATUS,
        "canic_root_store_bootstrap_status"
    );
    assert_eq!(
        canic::protocol::CANIC_ROOT_STORE_BOOTSTRAP,
        canic_core::protocol::CANIC_ROOT_STORE_BOOTSTRAP
    );
    assert_eq!(
        canic::protocol::CANIC_ROOT_STORE_BOOTSTRAP_STATUS,
        canic_core::protocol::CANIC_ROOT_STORE_BOOTSTRAP_STATUS
    );

    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/root.rs");
    let source = read_text(&macro_path);
    let bootstrap_attribute =
        preceding_attribute_context(&source, "async fn canic_root_store_bootstrap(");
    let status_attribute =
        preceding_attribute_context(&source, "async fn canic_root_store_bootstrap_status(");

    assert!(
        bootstrap_attribute.contains("canic_update(requires(caller::is_controller()))"),
        "root Store bootstrap must remain a controller-guarded update"
    );
    assert!(
        status_attribute.contains("canic_query(composite, requires(caller::is_controller()))"),
        "root Store bootstrap status must remain a controller-guarded composite query"
    );
}

#[test]
fn nonroot_runtime_activation_mutations_are_guarded_by_the_exact_root() {
    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/nonroot.rs");
    let source = read_text(&macro_path);

    for signature in [
        "async fn canic_component_runtime_directory_prepare(",
        "async fn canic_component_runtime_directory_synchronize(",
        "async fn canic_component_runtime_activate(",
        "async fn canic_prepare_fleet_credential_generation(",
        "async fn canic_activate_fleet(",
    ] {
        let attribute = preceding_attribute_context(&source, signature);
        assert!(
            attribute.contains("canic_update(internal, requires(caller::is_root()))"),
            "{signature} must remain guarded by the protected Fleet Subnet Root"
        );
    }
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
    assert_eq!(
        canic::protocol::CANIC_WASM_STORE_RECLAIM_DELETION_CYCLES,
        "canic_wasm_store_reclaim_deletion_cycles"
    );

    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/wasm_store.rs");
    let source = read_text(&macro_path);
    let attribute = preceding_attribute_context(
        &source,
        "async fn canic_wasm_store_reclaim_deletion_cycles(",
    );
    assert!(
        attribute.contains("canic_update(internal, requires(caller::is_root()))"),
        "Store deletion cycle reclamation must remain guarded by the protected root"
    );
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
            "blob-storage fixture missing gateway method: {method}"
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
        "billing endpoints must stay update endpoints with separate guards"
    );
    assert!(
        source.contains("requested_cycles: u128")
            && source.contains(
                ") -> Result<::canic::dto::blob_storage::BlobProjectCyclesTopUpReport, ::canic::Error>"
            )
            && source.contains("BlobStorageApi::fund_from_project_cycles("),
        "funding endpoint must keep returning the structured top-up report"
    );
    assert!(
        !source.contains("BlobStorageBillingConfig")
            && !source.contains("configure_billing")
            && !source.contains("billing_config"),
        "generated billing endpoints must not expose billing configuration as a public admin surface"
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
fn blob_storage_funding_report_dto_roundtrips_through_candid() {
    assert_candid_roundtrip(BlobProjectCyclesTopUpReport {
        requested_cycles: candid::Nat::from(77_u64),
        attached_cycles: candid::Nat::from(77_u64),
        project_cycles_before: candid::Nat::from(1_000_u64),
        project_cycles_after: candid::Nat::from(923_u64),
        reserve_cycles: candid::Nat::from(1_u64),
        cashier_total_after: candid::Nat::from(200_u64),
        skipped_reason: None,
    });
    assert_candid_roundtrip(BlobProjectCyclesTopUpReport {
        requested_cycles: candid::Nat::from(1_001_u64),
        attached_cycles: candid::Nat::from(0_u64),
        project_cycles_before: candid::Nat::from(1_000_u64),
        project_cycles_after: candid::Nat::from(1_000_u64),
        reserve_cycles: candid::Nat::from(999_u64),
        cashier_total_after: candid::Nat::from(0_u64),
        skipped_reason: Some("reserve would be violated".to_string()),
    });

    let report_env = candid_type_env::<BlobProjectCyclesTopUpReport>();
    assert!(
        report_env.contains("type BlobProjectCyclesTopUpReport = record")
            && report_env.contains("requested_cycles : nat")
            && report_env.contains("attached_cycles : nat")
            && report_env.contains("project_cycles_before : nat")
            && report_env.contains("project_cycles_after : nat")
            && report_env.contains("reserve_cycles : nat")
            && report_env.contains("cashier_total_after : nat")
            && report_env.contains("skipped_reason : opt text"),
        "blob-storage funding report DTO Candid changed:\n{report_env}"
    );
}

#[cfg(feature = "blob-storage-billing")]
#[test]
fn blob_storage_billing_config_dto_roundtrips_through_candid() {
    assert_candid_roundtrip(BlobStorageBillingConfig {
        cashier_canister_id: Principal::from_slice(&[1, 2, 3]),
        project_cycles_reserve: candid::Nat::from(1_u64),
        min_upload_balance: candid::Nat::from(10_u64),
        target_upload_balance: candid::Nat::from(100_u64),
        gateway_principal_limit: 8,
    });
}

#[cfg(feature = "blob-storage-billing")]
#[test]
fn blob_storage_billing_config_dto_candid_shape_is_pinned() {
    let config_env = candid_type_env::<BlobStorageBillingConfig>();
    assert!(
        config_env.contains("type BlobStorageBillingConfig = record")
            && config_env.contains("cashier_canister_id : principal")
            && config_env.contains("project_cycles_reserve : nat")
            && config_env.contains("min_upload_balance : nat")
            && config_env.contains("target_upload_balance : nat")
            && config_env.contains("gateway_principal_limit : nat64"),
        "blob-storage billing config DTO Candid changed:\n{config_env}"
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
        source.contains(
            "canic_query(internal, public, name = \"_immutableObjectStorageBlobsAreLive\")"
        ) && source.contains(
            "canic_query(internal, public, name = \"_immutableObjectStorageBlobsToDelete\")"
        ) && source.contains(
            "canic_update(internal, public, name = \"_immutableObjectStorageConfirmBlobDeletion\")"
        ) && source.contains(
            "canic_update(requires($guard), name = \"_immutableObjectStorageCreateCertificate\")"
        ),
        "blob-storage endpoint modes/guards must match the gateway contract"
    );

    let live_attr = preceding_attribute(&source, "fn canic_blob_storage_blobs_are_live(");
    let to_delete_attr = preceding_attribute(&source, "fn canic_blob_storage_blobs_to_delete(");
    let confirm_attr = preceding_attribute(&source, "fn canic_blob_storage_confirm_blob_deletion(");
    let create_attr = preceding_attribute(&source, "fn canic_blob_storage_create_certificate(");
    assert!(
        live_attr.contains("canic_query(internal, public")
            && !live_attr.contains("requires")
            && to_delete_attr.contains("canic_query(internal, public")
            && !to_delete_attr.contains("requires")
            && confirm_attr.contains("canic_update(internal, public")
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
        "endpoint macro must not emit deferred billing/sync gateway methods"
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
    assert_root_provisioning_facade_is_public();
    assert_root_delegation_protocol_constants();
    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/root.rs");
    let source = read_text(&macro_path);
    assert_root_delegation_macro_guards(&source);
    assert_root_delegation_endpoint_bindings(&source);
}

fn assert_root_provisioning_facade_is_public() {
    fn assert_signature<F, Fut>(function: F)
    where
        F: FnOnce(Principal) -> Fut,
        Fut: std::future::Future<Output = Result<(), canic::Error>>,
    {
        std::hint::black_box(function);
    }

    assert_signature(
        canic::api::auth::AuthApi::provision_chain_key_delegation_proof_for_issuer_root,
    );
}

fn assert_root_delegation_protocol_constants() {
    for (public, core, expected) in [
        (
            canic::protocol::CANIC_UPSERT_ROOT_ISSUER_POLICY,
            canic_core::protocol::CANIC_UPSERT_ROOT_ISSUER_POLICY,
            "canic_upsert_root_issuer_policy",
        ),
        (
            canic::protocol::CANIC_UPSERT_ROOT_ISSUER_RENEWAL_TEMPLATE,
            canic_core::protocol::CANIC_UPSERT_ROOT_ISSUER_RENEWAL_TEMPLATE,
            "canic_upsert_root_issuer_renewal_template",
        ),
        (
            canic::protocol::CANIC_ROOT_ISSUER_RENEWAL_STATUS,
            canic_core::protocol::CANIC_ROOT_ISSUER_RENEWAL_STATUS,
            "canic_root_issuer_renewal_status",
        ),
        (
            canic::protocol::CANIC_GET_OR_CREATE_CHAIN_KEY_DELEGATION_PROOF,
            canic_core::protocol::CANIC_GET_OR_CREATE_CHAIN_KEY_DELEGATION_PROOF,
            "canic_get_or_create_chain_key_delegation_proof",
        ),
    ] {
        assert_eq!(public, core);
        assert_eq!(public, expected);
    }
}

fn assert_root_delegation_macro_guards(source: &str) {
    let upsert_attr = preceding_attribute(source, "fn canic_upsert_root_issuer_policy(");
    let renewal_upsert_attr =
        preceding_attribute(source, "fn canic_upsert_root_issuer_renewal_template(");
    let renewal_status_attr = preceding_attribute(source, "fn canic_root_issuer_renewal_status(");
    let lazy_repair_attr =
        preceding_attribute_context(source, "fn canic_get_or_create_chain_key_delegation_proof(");
    assert!(
        upsert_attr.contains("canic_update")
            && upsert_attr.contains("caller::is_controller()")
            && !upsert_attr.contains("internal"),
        "root issuer policy upsert must remain a public controller-gated update"
    );
    assert!(
        renewal_upsert_attr.contains("canic_update")
            && renewal_upsert_attr.contains("caller::is_controller()")
            && !renewal_upsert_attr.contains("internal"),
        "root issuer renewal template upsert must remain a public controller-gated update"
    );
    assert!(
        renewal_status_attr.contains("canic_query")
            && renewal_status_attr.contains("caller::is_controller()")
            && !renewal_status_attr.contains("internal"),
        "root issuer renewal status must remain a public controller-gated query"
    );
    assert!(
        lazy_repair_attr.contains("canic_update")
            && lazy_repair_attr.contains("internal")
            && lazy_repair_attr.contains("ActiveComponentMemberPredicate")
            && lazy_repair_attr.contains("custom(")
            && !lazy_repair_attr.contains("caller::is_controller()"),
        "root chain-key lazy repair must remain an internal active-Component update"
    );
}

fn assert_root_delegation_endpoint_bindings(source: &str) {
    assert!(
        source.contains("fn canic_upsert_root_issuer_policy(")
            && source.contains("RootIssuerPolicyUpsertRequest")
            && source.contains("RootIssuerPolicyResponse")
            && source.contains("AuthApi::upsert_root_issuer_policy_root"),
        "root auth endpoint bundle must expose issuer policy upsert"
    );
    assert!(
        source.contains("fn canic_upsert_root_issuer_renewal_template(")
            && source.contains("RootIssuerRenewalTemplateUpsertRequest")
            && source.contains("RootIssuerRenewalTemplateResponse")
            && source.contains("AuthApi::upsert_root_issuer_renewal_template_root"),
        "root auth endpoint bundle must expose issuer renewal template upsert"
    );
    assert!(
        source.contains("fn canic_root_issuer_renewal_status(")
            && source.contains("RootIssuerRenewalStatusRequest")
            && source.contains("RootIssuerRenewalStatusResponse")
            && source.contains("AuthApi::root_issuer_renewal_status_root"),
        "root auth endpoint bundle must expose issuer renewal status"
    );
    assert!(
        source.contains("fn canic_get_or_create_chain_key_delegation_proof(")
            && source.contains("RootDelegationProofBatchProof")
            && source.contains("AuthApi::get_or_create_chain_key_delegation_proof_root"),
        "root auth endpoint bundle must expose chain-key lazy repair"
    );
}

#[test]
fn root_delegation_proof_dtos_roundtrip_through_candid() {
    assert_root_issuer_policy_dtos_roundtrip();
    assert_root_issuer_renewal_dtos_roundtrip();
    assert_root_delegation_proof_dtos_roundtrip();
    assert_active_delegation_proof_status_roundtrip();
}

fn assert_root_issuer_policy_dtos_roundtrip() {
    let issuer_pid = Principal::from_slice(&[17; 29]);
    let grant = test_delegated_role_grant();
    let audience = DelegationAudience::Fleet(test_fleet());
    let issuer_policy_request =
        root_issuer_policy_upsert_request(issuer_pid, audience.clone(), grant.clone());
    let issuer_policy_response = root_issuer_policy_response(issuer_pid, audience, grant);

    assert_candid_roundtrip(issuer_policy_request);
    assert_candid_roundtrip(issuer_policy_response);
}

fn assert_root_issuer_renewal_dtos_roundtrip() {
    let issuer_pid = Principal::from_slice(&[17; 29]);
    let batch_id = [19; 32];
    let cert_hash = [20; 32];
    let renewal_batch = RootIssuerRenewalBatchView {
        batch_id,
        status: RootIssuerRenewalBatchStatus::Prepared,
        cert_hash,
        proof_epoch: 4,
        prepared_at_ns: 60,
        expires_at_ns: 90,
        installed_at_ns: None,
        retry_after_ns: Some(80),
        failure: Some("CallFailed".to_string()),
    };
    let renewal_template = RootIssuerRenewalTemplateView {
        issuer_pid,
        enabled: true,
        aud: DelegationAudience::Fleet(test_fleet()),
        grants: vec![test_delegated_role_grant()],
        cert_ttl_ns: 60,
    };
    let renewal_template_request = RootIssuerRenewalTemplateUpsertRequest {
        issuer_pid,
        enabled: renewal_template.enabled,
        aud: renewal_template.aud.clone(),
        grants: renewal_template.grants.clone(),
        cert_ttl_ns: renewal_template.cert_ttl_ns,
    };
    let renewal_template_response = RootIssuerRenewalTemplateResponse {
        template: renewal_template.clone(),
    };
    let renewal_status_request = RootIssuerRenewalStatusRequest { issuer_pid };
    let renewal_status_response = RootIssuerRenewalStatusResponse {
        template: Some(renewal_template),
        state: Some(RootIssuerRenewalStateView {
            issuer_pid,
            template_fingerprint: [21; 32],
            last_installed_cert_hash: Some(cert_hash),
            last_installed_expires_at_ns: Some(90),
            last_installed_refresh_after_ns: Some(72),
            next_attempt_after_ns: 80,
            updated_at_ns: 70,
        }),
        latest_batch: Some(renewal_batch),
    };

    assert_candid_roundtrip(renewal_template_request);
    assert_candid_roundtrip(renewal_template_response);
    assert_candid_roundtrip(renewal_status_request);
    assert_candid_roundtrip(renewal_status_response);
}

fn assert_root_delegation_proof_dtos_roundtrip() {
    let issuer_pid = Principal::from_slice(&[17; 29]);
    let root_pid = Principal::from_slice(&[18; 29]);
    let cert_hash = [20; 32];
    let grant = test_delegated_role_grant();
    let audience = DelegationAudience::Fleet(test_fleet());
    let proof = root_delegation_proof(root_pid, issuer_pid, audience, grant);
    let chain_key_proof =
        RootProof::IcChainKeyBatchSignatureV1(chain_key_root_proof(root_pid, issuer_pid));
    let batch_proof = RootDelegationProofBatchProof {
        issuer_pid,
        cert_hash,
        proof,
    };
    assert_candid_roundtrip(chain_key_proof);
    assert_candid_roundtrip(batch_proof);
}

fn assert_active_delegation_proof_status_roundtrip() {
    let issuer_pid = Principal::from_slice(&[17; 29]);
    let root_pid = Principal::from_slice(&[18; 29]);
    let cert_hash = [20; 32];
    let status = ActiveDelegationProofStatusResponse {
        status: ActiveDelegationProofStatus::RefreshNeeded,
        root_pid: Some(root_pid),
        issuer_pid: Some(issuer_pid),
        cert_hash: Some(cert_hash),
        expires_at_ns: Some(90),
        refresh_after_ns: Some(72),
    };

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
        root_proof: RootProof::IcChainKeyBatchSignatureV1(chain_key_root_proof(
            root_pid, issuer_pid,
        )),
    }
}

fn chain_key_root_proof(
    root_canister_id: Principal,
    issuer_canister_id: Principal,
) -> IcChainKeyBatchSignatureProofV1 {
    let key_id = ChainKeyKeyId {
        name: "test_key_1".to_string(),
    };
    let grant = test_delegated_role_grant();

    IcChainKeyBatchSignatureProofV1 {
        header: ChainKeyBatchHeaderV1 {
            schema_version: 1,
            root_canister_id,
            batch_id: [31; 32],
            proof_epoch: 2,
            registry_epoch: 3,
            registry_hash: [32; 32],
            tree_root: [33; 32],
            not_before_ns: 10,
            expires_at_ns: 110,
            algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
            key_id: key_id.clone(),
            derivation_path_hash: [34; 32],
            key_version: 4,
        },
        delegation_cert: ChainKeyDelegationCertV1 {
            root_canister_id,
            issuer_canister_id,
            proof_epoch: 2,
            issuer_proof_algorithm: IssuerProofAlgorithm::IcCanisterSignatureV1,
            issuer_proof_binding_hash: [35; 32],
            issuer_proof_binding: IssuerProofBinding::IcCanisterSignatureV1 {
                seed_hash: [36; 32],
            },
            max_token_ttl_ns: 60,
            audience: DelegationAudience::Fleet(test_fleet()),
            grants: vec![grant],
            not_before_ns: 10,
            expires_at_ns: 110,
            registry_epoch: 3,
            registry_hash: [32; 32],
        },
        issuer_witness: ChainKeyBatchWitnessV1 {
            steps: vec![
                ChainKeyBatchWitnessStepV1::LeftSibling([37; 32]),
                ChainKeyBatchWitnessStepV1::RightSibling([38; 32]),
            ],
        },
        signature: ChainKeyRootSignatureV1 {
            algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
            key_id,
            derivation_path: vec![b"canic".to_vec(), b"root-delegation".to_vec()],
            public_key: vec![39; 33],
            signature: vec![40; 64],
        },
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
    let prepare_attribute = preceding_attribute(&source, "fn canic_prepare_role_attestation(");
    let get_attribute = preceding_attribute(&source, "fn canic_get_role_attestation(");
    assert!(
        prepare_attribute.contains("canic_update(internal, public)")
            && get_attribute.contains("canic_query(internal, public)"),
        "role-attestation endpoints must defer to active Component Registry admission"
    );
    assert!(
        source.contains("fn canic_prepare_role_attestation(")
            && source.contains("RoleAttestationPrepareResponse")
            && source.contains("ComponentAuthApi::prepare_role_attestation"),
        "root auth endpoint bundle must expose role-attestation prepare"
    );
    assert!(
        source.contains("fn canic_get_role_attestation(")
            && source.contains("RoleAttestationGetRequest")
            && source.contains("ComponentAuthApi::get_role_attestation"),
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
        preceding_attribute.contains("$crate::__internal::cdk::query"),
        "memory ledger diagnostic must use a raw query attribute in {}",
        macro_path.display()
    );
    assert!(
        !preceding_attribute.contains("canic_query"),
        "memory ledger diagnostic must not use normal Canic query dispatch in {}",
        macro_path.display()
    );
    assert!(
        endpoint.contains("$crate::__internal::cdk::api::is_controller")
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
fn runtime_introspection_endpoints_are_controller_guarded_by_default() {
    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/shared.rs");
    let source = read_text(&macro_path);
    let endpoint_macro = source
        .split("macro_rules! canic_emit_runtime_introspection_endpoints")
        .nth(1)
        .and_then(|rest| {
            rest.split("macro_rules! canic_emit_icrc_standards_endpoints")
                .next()
        })
        .expect("runtime introspection endpoint macro should exist");

    for endpoint in [
        "fn canic_health()",
        "fn canic_readiness(",
        "fn canic_runtime_status(",
    ] {
        assert!(
            endpoint_macro.contains(endpoint),
            "runtime introspection macro should emit {endpoint}"
        );
    }

    assert!(
        endpoint_macro
            .matches("requires(caller::is_controller())")
            .count()
            >= 3,
        "runtime introspection endpoints must be controller-guarded by default"
    );
    assert!(
        !endpoint_macro.contains("public)]"),
        "runtime introspection endpoints must not be public by default"
    );
}

#[test]
fn authority_snapshot_restore_endpoints_are_controller_guarded_and_authority_scoped() {
    let shared_path = workspace_root().join("crates/canic/src/macros/endpoints/shared.rs");
    let shared = read_text(&shared_path);
    let endpoint_macro = shared
        .split("macro_rules! canic_emit_authority_restore_endpoints")
        .nth(1)
        .and_then(|rest| {
            rest.split("macro_rules! canic_emit_lifecycle_core_endpoints")
                .next()
        })
        .expect("authority restore endpoint macro");
    for endpoint in [
        "canic_authority_restore_fence_status",
        "canic_authority_snapshot_prepare",
        "canic_authority_snapshot_resume",
    ] {
        assert!(
            endpoint_macro.contains(endpoint),
            "authority restore macro must emit {endpoint}"
        );
    }
    assert_eq!(
        endpoint_macro
            .matches("requires(caller::is_controller())")
            .count(),
        3,
        "every authority restore endpoint must remain controller-guarded"
    );
    assert!(
        !endpoint_macro.contains("public)]"),
        "authority restore endpoints must not become public"
    );

    let bundles = read_text(&workspace_root().join("crates/canic/src/macros/endpoints/bundles.rs"));
    let root_bundle = bundles
        .split("macro_rules! canic_bundle_root_only_endpoints")
        .nth(1)
        .and_then(|rest| {
            rest.split("macro_rules! canic_bundle_managed_nonroot_only_endpoints")
                .next()
        })
        .expect("root-only endpoint bundle");
    assert!(root_bundle.contains("canic_emit_authority_restore_endpoints!"));

    let coordinator =
        read_text(&workspace_root().join("crates/canic/src/macros/endpoints/fleet_coordinator.rs"));
    assert!(coordinator.contains("canic_emit_authority_restore_endpoints!"));
}

#[test]
fn root_icp_refill_endpoint_is_controller_guarded() {
    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/root.rs");
    let source = read_text(&macro_path);
    let attribute = preceding_attribute_context(&source, "async fn canic_icp_refill(");

    assert!(
        attribute.contains("canic_update(requires(caller::is_controller()))"),
        "root ICP refill endpoint must remain controller-guarded"
    );
}

#[test]
fn root_icp_refill_dto_candid_shapes_are_named() {
    let request_env = candid_type_env::<IcpRefillRequest>();
    assert!(
        request_env.contains("type IcpRefillRequest = record")
            && request_env.contains("operation_id : blob")
            && request_env.contains("source_subaccount : opt blob")
            && request_env.contains("amount_e8s : nat64")
            && request_env.contains("dry_run : bool"),
        "root ICP refill request Candid changed:\n{request_env}"
    );

    let dry_run_env = candid_type_env::<IcpRefillDryRun>();
    assert!(
        dry_run_env.contains("type IcpRefillDryRun = record")
            && dry_run_env.contains("operation_id : blob")
            && dry_run_env.contains("amount_e8s : nat64")
            && dry_run_env.contains("fee_e8s : nat64")
            && dry_run_env.contains("xdr_permyriad_per_icp : opt nat64")
            && dry_run_env.contains("estimated_cycles : opt nat"),
        "root ICP refill dry-run Candid changed:\n{dry_run_env}"
    );
}

#[test]
fn runtime_introspection_dto_candid_shapes_are_named() {
    let status_env = candid_type_env::<CanicRuntimeStatus>();

    assert!(
        status_env.contains("type CanicRuntimeStatus = record")
            && status_env.contains("schema_version : nat32")
            && status_env.contains("observed_at_ns : nat64")
            && status_env.contains("canister_id : principal")
            && status_env.contains("build_network : opt BuildNetwork")
            && status_env.contains("type BuildNetwork = variant { ic; local }")
            && status_env.contains("readiness : CanicReadinessStatus")
            && status_env.contains("auth : opt RuntimeAuthStatusSummary")
            && status_env.contains("blob_storage : opt RuntimeBlobStorageStatusSummary")
            && status_env.contains("receipt_capacity : opt RuntimeReceiptCapacityStatus")
            && status_env.contains("recent_failures : vec RecentFailure")
            && status_env.contains("visibility : vec RuntimeVisibilityEntry")
            && status_env.contains("type RuntimeAuthStatusSummary = record")
            && status_env.contains("auth_features : vec RuntimeFeatureStatus")
            && status_env.contains("type RuntimeBlobStorageStatusSummary = record")
            && status_env.contains("blob_storage_features : vec RuntimeFeatureStatus")
            && status_env.contains("type RuntimeReceiptCapacityStatus = record")
            && status_env.contains("receipt_record_limit : nat64")
            && status_env.contains("resource_total_record_limit : nat64")
            && status_env.contains("remaining_resource_total_headroom : nat64")
            && status_env.contains("warning_headroom_threshold : nat64")
            && status_env.contains("type CanicReadinessStatus = record")
            && status_env.contains("type RecentFailure = record")
            && status_env.contains("redacted : bool")
            && status_env.contains("type RuntimeFieldVisibility = variant")
            && status_env.contains("type CanicTimerStatus = record")
            && status_env.contains("scheduling_mode : TimerSchedulingMode")
            && status_env.contains("registration : TimerRegistrationStatus")
            && status_env.contains("condition : TimerProcessCondition")
            && status_env.contains("last_outcome : opt TimerExecutionOutcome")
            && status_env.contains("type TimerExecutionOutcome = variant")
            && status_env.contains("type TimerProcessCondition = variant")
            && status_env.contains("type TimerRegistrationStatus = variant")
            && status_env.contains("type TimerSchedulingMode = variant"),
        "runtime introspection DTO Candid changed:\n{status_env}"
    );
    for label in [
        "controller_only",
        "disabled",
        "feature_gated",
        "operator_only",
        "public_safe",
    ] {
        assert!(
            status_env.contains(label),
            "runtime introspection Candid labels must be canonical snake_case; missing {label}:\n{status_env}"
        );
    }

    let health_env = candid_type_env::<CanicHealthStatus>();
    assert!(
        health_env.contains("type CanicHealthStatus = record")
            && health_env.contains("status : HealthStatus")
            && health_env.contains("checks : vec RuntimeCheck"),
        "health DTO Candid changed:\n{health_env}"
    );
    for label in ["degraded", "healthy", "unhealthy", "unknown"] {
        assert!(
            health_env.contains(label),
            "health Candid labels must be canonical snake_case; missing {label}:\n{health_env}"
        );
    }

    let readiness_env = candid_type_env::<CanicReadinessStatus>();
    assert!(
        readiness_env.contains("type CanicReadinessStatus = record")
            && readiness_env.contains("blockers : vec RuntimeDiagnostic")
            && readiness_env.contains("warnings : vec RuntimeDiagnostic"),
        "readiness DTO Candid changed:\n{readiness_env}"
    );

    let _ = RuntimeFieldVisibility::ControllerOnly;
    let _ = RecentFailure {
        occurred_at_ns: 0,
        subsystem: String::new(),
        code: String::new(),
        severity: canic::dto::runtime::FailureSeverity::Info,
        summary: String::new(),
        correlation_id: None,
        redacted: true,
    };
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
        source.contains(&format!("const _: () = {marker};")),
        "lifecycle start macros must reference an actionable missing-finish marker"
    );
    assert!(
        source.contains(&format!("const {marker}: ()")),
        "finish! must define the same missing-finish marker"
    );
    assert!(
        marker.contains("missing_finish_macro")
            && marker.contains("add_canic_finish")
            && marker.contains("after_all_endpoints"),
        "missing-finish marker should read like a compiler-error hint"
    );
}
