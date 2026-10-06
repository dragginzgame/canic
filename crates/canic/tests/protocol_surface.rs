use std::collections::{BTreeSet, HashSet};

use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use candid::types::internal::TypeContainer;
use candid::types::{Type, TypeEnv, TypeInner};
use candid::{Principal, decode_one, encode_one};
use candid_parser::utils::CandidSource;
use canic::dto::{component_provisioning, fleet_admission, fleet_funding, fleet_registry, role};
use canic::ids;
use canic::{
    api::protocol::icrc21::Icrc21Dispatcher,
    dto::auth::{
        ActiveDelegationProofStatus, ActiveDelegationProofStatusResponse, ChainKeyAlgorithm,
        ChainKeyBatchHeaderV1, ChainKeyBatchWitnessStepV1, ChainKeyBatchWitnessV1,
        ChainKeyDelegationCertV1, ChainKeyKeyId, ChainKeyRootSignatureV1, DelegatedRoleGrant,
        DelegationAudience, DelegationCert, DelegationProof, IcChainKeyBatchSignatureProofV1,
        IssuerProofAlgorithm, IssuerProofBinding, RootDelegationProofBatchProof,
        RootIssuerConfigureRequest, RootIssuerConfigureResponse, RootIssuerPolicyView,
        RootIssuerRenewalBatchStatus, RootIssuerRenewalBatchView, RootIssuerRenewalStateView,
        RootIssuerRenewalStatusRequest, RootIssuerRenewalStatusResponse,
        RootIssuerRenewalTemplateView, RootProof,
    },
    dto::cascade::StateSnapshotInput,
    dto::cycles::Cycles,
    dto::env::{EnvBootstrapArgs, EnvSnapshotResponse},
    dto::error::Error as CanicError,
    dto::fleet_activation::FleetActivationStatusResponse,
    dto::fleet_admission::{
        FleetAdmissionPreparedProjectionStatus, FleetAdmissionProjectionPhase,
        FleetAdmissionProjectionStatusResponse,
    },
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
    dto::state::{FleetCommand, FleetCommandResponse, FleetMode, FleetStateResponse},
    ids::{
        CanisterRole, CanonicalNetworkId, ComponentBinding, ComponentInstanceId, ComponentSpecId,
        FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority,
        ManagedCanisterBinding, SubnetId,
    },
};

fn test_fleet() -> FleetKey {
    FleetKey {
        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
        fleet_id: FleetId::from_generated_bytes([1; 32]),
    }
}

fn maximum_admission_principal(index: usize) -> Principal {
    let mut bytes = [0_u8; 29];
    bytes[..8].copy_from_slice(
        &u64::try_from(index)
            .expect("fixture index fits u64")
            .to_be_bytes(),
    );
    bytes[8..].fill(u8::try_from(index % 251).expect("bounded fixture byte"));
    Principal::from_slice(&bytes)
}

fn admission_target() -> ManagedCanisterBinding {
    let fleet = FleetBinding {
        fleet: test_fleet(),
        app: ids::AppId::from("test"),
    };
    let placement_subnet = SubnetId::from_principal(Principal::from_slice(&[2; 29]));
    ManagedCanisterBinding::Component(ComponentBinding {
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet,
                coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[3; 29])),
                coordinator: Principal::from_slice(&[4; 29]),
                recovery_controllers: Vec::new(),
            },
            epoch: 1,
        },
        component: ComponentInstanceId::from_generated_bytes([5; 32]),
        component_spec: ComponentSpecId::try_from(String::from("default"))
            .expect("default Component Spec"),
        spec_hash: [6; 32],
        role: CanisterRole::from("app"),
        placement_subnet,
        fleet_subnet_root: Principal::from_slice(&[7; 29]),
        canister_id: Principal::from_slice(&[8; 29]),
    })
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

// Compare wire structure, independent of aliases, formatting and field order.
fn candid_type_matches<T: candid::CandidType>(env: &TypeEnv, actual: &Type) -> bool {
    let mut env = env.clone();
    let mut rust = TypeContainer::new();
    let expected = rust.add::<T>();
    let expected = env.merge_type(rust.env, expected);
    candid::types::subtype::equal(&mut HashSet::default(), &env, actual, &expected).is_ok()
}

fn assert_candid_type<T: candid::CandidType>(env: &TypeEnv, name: &str) {
    let actual = env
        .find_type(name)
        .unwrap_or_else(|err| panic!("{name}: {err}"));
    assert!(
        candid_type_matches::<T>(env, actual),
        "{name} differs from the Rust wire contract: {actual}"
    );
}

fn candid_field(env: &TypeEnv, name: &str, field: &str) -> Type {
    let ty = env.trace_type(env.find_type(name).unwrap()).unwrap();
    let fields = match ty.as_ref() {
        TypeInner::Record(fields) | TypeInner::Variant(fields) => fields,
        other => panic!("{name} must be a record or variant, got {other:?}"),
    };
    fields
        .iter()
        .find(|entry| entry.id.get_id() == candid::idl_hash(field))
        .unwrap_or_else(|| panic!("{name} has no field {field}"))
        .ty
        .clone()
}

#[test]
fn wire_shape_checks_accept_aliases_and_layout_but_reject_changed_contracts() {
    for source in [
        "type Error = record { code : nat16 };",
        "// another layout\ntype Code = nat16; type Error=record{code:Code;};",
    ] {
        let (env, _) = CandidSource::Text(source).load().unwrap();
        assert_candid_type::<CanicError>(&env, "Error");
    }
    for source in [
        "type Error = record { code : nat32 };",
        "type Error = variant { code : nat16 };",
        "type Error = record { code : nat16; extra : text };",
    ] {
        let (env, _) = CandidSource::Text(source).load().unwrap();
        assert!(!candid_type_matches::<CanicError>(
            &env,
            env.find_type("Error").unwrap()
        ));
    }
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
fn fleet_admission_projection_candid_uses_the_bounded_managed_role_contract() {
    let principal = Principal::from_slice(&[0x31; 29]);
    let target = admission_target();
    let authority = match &target {
        ManagedCanisterBinding::Component(binding) => binding.authority.binding.clone(),
        ManagedCanisterBinding::ComponentChild(_) => unreachable!(),
    };
    let status_value = FleetAdmissionProjectionStatusResponse {
        authority,
        target,
        generation: 7,
        policy_digest: [0x41; 32],
        projection_digest: [0x51; 32],
        phase: FleetAdmissionProjectionPhase::Fenced,
        prepared: Some(FleetAdmissionPreparedProjectionStatus {
            generation: 8,
            policy_digest: [0x61; 32],
            projection_digest: [0x71; 32],
        }),
        principals: Page {
            entries: vec![principal],
            total: 1,
        },
        maximum_page_size: 128,
    };
    let status_bytes = encode_one(&status_value).expect("encode Fleet-admission status");
    let decoded = decode_one::<FleetAdmissionProjectionStatusResponse>(&status_bytes)
        .expect("decode Fleet-admission status");
    assert_eq!(decoded.principals.entries, status_value.principals.entries);
    assert_eq!(decoded.principals.total, status_value.principals.total);
    assert_eq!(decoded.generation, status_value.generation);
    assert_eq!(decoded.policy_digest, status_value.policy_digest);
    assert_eq!(decoded.projection_digest, status_value.projection_digest);
    assert_eq!(decoded.phase, status_value.phase);
    assert_eq!(decoded.prepared, status_value.prepared);
    assert_eq!(decoded.maximum_page_size, status_value.maximum_page_size);

    let status = candid_type_env::<FleetAdmissionProjectionStatusResponse>();
    for field in [
        "authority",
        "target",
        "generation",
        "policy_digest",
        "projection_digest",
        "phase",
        "prepared",
        "principals",
        "maximum_page_size",
    ] {
        assert!(
            status.contains(field),
            "projection status omits {field}:\n{status}"
        );
    }
    assert!(!status.contains("operation_id") && !status.contains("request_hash"));

    let maximum_principals = (0..256)
        .map(maximum_admission_principal)
        .collect::<Vec<_>>();
    let target = admission_target();
    let authority = match &target {
        ManagedCanisterBinding::Component(binding) => binding.authority.binding.clone(),
        ManagedCanisterBinding::ComponentChild(_) => unreachable!(),
    };
    let maximum_status = encode_one(FleetAdmissionProjectionStatusResponse {
        authority,
        target,
        generation: u64::MAX,
        policy_digest: [0xfb; 32],
        projection_digest: [0xfc; 32],
        phase: FleetAdmissionProjectionPhase::Open,
        prepared: None,
        principals: Page {
            entries: maximum_principals[..128].to_vec(),
            total: 256,
        },
        maximum_page_size: 128,
    })
    .expect("maximum Fleet-admission status Candid");
    assert!(maximum_status.len() < 16 * 1_024);

    let role_capability = candid_type_env::<canic::dto::role::RoleCapability>();
    assert!(role_capability.contains("FleetAdmissionProjection"));
}

#[test]
fn public_error_contract_is_the_compact_nat16_hard_cut() {
    let (env, _) = CandidSource::Text("type Error = record { code : nat16 };")
        .load()
        .unwrap();
    assert_candid_type::<CanicError>(&env, "Error");
    assert_candid_roundtrip(CanicError::from_registered(
        canic_core::diagnostics::codes::REQUEST_INVALID,
    ));

    for relative_path in [
        "crates/canic/candid/fleet_coordinator.did",
        "crates/canic/candid/wasm_store.did",
    ] {
        let did_path = workspace_root().join(relative_path);
        let did = read_text(&did_path);
        let (env, _) = CandidSource::Text(&did).load().unwrap();
        assert_candid_type::<CanicError>(&env, "Error");
    }
}

#[test]
fn authority_restore_release_status_matches_canonical_candid() {
    use canic::dto::authority_restore::{
        AuthorityRestoreFencePhase, AuthorityRestoreFenceStatusResponse,
    };

    let did = read_text(&workspace_root().join("crates/canic/candid/fleet_coordinator.did"));
    let (mut env, _) = CandidSource::Text(&did).load().unwrap();
    let canonical = env
        .find_type("AuthorityRestoreFenceStatusResponse")
        .unwrap()
        .clone();
    let mut rust = TypeContainer::new();
    let response = rust.add::<AuthorityRestoreFenceStatusResponse>();
    let response = env.merge_type(rust.env, response);
    candid::types::subtype::equal(&mut HashSet::default(), &env, &canonical, &response).unwrap();
    assert_candid_roundtrip(AuthorityRestoreFenceStatusResponse {
        authority_canister: Principal::from_slice(&[1]),
        phase: AuthorityRestoreFencePhase::ReleaseSealed {
            review_sha256: [2; 32],
            recipient: Principal::from_slice(&[3]),
        },
        operation_id: Some([4; 32]),
        history_total_num_changes: None,
        changed_at_ns: Some(5),
    });
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

    let relative_path = "crates/canic/candid/wasm_store.did";
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
}

#[test]
fn composed_framework_fleet_admission_facade_is_typed_and_synchronous() {
    let facade: fn() -> Result<Principal, canic::access::AccessError> =
        canic::fleet_admission::require_caller;
    let _ = facade;
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
    let authorization = preceding_attribute(&role_surface, "async fn canic_control_status");
    assert!(authorization.contains("requires(caller::is_root())"));
    assert!(role_surface.contains("ControlStatusRequest::ApplicationSessionAudit"));
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
fn wasm_store_exposes_cycle_history_through_observability() {
    let did_path = workspace_root().join("crates/canic/candid/wasm_store.did");
    let did = read_text(&did_path);

    let (env, _) = CandidSource::Text(&did).load().unwrap();
    assert!(candid_type_matches::<canic::dto::page::PageRequest>(
        &env,
        &candid_field(&env, "ObservabilityRequest", "CycleHistory")
    ));
}

#[test]
fn wasm_store_canonical_did_parses() {
    let did_path = workspace_root().join("crates/canic/candid/wasm_store.did");
    let did = read_text(&did_path);
    let (env, actor) = CandidSource::Text(&did)
        .load()
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", did_path.display()));

    assert_candid_type::<FleetKey>(&env, "FleetKey");
    assert_candid_type::<canic::dto::fleet_subnet_root::FleetSubnetWasmStoreInitArgs>(
        &env,
        "FleetSubnetWasmStoreInitArgs",
    );
    assert!(candid_type_matches::<
        canic::ids::FleetSubnetWasmStoreAuthority,
    >(
        &env,
        &candid_field(&env, "StoreStatusResponse", "Authority")
    ));
    assert!(candid_type_matches::<StateSnapshotInput>(
        &env,
        &candid_field(&env, "StoreCommand", "SynchronizeState")
    ));

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
            canic::protocol::CANIC_OBSERVABILITY,
            canic::protocol::CANIC_PUBLIC_STATUS,
            canic::protocol::CANIC_WASM_STORE_CATALOG,
            canic::protocol::CANIC_WASM_STORE_CHUNK,
            canic::protocol::CANIC_WASM_STORE_COMMAND,
            canic::protocol::CANIC_WASM_STORE_FIXTURE_CHUNK,
            canic::protocol::CANIC_WASM_STORE_PUBLISH_CHUNK,
            canic::protocol::CANIC_WASM_STORE_PUBLISH_FIXTURE,
            canic::protocol::CANIC_WASM_STORE_STATUS,
            canic::protocol::ICRC10_SUPPORTED_STANDARDS,
        ],
        "canonical Store exposes its current read, control, byte and ICRC-10 methods"
    );

    let status_env = candid_type_env::<FleetActivationStatusResponse>();
    assert!(status_env.contains("FleetActivationStatusResponse"));
    assert!(status_env.contains("FleetActivationIdentity"));
    assert!(status_env.contains("FleetCascadeActivationEvidence"));
    assert!(status_env.contains("FleetCredentialManifest"));
}

#[test]
fn canonical_store_exposes_public_and_protected_reads() {
    let did = read_text(&workspace_root().join("crates/canic/candid/wasm_store.did"));
    let (env, actor) = CandidSource::Text(&did).load().unwrap();
    let service = env.as_service(actor.as_ref().unwrap()).unwrap();
    for method in [
        canic::protocol::CANIC_PUBLIC_STATUS,
        canic::protocol::CANIC_OBSERVABILITY,
        canic::protocol::CANIC_WASM_STORE_CATALOG,
        canic::protocol::CANIC_WASM_STORE_STATUS,
    ] {
        assert!(service.iter().any(|(name, _)| name == method));
    }
}

#[test]
fn fleet_coordinator_canonical_did_parses() {
    let did_path = workspace_root().join("crates/canic/candid/fleet_coordinator.did");
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
            canic::protocol::CANIC_COORDINATOR_COMMAND,
            canic::protocol::CANIC_COORDINATOR_OPERATION_STATUS,
            canic::protocol::CANIC_COORDINATOR_REGISTRY,
            canic::protocol::CANIC_OBSERVABILITY,
            canic::protocol::CANIC_PUBLIC_STATUS,
        ],
        "Fleet Coordinator must expose only its role-owned command and status methods"
    );
}

#[test]
fn fleet_coordinator_candid_contains_protected_admission_and_funding_protocol_types() {
    macro_rules! contracts {
        ($env:ident, $module:ident: $($name:ident),+ $(,)?) => {
            $(assert_candid_type::<$module::$name>(&$env, stringify!($name));)+
        };
    }
    let did = read_text(&workspace_root().join("crates/canic/candid/fleet_coordinator.did"));
    let (env, _) = CandidSource::Text(&did).load().unwrap();
    contracts!(env, fleet_admission:
        FleetAdmissionMutationAction, FleetAdmissionMutationRequest,
        FleetAdmissionMutationResponse, FleetAdmissionOperationStatusResponse,
        FleetAdmissionPolicyStatus, FleetAdmissionStatusRequest, FleetAdmissionStatusResponse,
    );
    contracts!(env, fleet_funding:
        FleetFundingPolicyRotationApplyRequest, FleetFundingPolicyRotationBeginRequest,
        FleetFundingPolicyRotationPlanHeader, FleetFundingPolicyRotationReceipt,
        FleetFundingPolicyRotationRootPlan, FleetFundingPolicyRotationStageRootRequest,
        FleetFundingPolicyUsage, FleetRootFundingAcceptanceReceipt,
        FleetRootFundingAcceptanceRequest, FleetRootFundingNoGrantReason,
        FleetRootFundingRequest, FleetRootFundingResponse,
    );
    contracts!(env, component_provisioning:
        FleetComponentProvisioningRetryStage, FleetComponentProvisioningRootFailure,
        FleetComponentProvisioningPrepareRequest, FleetComponentProvisioningStatusResponse,
    );
    contracts!(env, ids:
        FleetCoordinatorRootFundingPolicy, FleetFundingProfile,
        FleetSubnetRootFundingAuthority, FleetSubnetRootFundingPolicy,
        FleetSubnetRootIcpRefillPolicy, FleetSubnetRootAutomaticIcpRefillPolicy,
        FleetSubnetRootBinding,
    );
    contracts!(env, fleet_registry: FleetSubnetRootEntry);
    contracts!(env, role: RoleCapability);
    #[cfg(feature = "fleet-coordinator-canister")]
    assert_candid_type::<
        canic_control_plane::dto::fleet_coordinator::FleetFundingPolicyRotationStatusResponse,
    >(&env, "FleetFundingPolicyRotationStatusResponse");
}

#[test]
fn fleet_coordinator_diagnostic_and_retirement_types_match_rust() {
    fn assert_current_type<T: candid::CandidType>(name: &str) {
        let did = read_text(&workspace_root().join("crates/canic/candid/fleet_coordinator.did"));
        let (mut env, _) = CandidSource::Text(&did)
            .load()
            .expect("parse canonical Coordinator Candid");
        let canonical = env
            .find_type(name)
            .expect("canonical retirement type")
            .clone();
        let mut rust = TypeContainer::new();
        let ty = rust.add::<T>();
        let ty = env.merge_type(rust.env, ty);
        candid::types::subtype::equal(&mut HashSet::default(), &env, &canonical, &ty)
            .expect("canonical retirement type must equal the current Rust contract");
    }
    use canic::dto::fleet_registry::{
        FleetRetirementStatus, FleetSubnetRootDeletionReadinessRequest,
        FleetSubnetRootDrainingReservationRequest,
    };
    assert_current_type::<FleetSubnetRootDeletionReadinessRequest>(
        "FleetSubnetRootDeletionReadinessRequest",
    );
    assert_current_type::<FleetSubnetRootDrainingReservationRequest>(
        "FleetSubnetRootDrainingReservationRequest",
    );
    assert_current_type::<FleetRetirementStatus>("FleetRetirementStatus");
    assert_current_type::<canic::dto::component_provisioning::ProvisioningFailureOrigin>(
        "ProvisioningFailureOrigin",
    );
}

#[test]
fn provisioning_origin_preserves_explicit_deadline_presence() {
    use canic::dto::component_provisioning::{
        ProvisioningFailureOrigin, ProvisioningFailureStage, ProvisioningRetryCategory,
    };
    for retry_at_ns in [Some(u64::MAX), None] {
        let origin = ProvisioningFailureOrigin {
            failed_at_ns: 123,
            retry_at_ns,
            stage: ProvisioningFailureStage::ComponentMembership,
            target: Principal::from_slice(&[8]),
            operation_id: [9; 32],
            diagnostic_code: 137,
            retry_category: ProvisioningRetryCategory::Backoff,
        };
        assert_candid_roundtrip(origin);
    }
}

#[test]
fn fleet_coordinator_command_surface_is_profile_exact() {
    let did_path = workspace_root().join("crates/canic/candid/fleet_coordinator.did");
    let did = read_text(&did_path);
    let (env, _) = CandidSource::Text(&did).load().unwrap();
    let request = env
        .trace_type(env.find_type("CoordinatorCommand").unwrap())
        .unwrap();
    let TypeInner::Variant(variants) = request.as_ref() else {
        panic!("CoordinatorCommand must be a variant");
    };

    let expected = [
        "AcknowledgeRootSnapshot",
        "ActivateRegistry",
        "ApplyFundingPolicyRotation",
        "BeginFundingPolicyRotation",
        "CompleteRootDeletion",
        "JoinRoot",
        "MutateAdmission",
        "PrepareAuthoritySnapshot",
        "PrepareRootDeletionExecution",
        "ProvisionComponents",
        "RemoveRoot",
        "RequestRootFunding",
        "ResumeAuthoritySnapshot",
        "Retire",
        "SetRootFunding",
        "StageFundingPolicyRotationRoot",
    ];
    assert_eq!(
        variants
            .iter()
            .map(|variant| variant.id.get_id())
            .collect::<BTreeSet<_>>(),
        expected
            .into_iter()
            .map(candid::idl_hash)
            .collect::<BTreeSet<_>>(),
        "CoordinatorCommand must expose the reviewed command identities"
    );
}

#[cfg(any(
    feature = "fleet-coordinator-canister",
    feature = "control-plane",
    feature = "wasm-store-canister"
))]
mod infrastructure_read_contracts {
    use super::*;

    fn assert_contract<Q: candid::CandidType, R: candid::CandidType>(path: &str, method: &str) {
        let did = read_text(&workspace_root().join(path));
        let (mut env, actor) = CandidSource::Text(&did)
            .load()
            .expect("parse canonical read contract");
        let service = env
            .as_service(actor.as_ref().expect("canonical service"))
            .unwrap();
        let (_, ty) = service
            .iter()
            .find(|(name, _)| name == method)
            .expect("current read endpoint");
        let function = env.as_func(ty).unwrap();
        assert_eq!(function.args.len(), 1);
        assert_eq!(function.rets.len(), 1);
        assert_eq!(function.modes, vec![candid::types::FuncMode::Query]);
        let canonical_request = function.args[0].clone();
        let canonical_response = function.rets[0].clone();
        let mut rust = TypeContainer::new();
        let request = rust.add::<Q>();
        let response = rust.add::<Result<R, canic::Error>>();
        let request = env.merge_type(rust.env.clone(), request);
        let response = env.merge_type(rust.env, response);
        for (canonical, ty) in [(canonical_request, request), (canonical_response, response)] {
            candid::types::subtype::equal(&mut HashSet::default(), &env, &canonical, &ty)
                .expect("read contract matches its current authority owner");
        }
    }

    #[cfg(feature = "fleet-coordinator-canister")]
    #[test]
    fn coordinator_reads_match_their_authority_owner() {
        use canic::dto::fleet_coordinator::{
            CoordinatorObservabilityRequest, CoordinatorObservabilityResponse,
            CoordinatorOperationReadRequest, CoordinatorOperationReadResponse,
            CoordinatorRegistryRequest, CoordinatorRegistryResponse,
        };

        let coordinator = "crates/canic/candid/fleet_coordinator.did";
        assert_contract::<CoordinatorObservabilityRequest, CoordinatorObservabilityResponse>(
            coordinator,
            canic::protocol::CANIC_OBSERVABILITY,
        );
        assert_contract::<CoordinatorRegistryRequest, CoordinatorRegistryResponse>(
            coordinator,
            canic::protocol::CANIC_COORDINATOR_REGISTRY,
        );
        assert_contract::<CoordinatorOperationReadRequest, CoordinatorOperationReadResponse>(
            coordinator,
            canic::protocol::CANIC_COORDINATOR_OPERATION_STATUS,
        );
    }

    #[cfg(any(feature = "control-plane", feature = "wasm-store-canister"))]
    #[test]
    fn store_reads_match_their_authority_owner() {
        use canic::dto::template::{
            StoreCatalogRequest, StoreCatalogResponse, StoreObservabilityRequest,
            StoreObservabilityResponse, StoreStatusRequest, StoreStatusResponse,
        };

        let store = "crates/canic/candid/wasm_store.did";
        assert_contract::<StoreStatusRequest, StoreStatusResponse>(
            store,
            canic::protocol::CANIC_WASM_STORE_STATUS,
        );
        assert_contract::<StoreCatalogRequest, StoreCatalogResponse>(
            store,
            canic::protocol::CANIC_WASM_STORE_CATALOG,
        );
        assert_contract::<StoreObservabilityRequest, StoreObservabilityResponse>(
            store,
            canic::protocol::CANIC_OBSERVABILITY,
        );
    }
}

#[test]
fn infrastructure_role_ingress_names_match_current_protocol() {
    assert_eq!(canic::protocol::CANIC_COMMAND, "canic_command");
    assert_eq!(
        canic::protocol::CANIC_COORDINATOR_COMMAND,
        "canic_coordinator_command"
    );
    assert_eq!(canic::protocol::CANIC_OBSERVABILITY, "canic_observability");
    assert_eq!(canic::protocol::CANIC_ROOT_COMMAND, "canic_root_command");
    assert_eq!(canic::protocol::CANIC_ROOT_STATUS, "canic_root_status");
    assert_eq!(canic::protocol::CANIC_PUBLIC_STATUS, "canic_public_status");
    assert_eq!(
        canic::protocol::CANIC_WASM_STORE_COMMAND,
        "canic_wasm_store_command"
    );
    assert_eq!(
        canic::protocol::CANIC_WASM_STORE_STATUS,
        "canic_wasm_store_status"
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
}

#[test]
fn active_delegation_proof_installer_surface_is_issuer_gated() {
    let endpoint_path = workspace_root().join("crates/canic/src/macros/endpoints/role.rs");
    let endpoint_source = read_text(&endpoint_path);
    assert!(
        endpoint_source.contains("#[cfg(canic_capability_delegated_token_issuer)]")
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
        "GetChainKeyPublicKey",
        "ConfigureIssuer",
    ] {
        assert!(
            source.contains(variant),
            "Root command surface lacks {variant}"
        );
    }
    assert!(source.contains("IssuerRenewal(::canic::dto::auth::RootIssuerRenewalStatusRequest)"));
    assert!(source.contains("AuthApi::get_or_create_chain_key_delegation_proof_root"));
    assert!(source.contains("AuthApi::configure_issuer_root"));
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
    assert_root_issuer_configure_dtos_roundtrip();
    assert_root_issuer_renewal_dtos_roundtrip();
    assert_root_delegation_proof_dtos_roundtrip();
    assert_active_delegation_proof_status_roundtrip();
}

fn assert_root_issuer_configure_dtos_roundtrip() {
    let issuer_pid = Principal::from_slice(&[17; 29]);
    let grant = test_delegated_role_grant();
    let audience = DelegationAudience::Fleet(test_fleet());
    let issuer_policy_request =
        root_issuer_configure_request(issuer_pid, audience.clone(), grant.clone());
    let issuer_policy_response = root_issuer_configure_response(issuer_pid, audience, grant);

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

fn root_issuer_configure_request(
    issuer_pid: Principal,
    audience: DelegationAudience,
    grant: DelegatedRoleGrant,
) -> RootIssuerConfigureRequest {
    RootIssuerConfigureRequest {
        issuer_pid,
        enabled: true,
        aud: audience,
        grants: vec![grant],
        cert_ttl_ns: 60,
        refresh_after_ratio_bps: 8_000,
    }
}

fn root_issuer_configure_response(
    issuer_pid: Principal,
    audience: DelegationAudience,
    grant: DelegatedRoleGrant,
) -> RootIssuerConfigureResponse {
    RootIssuerConfigureResponse {
        issuer: RootIssuerPolicyView {
            issuer_pid,
            enabled: true,
            allowed_audiences: vec![audience.clone()],
            allowed_grants: vec![grant.clone()],
            max_cert_ttl_ns: 60,
            refresh_after_ratio_bps: 8_000,
        },
        template: RootIssuerRenewalTemplateView {
            issuer_pid,
            enabled: true,
            aud: audience,
            grants: vec![grant],
            cert_ttl_ns: 60,
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
            && ledger_env.contains("ledger_memory_manager_id : nat8")
            && ledger_env.contains("schema_version : opt nat32")
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
            && status_env.contains("receipt_capacity : opt RuntimeReceiptCapacityStatus")
            && status_env.contains("timer_inventory : RuntimeCheck")
            && status_env.contains("recent_failures : vec RecentFailure")
            && status_env.contains("visibility : vec RuntimeVisibilityEntry")
            && status_env.contains("type RuntimeAuthStatusSummary = record")
            && status_env.contains("auth_features : vec RuntimeFeatureStatus")
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
}

#[test]
fn public_history_canonical_types_match_rust() {
    fn check<T: candid::CandidType>(name: &str) {
        let did = read_text(&workspace_root().join("crates/canic/candid/wasm_store.did"));
        let (mut env, _) = CandidSource::Text(&did)
            .load()
            .expect("canonical Store Candid");
        let canonical = env.find_type(name).unwrap().clone();
        let mut rust = TypeContainer::new();
        let ty = rust.add::<T>();
        let ty = env.merge_type(rust.env, ty);
        candid::types::subtype::equal(&mut HashSet::default(), &env, &canonical, &ty)
            .expect("canonical public metrics type equals current Rust contract");
    }
    use canic::dto::public_status::{PublicHistoryRequest, PublicHistorySnapshot, PublicMetric};
    check::<PublicMetric>("PublicMetric");
    check::<PublicHistoryRequest>("PublicHistoryRequest");
    check::<PublicHistorySnapshot>("PublicHistorySnapshot");
}

#[test]
fn public_health_canonical_types_match_rust() {
    for file in ["fleet_coordinator.did", "wasm_store.did"] {
        let did = read_text(&workspace_root().join("crates/canic/candid").join(file));
        let (mut env, _) = CandidSource::Text(&did).load().expect("canonical Candid");
        let canonical = env.find_type("PublicHealth").unwrap().clone();
        let mut rust = TypeContainer::new();
        let ty = rust.add::<canic::dto::public_status::PublicHealth>();
        let ty = env.merge_type(rust.env, ty);
        candid::types::subtype::equal(&mut HashSet::default(), &env, &canonical, &ty)
            .expect("canonical public health equals current Rust contract");
    }
}

#[test]
fn fleet_recovery_controller_binding_matches_canonical_candid() {
    for file in ["fleet_coordinator.did", "wasm_store.did"] {
        let did = read_text(&workspace_root().join("crates/canic/candid").join(file));
        let (mut env, _) = CandidSource::Text(&did).load().expect("canonical Candid");
        let canonical = env.find_type("FleetCoordinatorBinding").unwrap().clone();
        let mut rust = TypeContainer::new();
        let ty = rust.add::<ids::FleetCoordinatorBinding>();
        let ty = env.merge_type(rust.env, ty);
        candid::types::subtype::equal(&mut HashSet::default(), &env, &canonical, &ty)
            .expect("Fleet recovery controller binding matches canonical Candid");
    }
}

#[cfg(feature = "control-plane")]
#[test]
fn fleet_funding_rotation_status_matches_canonical_candid_after_recovery_binding() {
    let did = read_text(&workspace_root().join("crates/canic/candid/fleet_coordinator.did"));
    let (mut env, _) = CandidSource::Text(&did).load().expect("Coordinator Candid");
    let canonical = env
        .find_type("FleetFundingPolicyRotationStatusPhase")
        .unwrap()
        .clone();
    let mut rust = TypeContainer::new();
    let ty = rust
        .add::<canic_control_plane::dto::fleet_coordinator::FleetFundingPolicyRotationStatusPhase>(
        );
    let ty = env.merge_type(rust.env, ty);
    candid::types::subtype::equal(&mut HashSet::default(), &env, &canonical, &ty)
        .expect("funding rotation status retains its canonical Candid shape");
}

#[test]
fn provisioning_failure_stage_matches_canonical_candid() {
    let did = read_text(&workspace_root().join("crates/canic/candid/fleet_coordinator.did"));
    let (mut env, _) = CandidSource::Text(&did).load().expect("Coordinator Candid");
    let canonical = env.find_type("ProvisioningFailureStage").unwrap().clone();
    let mut rust = TypeContainer::new();
    let ty = rust.add::<canic::dto::component_provisioning::ProvisioningFailureStage>();
    let ty = env.merge_type(rust.env, ty);
    candid::types::subtype::equal(&mut HashSet::default(), &env, &canonical, &ty)
        .expect("protected provisioning stage equals the current Rust contract");
}

#[cfg(any(feature = "control-plane", feature = "wasm-store-canister"))]
#[test]
fn state_cascade_store_response_matches_canonical_candid() {
    let did = read_text(&workspace_root().join("crates/canic/candid/wasm_store.did"));
    let (mut env, _) = CandidSource::Text(&did)
        .load()
        .expect("canonical Store Candid");
    let canonical = env.find_type("StoreCommandResponse").unwrap().clone();
    let mut rust = TypeContainer::new();
    let ty = rust.add::<canic_control_plane::dto::template::StoreCommandResponse>();
    let ty = env.merge_type(rust.env, ty);
    candid::types::subtype::equal(&mut HashSet::default(), &env, &canonical, &ty)
        .expect("Store response contract equals current Rust, including cascade outcomes");
}

#[cfg(feature = "wasm-store-canister")]
#[test]
fn store_preparation_command_matches_canonical_candid() {
    let did = read_text(&workspace_root().join("crates/canic/candid/wasm_store.did"));
    let (mut env, _) = CandidSource::Text(&did)
        .load()
        .expect("canonical Store Candid");
    let mut rust = TypeContainer::new();
    let command = rust.add::<canic_control_plane::dto::template::StoreCommand>();
    let chunk = rust.add::<canic_control_plane::dto::template::TemplateChunkInput>();
    for (name, ty) in [("StoreCommand", command), ("TemplateChunkInput", chunk)] {
        let canonical = env.find_type(name).unwrap().clone();
        let ty = env.merge_type(rust.env.clone(), ty);
        candid::types::subtype::equal(&mut HashSet::default(), &env, &canonical, &ty)
            .expect("Store publication matches its canonical Candid contract");
    }
}

#[cfg(feature = "control-plane")]
mod capacity_import_surface {
    use candid::{decode_one, encode_one};
    use canic::dto::pool_import::{PoolImportCommand, PoolImportIdentity};

    canic::canic_emit_root_command_endpoint!();

    #[test]
    fn capacity_import_generated_root_command_preserves_operation_identity() {
        let identity = PoolImportIdentity {
            sequence: u64::MAX - 1,
            plan_sha256: [42; 32],
        };
        let command = RootCommand::ImportPoolCapacity(PoolImportCommand::Settle(identity));
        let restored = decode_one::<RootCommand>(&encode_one(command).unwrap()).unwrap();
        assert!(matches!(restored,
            RootCommand::ImportPoolCapacity(PoolImportCommand::Settle(found)) if found == identity
        ));
    }
}

mod lean_observability_relay {
    use super::*;
    canic::__canic_emit_relay_observability_response!();

    #[test]
    fn lean_reply_decodes_at_root_and_disabled_reads_fail_without_collecting() {
        use canic::dto::observability::{
            CanisterObservabilityRequest, CanisterObservabilityResponse,
        };
        let encoded = encode_one(RelayedObservabilityResponse::CycleBalance(
            canic::dto::role::CycleBalanceStatusResponse { cycles: 123 },
        ))
        .unwrap();
        let decoded: CanisterObservabilityResponse = decode_one(&encoded).unwrap();
        assert!(
            matches!(decoded, CanisterObservabilityResponse::CycleBalance(value) if value.cycles == 123)
        );
        for request in [
            CanisterObservabilityRequest::MemoryAllocations,
            CanisterObservabilityRequest::CycleHistory(canic::dto::page::PageRequest {
                offset: 0,
                limit: 10,
            }),
        ] {
            let result = canic::__canic_sensitive_observability_response!(
                request,
                RelayedObservabilityResponse
            );
            assert!(
                matches!(result, Err(error) if error == canic::Error::from_registered(canic::diagnostics::codes::REQUEST_INVALID))
            );
        }
        let candid = candid_type_env::<RelayedObservabilityResponse>();
        assert!(candid.contains("CycleBalance"));
        assert!(candid.contains("ChildFunding"));
        assert!(!candid.contains("MemoryAllocations"));
        assert!(!candid.contains("CycleHistory"));
        assert!(!candid.contains("Metrics"));
    }
}
