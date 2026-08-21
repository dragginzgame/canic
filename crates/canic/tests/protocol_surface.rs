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
    dto::cycles::Cycles,
    dto::env::{EnvBootstrapArgs, EnvSnapshotResponse},
    dto::error::Error as CanicError,
    dto::fleet_activation::FleetActivationStatusResponse,
    dto::icp_refill::{IcpRefillDryRun, IcpRefillRequest},
    dto::icrc21::{
        ConsentInfo, ConsentMessage, ConsentMessageMetadata, ConsentMessageRequest,
        ConsentMessageResponse, ConsentMessageSpec, DisplayMessageType,
    },
    dto::memory::MemoryLedgerResponse,
    dto::page::Page,
    dto::rpc::{CyclesFundingPreflightResponse, CyclesResponse, Response as RootRpcResponse},
    dto::runtime::{
        CanicHealthStatus, CanicReadinessStatus, CanicRuntimeStatus, RecentFailure,
        RuntimeFieldVisibility,
    },
    dto::runtime_whitelist::{
        RuntimeWhitelistCommand, RuntimeWhitelistMutationOutcome, RuntimeWhitelistMutationRequest,
        RuntimeWhitelistMutationResponse, RuntimeWhitelistStatusResponse,
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

fn maximum_runtime_whitelist_principal(index: usize) -> Principal {
    let mut bytes = [0_u8; 29];
    bytes[..8].copy_from_slice(
        &u64::try_from(index)
            .expect("fixture index fits u64")
            .to_be_bytes(),
    );
    bytes[8..].fill(u8::try_from(index % 251).expect("bounded fixture byte"));
    Principal::from_slice(&bytes)
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

#[test]
fn endpoint_emitters_match_the_current_role_and_separate_blob_surfaces() {
    let endpoint_root = workspace_root().join("crates/canic/src/macros/endpoints");
    let mut methods = Vec::new();
    for file in [
        "blob_storage.rs",
        "blob_storage_billing.rs",
        "fleet_coordinator.rs",
        "role.rs",
        "root.rs",
        "standards.rs",
        "wasm_store.rs",
    ] {
        for line in read_text(&endpoint_root.join(file)).lines() {
            let trimmed = line.trim_start();
            let signature = trimmed
                .strip_prefix("fn ")
                .or_else(|| trimmed.strip_prefix("async fn "));
            let Some(name) = signature
                .and_then(|signature| signature.split_once('('))
                .map(|(name, _)| name)
                .filter(|name| name.starts_with("canic_"))
            else {
                continue;
            };
            methods.push(name.to_string());
        }
    }
    methods.sort();
    methods.dedup();

    assert_eq!(
        methods,
        [
            "canic_blob_storage_blobs_are_live",
            "canic_blob_storage_blobs_to_delete",
            "canic_blob_storage_confirm_blob_deletion",
            "canic_blob_storage_create_certificate",
            "canic_blob_storage_fund_from_project_cycles",
            "canic_blob_storage_status",
            "canic_blob_storage_update_gateway_principals",
            "canic_command",
            "canic_status",
            "canic_wasm_store_chunk",
            "canic_wasm_store_publish_chunk",
        ],
        "facade endpoint emitters diverged from the current role-owned and separately scoped blob surfaces"
    );
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
fn runtime_whitelist_candid_uses_the_bounded_managed_role_contract() {
    let principal = Principal::from_slice(&[0x31; 29]);
    let request = RuntimeWhitelistMutationRequest {
        principal,
        expected_revision: 7,
        operation_id: [0x41; 32],
    };
    assert_candid_roundtrip(RuntimeWhitelistCommand::Add(request.clone()));
    assert_candid_roundtrip(RuntimeWhitelistCommand::Remove(request));
    assert_candid_roundtrip(RuntimeWhitelistMutationResponse {
        outcome: RuntimeWhitelistMutationOutcome::AlreadyPresent,
        principal,
        revision: 7,
        membership_digest: [0x51; 32],
    });
    let status_value = RuntimeWhitelistStatusResponse {
        principals: Page {
            entries: vec![principal],
            total: 1,
        },
        revision: 7,
        membership_digest: [0x51; 32],
        maximum_principals: 256,
    };
    let status_bytes = encode_one(&status_value).expect("encode runtime-whitelist status");
    let decoded = decode_one::<RuntimeWhitelistStatusResponse>(&status_bytes)
        .expect("decode runtime-whitelist status");
    assert_eq!(decoded.principals.entries, status_value.principals.entries);
    assert_eq!(decoded.principals.total, status_value.principals.total);
    assert_eq!(decoded.revision, status_value.revision);
    assert_eq!(decoded.membership_digest, status_value.membership_digest);
    assert_eq!(decoded.maximum_principals, status_value.maximum_principals);

    let command = candid_type_env::<RuntimeWhitelistCommand>();
    for field in [
        "Add",
        "Remove",
        "expected_revision",
        "operation_id",
        "principal",
    ] {
        assert!(
            command.contains(field),
            "runtime-whitelist command omits {field}:\n{command}"
        );
    }
    let status = candid_type_env::<RuntimeWhitelistStatusResponse>();
    assert!(status.contains("principals") && status.contains("maximum_principals"));
    assert!(!status.contains("operation_id") && !status.contains("request_hash"));

    let maximum_principals = (0..256)
        .map(maximum_runtime_whitelist_principal)
        .collect::<Vec<_>>();
    let maximum_status = encode_one(RuntimeWhitelistStatusResponse {
        principals: Page {
            entries: maximum_principals[..128].to_vec(),
            total: 256,
        },
        revision: u64::MAX,
        membership_digest: [0xfb; 32],
        maximum_principals: 256,
    })
    .expect("maximum runtime-whitelist status Candid");
    let maximum_request = encode_one(RuntimeWhitelistMutationRequest {
        principal: maximum_principals[255],
        expected_revision: u64::MAX,
        operation_id: [0xfa; 32],
    })
    .expect("maximum runtime-whitelist request Candid");
    assert_eq!(maximum_status.len(), 4_072);
    assert_eq!(maximum_request.len(), 101);
}

#[test]
fn public_error_contract_is_the_compact_nat16_hard_cut() {
    let error_env = candid_type_env::<CanicError>();
    assert!(
        error_env.contains("type Error = record { code : nat16 }"),
        "public Error must contain only one compact nat16 code:\n{error_env}"
    );
    assert_candid_roundtrip(CanicError::from_registered(
        canic_core::diagnostics::codes::REQUEST_INVALID,
    ));

    for relative_path in [
        "crates/canic-fleet-coordinator/fleet_coordinator.did",
        "crates/canic-wasm-store/wasm_store.did",
    ] {
        let did_path = workspace_root().join(relative_path);
        let did = read_text(&did_path);
        assert!(
            did.contains("type Error = record { code : nat16 };"),
            "checked-in service DID lacks the compact Error shape in {relative_path}"
        );
    }
}

#[test]
fn cycles_preflight_contract_preserves_caller_continuation_values() {
    for response in [
        CyclesResponse::PreflightRejected(
            CyclesFundingPreflightResponse::ParentFundingUnavailable {
                approved_cycles: 123,
            },
        ),
        CyclesResponse::PreflightRejected(CyclesFundingPreflightResponse::ChildBudgetExhausted {
            remaining_child_budget: 456,
            max_per_child: 789,
        }),
        CyclesResponse::PreflightRejected(CyclesFundingPreflightResponse::CooldownActive {
            retry_after_secs: 30,
        }),
        CyclesResponse::Transferred {
            cycles_transferred: 100,
        },
    ] {
        assert_candid_roundtrip(response);
    }

    let response_env = candid_type_env::<CyclesResponse>();
    for field in [
        "approved_cycles : nat",
        "remaining_child_budget : nat",
        "max_per_child : nat",
        "retry_after_secs : nat64",
        "cycles_transferred : nat",
    ] {
        assert!(
            response_env.contains(field),
            "cycles response omits caller-required field {field}:\n{response_env}"
        );
    }
    assert!(
        !response_env.contains("available_cycles"),
        "cycles response must not expose the parent balance:\n{response_env}"
    );

    let relative_path = "crates/canic-wasm-store/wasm_store.did";
    let did = read_text(&workspace_root().join(relative_path));
    for field in [
        "approved_cycles : nat",
        "remaining_child_budget : nat",
        "max_per_child : nat",
        "retry_after_secs : nat64",
    ] {
        assert!(
            did.contains(field),
            "checked-in service DID omits {field} in {relative_path}"
        );
    }
    assert!(
        !did.contains("type CyclesResponse = record { cycles_transferred : nat };"),
        "checked-in service DID retains the pre-closeout cycles record in {relative_path}"
    );
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
    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/root.rs");
    let source = read_text(&macro_path);
    assert!(
        source.contains("RespondCapability(::canic::dto::capability::RootCapabilityEnvelopeV1)")
            && source.contains("if matches!(&command, RootCommand::RespondCapability(_))")
            && source.contains("RootCapabilityCallerPredicate")
            && source.contains("RootCommand::RespondCapability(envelope)")
            && source.contains("ComponentRpcApi::response_capability_v1_root(envelope)"),
        "Root RespondCapability must retain Component Registry authority inside its command variant"
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
fn local_application_authorization_facade_has_one_public_owner() {
    const READ: canic::access::auth::ApplicationScopeRef<'static> =
        canic::application_scope!("app:read");
    let request = canic::access::auth::LocalApplicationAuthorizationRequest {
        observed_transport_caller: Principal::anonymous(),
        required_scope: READ,
    };
    assert_eq!(request.required_scope.as_str(), "app:read");

    let facade: for<'a> fn(
        canic::access::auth::LocalApplicationAuthorizationRequest<'a>,
    ) -> canic::access::auth::LocalApplicationAuthorizationDecision =
        canic::access::auth::authorize_local_application;
    let _ = facade;

    assert!(
        !workspace_root()
            .join("crates/canic-core/src/access/application_authorization.rs")
            .exists(),
        "the intermediate scope-only facade module must not survive B5"
    );
}

#[test]
fn application_session_audit_is_bounded_protected_and_secret_free() {
    let audit_env = candid_type_env::<canic::dto::auth::ApplicationSessionAuditResponse>();
    for required in [
        "allowed_scopes",
        "authority_generation",
        "minimum_accepted_registry_epoch",
        "sessions",
        "transport_caller",
    ] {
        assert!(audit_env.contains(required), "audit omits {required}");
    }
    for forbidden in [
        "delegated_token",
        "establishment_request_hash",
        "proof_fingerprint",
        "proof_bytes",
    ] {
        assert!(
            !audit_env.contains(forbidden),
            "operator audit exposes {forbidden}"
        );
    }

    let role_surface =
        read_text(&workspace_root().join("crates/canic/src/macros/endpoints/role.rs"));
    let audit_authorization = role_surface
        .split("CanisterStatusRequest::ApplicationSessionAudit(_) =>")
        .nth(1)
        .expect("audit authorization arm");
    let audit_authorization = audit_authorization
        .split("CanisterStatusRequest::CycleBalance")
        .next()
        .expect("bounded audit authorization arm");
    assert!(audit_authorization.contains("access::auth::is_root(caller)"));
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

#[test]
fn wasm_store_exposes_cycle_history_only_through_status() {
    let did_path = workspace_root().join("crates/canic-wasm-store/wasm_store.did");
    let did = read_text(&did_path);

    assert!(
        did.contains("type PageRequest = record { offset : nat64; limit : nat64 };")
            && did.contains("CycleHistory : PageRequest"),
        "Store cycle history must be a status variant in {}",
        did_path.display()
    );
    assert!(
        !did.contains("type CycleTopupEvent = record")
            && !did.contains("type CycleTopupEventStatus = variant"),
        "Wasm Store must not retain AutomaticTopup types in {}",
        did_path.display()
    );
}

#[test]
fn role_capability_surfaces_are_pruned_at_the_destination_macro() {
    let root = workspace_root();
    let role_surface = read_text(&root.join("crates/canic/src/macros/endpoints/role.rs"));
    assert!(
        role_surface.contains("#[cfg(canic_capability_automatic_topup)]")
            && role_surface.contains("CanisterStatusRequest::CycleTopups")
            && role_surface.contains("CanisterStatusResponse::CycleTopups"),
        "the top-up status variant must compile only for AutomaticTopup profiles"
    );

    assert!(
        role_surface.contains("#[cfg(canic_capability_sharding)]")
            && role_surface.contains("CanisterStatusRequest::Children")
            && role_surface.contains("CanisterStatusResponse::Children"),
        "the managed children status variant must compile only for Sharding profiles"
    );

    assert_eq!(
        role_surface
            .matches("#[cfg(canic_capability_local_application_authorization)]")
            .count(),
        12,
        "application-session request, response, authorization and dispatch sites must share one compile-time capability"
    );
    for surface in [
        "CanisterCommand::ApplicationSession",
        "CanisterCommandResponse::ApplicationSession",
        "CanisterStatusRequest::ApplicationSession",
        "CanisterStatusRequest::ApplicationSessionAudit",
        "CanisterStatusResponse::ApplicationSession",
        "CanisterStatusResponse::ApplicationSessionAudit",
    ] {
        assert!(
            role_surface.contains(surface),
            "managed role surface omits {surface}"
        );
    }
    for relative_path in [
        "crates/canic/src/macros/endpoints/root.rs",
        "crates/canic/src/macros/endpoints/fleet_coordinator.rs",
        "crates/canic/src/macros/endpoints/wasm_store.rs",
    ] {
        assert!(
            !read_text(&root.join(relative_path)).contains("ApplicationSession"),
            "infrastructure surface acquired application-session authority in {relative_path}"
        );
    }

    let root_surface = read_text(&root.join("crates/canic/src/macros/endpoints/root.rs"));
    let command_macro = root_surface
        .split("macro_rules! canic_emit_root_command_endpoint")
        .nth(1)
        .and_then(|tail| {
            tail.split("macro_rules! canic_emit_root_status_endpoint")
                .next()
        })
        .expect("Root command destination macro");
    assert_eq!(
        command_macro
            .matches("#[cfg(canic_capability_role_attestation_signer)]")
            .count(),
        4,
        "Root attestation command request, response, authority, and dispatch must share one compile-time capability"
    );
    let status_macro = root_surface
        .split("macro_rules! canic_emit_root_status_endpoint")
        .nth(1)
        .expect("Root status destination macro");
    assert_eq!(
        status_macro
            .matches("#[cfg(canic_capability_role_attestation_signer)]")
            .count(),
        4,
        "Root attestation request, response, authority, and dispatch must share one compile-time capability"
    );

    assert!(
        !root_surface.contains("canic_emit_root_auth_attestation_endpoints"),
        "Root attestation must be pruned as command/status variants rather than standalone methods"
    );

    assert!(
        role_surface.contains("RespondCapability(")
            && role_surface.contains("CanisterCommandResponse::RespondCapability"),
        "managed capability dispatch must remain a command variant"
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
        did.contains("type FleetSubnetWasmStoreInitArgs = record")
            && did.contains("authority : FleetSubnetWasmStoreAuthority;")
            && did.contains("Authority : FleetSubnetWasmStoreAuthority")
            && did.contains(
                "type StateSnapshotInput = record { fleet_state : opt FleetStateInput };"
            )
            && did.contains("SynchronizeState : StateSnapshotInput")
            && !did.contains("type CanisterInitAuthority = variant")
            && !did.contains("FleetDirectoryInput")
            && !did.contains("fleet_directory"),
        "canonical Wasm-store DID must expose its exact sibling authority and state-cascade contract"
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
    let methods = service
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        methods,
        vec![
            canic::protocol::CANIC_COMMAND,
            canic::protocol::CANIC_STATUS,
            canic::protocol::CANIC_WASM_STORE_CHUNK,
            canic::protocol::CANIC_WASM_STORE_PUBLISH_CHUNK,
            canic::protocol::ICRC10_SUPPORTED_STANDARDS,
        ],
        "canonical Store must expose only role-owned control, two byte lanes and ICRC-10"
    );

    let status_env = candid_type_env::<FleetActivationStatusResponse>();
    assert!(status_env.contains("FleetActivationStatusResponse"));
    assert!(status_env.contains("FleetActivationIdentity"));
    assert!(status_env.contains("FleetCascadeActivationEvidence"));
    assert!(status_env.contains("FleetCredentialManifest"));
}

#[test]
fn wasm_store_status_surface_is_profile_exact() {
    let did_path = workspace_root().join("crates/canic-wasm-store/wasm_store.did");
    let did = read_text(&did_path);
    let request = did
        .split("type StoreStatusRequest = variant {")
        .nth(1)
        .and_then(|tail| tail.split("};").next())
        .expect("canonical Store DID must declare StoreStatusRequest");

    for variant in [
        "Authority",
        "Catalog",
        "CycleBalance",
        "CycleHistory : PageRequest",
        "Operation : OperationReceipt",
        "Overview",
        "Storage",
    ] {
        assert!(
            request.contains(variant),
            "StoreStatusRequest omits {variant}:\n{request}"
        );
    }
    assert!(
        !request.contains("CycleTopups"),
        "the implicit Store profile must not acquire AutomaticTopup"
    );
    assert!(
        did.contains(&format!(
            "{} : (StoreStatusRequest) -> (Result_1) query;",
            canic::protocol::CANIC_STATUS
        )),
        "canonical Store DID must expose its role-owned status query"
    );
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

    let methods = service
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        methods,
        vec![
            canic::protocol::CANIC_COMMAND,
            canic::protocol::CANIC_STATUS,
        ],
        "Fleet Coordinator must expose only its role-owned command and status methods"
    );
}

#[test]
fn fleet_coordinator_candid_contains_only_the_protected_policy_type_expansion() {
    let did =
        read_text(&workspace_root().join("crates/canic-fleet-coordinator/fleet_coordinator.did"));
    for declaration in [
        "root_funding : opt FleetCoordinatorRootFundingPolicy;",
        "type FleetCoordinatorRootFundingPolicy = record {",
        "type FleetSubnetRootFundingAuthority = record {",
        "type FleetSubnetRootFundingPolicy = record {",
        "type FleetSubnetRootIcpRefillPolicy = record {",
        "type FleetSubnetRootAutomaticIcpRefillPolicy = record {",
        "funding : FleetSubnetRootFundingAuthority;",
    ] {
        assert!(
            did.contains(declaration),
            "canonical Coordinator DID omits protected policy declaration {declaration}"
        );
    }
    assert_eq!(
        did.matches("funding : FleetSubnetRootFundingAuthority;")
            .count(),
        2,
        "only the protected root binding and Registry entry should carry root funding authority"
    );
}

#[test]
fn fleet_coordinator_command_surface_is_profile_exact() {
    let did_path = workspace_root().join("crates/canic-fleet-coordinator/fleet_coordinator.did");
    let did = read_text(&did_path);
    let request = did
        .split("type CoordinatorCommand = variant {")
        .nth(1)
        .and_then(|tail| tail.split("};").next())
        .expect("canonical Coordinator DID must declare CoordinatorCommand");

    for variant in [
        "AcknowledgeRootSnapshot",
        "ActivateRegistry",
        "CompleteRootDeletion",
        "JoinRoot",
        "PrepareAuthoritySnapshot",
        "PrepareRootDeletionExecution",
        "ProvisionComponents",
        "RemoveRoot",
        "ResumeAuthoritySnapshot",
    ] {
        assert!(
            request.contains(variant),
            "CoordinatorCommand omits {variant}:\n{request}"
        );
    }
    assert_eq!(
        request.lines().filter(|line| line.contains(';')).count(),
        9,
        "CoordinatorCommand acquired an unreviewed variant:\n{request}"
    );
}

#[test]
fn fleet_coordinator_status_surface_is_profile_exact() {
    let did_path = workspace_root().join("crates/canic-fleet-coordinator/fleet_coordinator.did");
    let did = read_text(&did_path);
    let request = did
        .split("type CoordinatorStatusRequest = variant {")
        .nth(1)
        .and_then(|tail| tail.split("};").next())
        .expect("canonical Coordinator DID must declare CoordinatorStatusRequest");

    for variant in [
        "AuthorityRestore",
        // Candid is structural: the extractor deduplicates the identical
        // OperationStatusRequest and OperationReceipt records under this name.
        "Operation : OperationReceipt",
        "Overview",
        "Registry",
        "RegistryManifest",
        "RegistryVersion",
        "RootAcknowledgements",
    ] {
        assert!(
            request.contains(variant),
            "CoordinatorStatusRequest omits {variant}:\n{request}"
        );
    }
    assert_eq!(
        request.lines().filter(|line| line.contains(';')).count(),
        7,
        "CoordinatorStatusRequest acquired an unreviewed variant:\n{request}"
    );
    assert!(
        did.contains("canic_status : (CoordinatorStatusRequest) -> (Result_1) query;"),
        "canonical Coordinator DID must expose its role-owned status query"
    );
}

#[test]
fn role_status_dispatchers_keep_variant_specific_authority() {
    let coordinator =
        read_text(&workspace_root().join("crates/canic/src/macros/endpoints/fleet_coordinator.rs"));
    assert!(
        coordinator.contains("CoordinatorStatusRequest::Operation(_)")
            && coordinator.contains("| CoordinatorStatusRequest::Overview")
            && coordinator.contains("| CoordinatorStatusRequest::Registry")
            && coordinator.contains("access::auth::is_controller(caller)"),
        "only Coordinator Overview and owner-authorized Operation/Registry may bypass the blanket controller check"
    );
    assert!(
        coordinator.contains("FleetCoordinatorApi::operation_status(")
            && coordinator.contains("FleetCoordinatorApi::registry_for_calling_status()"),
        "Coordinator participant-visible status variants must delegate to their caller-aware owners"
    );
    let coordinator_api = read_text(
        &workspace_root().join("crates/canic-control-plane/src/api/fleet_coordinator.rs"),
    );
    assert!(
        coordinator_api.contains("FleetCoordinatorWorkflow::operation_status_for_caller(")
            && coordinator_api.contains("FleetCoordinatorWorkflow::registry_for_caller(")
            && coordinator_api.matches("is_controller(&caller)").count() >= 2,
        "Coordinator Operation and Registry owners must receive the exact caller and controller fact"
    );

    let store =
        read_text(&workspace_root().join("crates/canic/src/macros/endpoints/wasm_store.rs"));
    let store_operation_authority = store
        .split("StoreStatusRequest::Operation(_) => {")
        .nth(1)
        .and_then(|tail| tail.split("StoreStatusRequest::CycleBalance").next())
        .expect("Store Operation authority arm");
    assert!(
        store_operation_authority.contains("access::auth::is_controller(caller)")
            && !store_operation_authority.contains("access::auth::is_root(caller)"),
        "the current Store Operation owner is Fleet activation and remains controller-only"
    );
    assert!(
        store.contains("StoreStatusRequest::Catalog | StoreStatusRequest::Storage")
            && store.contains("access::auth::is_root(caller)"),
        "Store catalogue and storage observations remain exact-root-only"
    );

    let root = read_text(&workspace_root().join("crates/canic/src/macros/endpoints/root.rs"));
    let root_operation_dispatch = root
        .split("RootStatusRequest::Operation(request) => {")
        .nth(1)
        .and_then(|tail| tail.split("RootStatusRequest::Overview").next())
        .expect("Root Operation dispatch arm");
    assert!(
        root_operation_dispatch.contains("request.operation_id")
            && root_operation_dispatch.contains("caller,")
            && root_operation_dispatch.contains("is_controller(&caller)"),
        "Root Operation dispatch must pass the caller and controller fact to its durable owner"
    );

    let root_operation = read_text(
        &workspace_root().join("crates/canic-control-plane/src/workflow/root_status/mod.rs"),
    );
    let allocation = read_text(
        &workspace_root().join("crates/canic-control-plane/src/workflow/component_registry/mod.rs"),
    );
    assert!(
        root_operation.contains("if !caller_is_controller")
            && allocation.contains("ComponentProvisioningOrigin::FleetAdministrator")
            && allocation.contains("revalidate_retained_peer_origin(")
            && allocation
                .contains("ComponentProvisioningOrigin::ComponentGroup { .. } => return Ok(None)"),
        "Root Operation authority must distinguish controller, exact peer, and group owners"
    );
}

#[test]
fn root_and_coordinator_role_ingress_are_command_status_only() {
    assert_eq!(canic::protocol::CANIC_COMMAND, "canic_command");
    assert_eq!(canic::protocol::CANIC_STATUS, "canic_status");

    let bundles = read_text(&workspace_root().join("crates/canic/src/macros/endpoints/bundles.rs"));
    let root_bundle = bundles
        .split("macro_rules! canic_bundle_root_only_endpoints")
        .nth(1)
        .and_then(|tail| tail.split("macro_rules!").next())
        .expect("Root endpoint bundle");
    assert!(
        root_bundle.contains("$crate::canic_emit_root_command_endpoint!();")
            && root_bundle.contains("$crate::canic_emit_root_status_endpoint!();"),
        "Root bundle must emit its one command and one status method"
    );
    assert_eq!(
        root_bundle.matches("$crate::canic_emit_").count(),
        2,
        "Root bundle acquired another endpoint family:\n{root_bundle}"
    );

    let root = read_text(&workspace_root().join("crates/canic/src/macros/endpoints/root.rs"));
    for variant in [
        "ProvisionComponent(",
        "ProvisionComponents(",
        "RemoveComponent(",
        "RemoveRoot(",
        "SynchronizeRegistry(",
    ] {
        assert!(
            root.contains(variant),
            "Root command union omits current intent {variant}"
        );
    }
    for variant in [
        "FleetAuthority",
        "Inventory",
        "Operation(",
        "Overview",
        "Pool(",
        "StoreOverview",
    ] {
        assert!(
            root.contains(variant),
            "Root status union omits current selector {variant}"
        );
    }
    assert!(
        root.contains("require_root_command_variant_allowed")
            && root.contains("require_root_status_variant_allowed")
            && root.contains("LifecycleApi::root_operation_status(")
            && root.contains("__canic_inspect_root_update_message")
            && root.contains("__canic_payload_max_bytes"),
        "Root ingress must retain variant-aware lifecycle and durable-operation authority"
    );
    for removed_phase in [
        "ActivateComponents(",
        "AdvanceComponents(",
        "PublishComponents(",
    ] {
        assert!(
            !root.contains(removed_phase),
            "Root command surface retained autonomous phase {removed_phase}"
        );
    }

    assert_coordinator_ingress_is_command_status_only();
}

fn assert_coordinator_ingress_is_command_status_only() {
    let coordinator = read_text(
        &workspace_root().join("crates/canic-control-plane/src/dto/fleet_coordinator.rs"),
    );
    for variant in [
        "ActivateRegistry(",
        "JoinRoot(",
        "ProvisionComponents(",
        "RemoveRoot(",
    ] {
        assert!(
            coordinator.contains(variant),
            "Coordinator command union omits current intent {variant}"
        );
    }
    let coordinator_endpoints =
        read_text(&workspace_root().join("crates/canic/src/macros/endpoints/fleet_coordinator.rs"));
    assert!(
        coordinator_endpoints.contains("__canic_inspect_fleet_coordinator_update_message")
            && coordinator_endpoints.contains("__canic_fleet_coordinator_payload_max_bytes"),
        "Coordinator ingress must decode the selected command before accepting its exact bound"
    );
    for variant in [
        "Operation(OperationStatusRequest)",
        "Overview",
        "Registry",
        "RegistryVersion",
    ] {
        assert!(
            coordinator.contains(variant),
            "Coordinator status union omits current selector {variant}"
        );
    }

    let did_path = workspace_root().join("crates/canic-fleet-coordinator/fleet_coordinator.did");
    let did = read_text(&did_path);
    let (env, actor) = CandidSource::Text(&did)
        .load()
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", did_path.display()));
    let actor = actor.unwrap_or_else(|| panic!("missing service in {}", did_path.display()));
    let service = env
        .as_service(&actor)
        .unwrap_or_else(|err| panic!("invalid service in {}: {err}", did_path.display()));
    let mut canic_methods = service
        .iter()
        .map(|(name, _)| name.as_str())
        .filter(|name| name.starts_with("canic_"))
        .collect::<Vec<_>>();
    canic_methods.sort_unstable();
    assert_eq!(canic_methods, ["canic_command", "canic_status"]);
}

#[test]
fn managed_and_store_activation_variants_are_guarded_by_the_exact_root() {
    let managed = read_text(&workspace_root().join("crates/canic/src/macros/endpoints/role.rs"));
    assert!(
        managed.contains("ConfigureRuntime(request)")
            && managed.contains("access::auth::is_root(caller)"),
        "managed runtime configuration must authorize its command variant through the exact Root"
    );

    let store =
        read_text(&workspace_root().join("crates/canic/src/macros/endpoints/wasm_store.rs"));
    assert!(
        store.contains("StoreCommand::ActivateFleet(_)")
            && store.contains("StoreCommand::PrepareFleetCredential(_)")
            && store.contains("access::auth::is_root(caller)"),
        "Store activation variants must authorize through the exact Root"
    );
}

#[test]
fn public_protocol_reexports_only_wasm_store_byte_lanes() {
    assert_eq!(
        canic::protocol::CANIC_WASM_STORE_CHUNK,
        "canic_wasm_store_chunk"
    );
    assert_eq!(
        canic::protocol::CANIC_WASM_STORE_PUBLISH_CHUNK,
        "canic_wasm_store_publish_chunk"
    );

    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/wasm_store.rs");
    let source = read_text(&macro_path);
    assert!(
        source.contains("StoreCommand::ReclaimDeletionCycles(request)")
            && source.contains("StoreCommand::RunGc(request)"),
        "Store control effects must be command variants"
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
    let endpoint_path = workspace_root().join("crates/canic/src/macros/endpoints/role.rs");
    let endpoint_source = read_text(&endpoint_path);
    assert!(
        endpoint_source.contains("#[cfg(canic_delegated_token_issuer)]")
            && endpoint_source.contains("InstallDelegationProof(")
            && endpoint_source.contains("PrepareDelegatedToken(")
            && endpoint_source.contains("ActiveDelegationProof")
            && endpoint_source.contains("DelegatedToken("),
        "managed auth command and status variants must be issuer-profile gated"
    );
    assert!(
        endpoint_source.contains("access::auth::is_controller(caller)")
            && endpoint_source.contains("AuthApi::install_active_delegation_proof"),
        "active-proof installation must remain controller authorized"
    );
}

#[test]
fn root_delegation_commands_are_variant_owned() {
    assert_root_provisioning_facade_is_public();

    let source = read_text(&workspace_root().join("crates/canic/src/macros/endpoints/root.rs"));
    for variant in [
        "GetOrCreateDelegationProof",
        "UpsertIssuerPolicy",
        "UpsertIssuerRenewalTemplate",
    ] {
        assert!(
            source.contains(variant),
            "Root command surface lacks {variant}"
        );
    }
    assert!(source.contains("IssuerRenewal(::canic::dto::auth::RootIssuerRenewalStatusRequest)"));
    assert!(source.contains("AuthApi::get_or_create_chain_key_delegation_proof_root"));
    assert!(source.contains("AuthApi::upsert_root_issuer_policy_root"));
    assert!(source.contains("AuthApi::upsert_root_issuer_renewal_template_root"));
    assert!(source.contains("AuthApi::root_issuer_renewal_status_root"));
    assert!(source.contains("ActiveComponentMemberPredicate"));
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
fn standalone_local_status_keeps_sensitive_selectors_controller_guarded() {
    let macro_path = workspace_root().join("crates/canic/src/macros/endpoints/role.rs");
    let source = read_text(&macro_path);
    let endpoint_macro = source
        .split("macro_rules! __canic_emit_local_status_endpoint")
        .nth(1)
        .and_then(|rest| {
            rest.split("macro_rules! __canic_emit_managed_command_endpoint")
                .next()
        })
        .expect("standalone-local status macro should exist");

    for selector in [
        "CanisterStatusRequest::Health",
        "CanisterStatusRequest::Logs(_)",
        "CanisterStatusRequest::Readiness",
        "CanisterStatusRequest::Runtime",
    ] {
        assert!(
            endpoint_macro.contains(selector),
            "standalone-local status authority omits {selector}"
        );
    }

    assert!(
        endpoint_macro.contains("access::auth::is_controller(caller)")
            && !endpoint_macro.contains("CanisterStatusRequest::Binding")
            && !endpoint_macro.contains("CanisterStatusRequest::Operation")
            && !endpoint_macro.contains("CanisterCommand"),
        "standalone-local status must stay controller-guarded and omit Fleet-only authority"
    );
}

#[test]
fn root_authority_restore_and_cycle_refill_are_variant_guarded() {
    let source = read_text(&workspace_root().join("crates/canic/src/macros/endpoints/root.rs"));
    assert!(source.contains("AuthorityRestoreApi::require_command_variant_allowed"));
    assert!(source.contains("RootCommand::PrepareAuthoritySnapshot(_)"));
    assert!(source.contains("RootCommand::ResumeAuthoritySnapshot(_)"));
    assert!(source.contains("RootCommand::PreviewCycleRefill(_)"));
    assert!(source.contains("RootCommand::RefillCycles(_)"));
    assert!(source.contains("let controller_command = matches!"));
    assert!(source.contains("LifecycleApi::prepare_authority_snapshot(request)"));
    assert!(source.contains("LifecycleApi::resume_authority_snapshot(request)"));
}

#[test]
fn root_and_coordinator_commands_authorize_before_state_gates_or_dispatch() {
    let root = read_text(&workspace_root().join("crates/canic/src/macros/endpoints/root.rs"));
    let root_command = root
        .split("async fn canic_command(")
        .nth(1)
        .and_then(|tail| tail.split("match command {").next())
        .expect("Root command admission");
    let root_state_gate = root_command
        .find("AuthorityRestoreApi::require_command_variant_allowed")
        .expect("Root authority-restore gate");
    for authorization in [
        "access::auth::is_controller(caller)",
        "authorize_fleet_subnet_root_removal_caller(",
        "ActiveComponentMemberPredicate",
        "authorize_component_child_caller(request, caller)",
        "authorize_coordinator_caller(caller)",
        "authorize_peer_component_allocation_caller(request, caller)",
        "RootCapabilityCallerPredicate",
    ] {
        let offset = root_command
            .find(authorization)
            .unwrap_or_else(|| panic!("Root admission lacks {authorization}"));
        assert!(
            offset < root_state_gate,
            "Root {authorization} must precede protected state gates"
        );
    }

    let coordinator =
        read_text(&workspace_root().join("crates/canic/src/macros/endpoints/fleet_coordinator.rs"));
    let coordinator_command = coordinator
        .split("async fn canic_command(")
        .nth(1)
        .and_then(|tail| tail.split("FleetCoordinatorApi::command(").next())
        .expect("Coordinator command admission");
    let coordinator_state_gate = coordinator_command
        .find("AuthorityRestoreApi::require_command_variant_allowed")
        .expect("Coordinator authority-restore gate");
    for authorization in [
        "authorize_calling_root_snapshot()",
        "access::auth::is_controller(caller)",
    ] {
        let offset = coordinator_command
            .find(authorization)
            .unwrap_or_else(|| panic!("Coordinator admission lacks {authorization}"));
        assert!(
            offset < coordinator_state_gate,
            "Coordinator {authorization} must precede protected state gates"
        );
    }
    let coordinator_status = coordinator
        .split("async fn canic_status(")
        .nth(1)
        .expect("Coordinator status admission");
    assert!(
        coordinator_status
            .find("authorize_calling_registry_status()")
            .expect("Coordinator Registry authorization")
            < coordinator_status
                .find("match request {")
                .expect("Coordinator status dispatch"),
        "Coordinator Registry authority must precede status dispatch"
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
            && status_env.contains("timer_inventory : RuntimeCheck")
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
            && status_env.contains("type CanisterTimerStatus = record")
            && status_env.contains("scheduler_performance : TimerCallbackPerformanceStatus")
            && status_env.contains("work_performance : TimerCallbackPerformanceStatus")
            && status_env.contains("type TimerCallbackPerformanceStatus = record")
            && status_env.contains("instruction_samples_since_runtime_start : nat64")
            && status_env.contains("memory_page_samples_since_runtime_start : nat64")
            && status_env.contains("memory_pages_latest : opt TimerMemoryPageSampleStatus")
            && status_env.contains("maximum_wasm_memory_growth_pages : opt nat64")
            && status_env.contains("maximum_stable_memory_growth_pages : opt nat64")
            && status_env.contains("type TimerMemoryPageSampleStatus = record")
            && status_env.contains("start : TimerMemoryPageExtentStatus")
            && status_env.contains("end : TimerMemoryPageExtentStatus")
            && status_env.contains("type TimerMemoryPageExtentStatus = record")
            && status_env.contains("wasm_pages : nat64")
            && status_env.contains("stable_pages : nat64")
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
