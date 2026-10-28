use canic::{
    api::protocol::icrc21::Icrc21Dispatcher,
    dto::{
        auth::{
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
        cycles::Cycles,
        error::Error as CanicError,
        fleet_admission::{
            FleetAdmissionPreparedProjectionStatus, FleetAdmissionProjectionPhase,
            FleetAdmissionProjectionStatusResponse,
        },
        page::Page,
        rpc::{CyclesFundingPreflightResponse, CyclesResponse, Response as RootRpcResponse},
    },
    ids,
    ids::{
        CanisterRole, CanonicalNetworkId, ComponentBinding, ComponentInstanceId, ComponentSpecId,
        FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority,
        ManagedCanisterBinding, SubnetId,
    },
};
use canic_contracts::protocol::authority::{decode, encode};
use ic_platform::icrc_ledger_types::icrc21::{
    ConsentInfo, ConsentMessage, ConsentMessageMetadata, ConsentMessageRequest, ConsentMessageSpec,
    DisplayMessageType, Icrc21Error,
};
use ic_platform::{candid_parser::CandidSource, principal::ExternalPrincipal as Principal};
use std::{
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
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

fn assert_wire_roundtrip<T>(value: T)
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Eq + Debug,
{
    let encoded = encode(&value).expect("encode Wire value");
    let decoded = decode::<T>(&encoded).expect("decode Wire value");
    assert_eq!(decoded, value);
}

// Compare wire structure, independent of aliases, formatting and field order.

#[test]
fn fleet_admission_projection_wire_preserves_the_bounded_managed_role_contract() {
    use canic::dto::admission_control::ManagedStatus;
    use canic::protocol::admission_control::{RESPONSE_LIMITS, managed_status::METHOD_ID};
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
    let status_bytes = ic_wire::native::encode_response(
        METHOD_ID,
        &Ok::<ManagedStatus, CanicError>(status_value.clone().into()),
        RESPONSE_LIMITS,
    )
    .unwrap();
    let decoded: Result<ManagedStatus, CanicError> =
        ic_wire::native::decode_response(METHOD_ID, &status_bytes, RESPONSE_LIMITS).unwrap();
    let decoded: FleetAdmissionProjectionStatusResponse = decoded.unwrap().into();
    assert_eq!(decoded.authority, status_value.authority);
    assert_eq!(decoded.target, status_value.target);
    assert_eq!(decoded.principals.entries, status_value.principals.entries);
    assert_eq!(decoded.principals.total, status_value.principals.total);
    assert_eq!(decoded.generation, status_value.generation);
    assert_eq!(decoded.policy_digest, status_value.policy_digest);
    assert_eq!(decoded.projection_digest, status_value.projection_digest);
    assert_eq!(decoded.phase, status_value.phase);
    assert_eq!(decoded.prepared, status_value.prepared);
    assert_eq!(decoded.maximum_page_size, status_value.maximum_page_size);

    let maximum_principals = (0..256)
        .map(maximum_admission_principal)
        .collect::<Vec<_>>();
    let target = admission_target();
    let authority = match &target {
        ManagedCanisterBinding::Component(binding) => binding.authority.binding.clone(),
        ManagedCanisterBinding::ComponentChild(_) => unreachable!(),
    };
    let maximum_status: ManagedStatus = FleetAdmissionProjectionStatusResponse {
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
    }
    .into();
    let maximum_status = ic_wire::native::encode_response(
        METHOD_ID,
        &Ok::<_, CanicError>(maximum_status),
        RESPONSE_LIMITS,
    )
    .unwrap();
    assert!(maximum_status.len() < 16 * 1_024);
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
        assert_wire_roundtrip(response);
    }
}

#[test]
fn root_rpc_commands_without_result_data_use_unit_variants() {
    for response in [
        RootRpcResponse::AcknowledgePlacementReceipt,
        RootRpcResponse::RecycleCanister,
    ] {
        let encoded = encode(&response).expect("encode root RPC response");
        let decoded = decode::<RootRpcResponse>(&encoded).expect("decode root RPC unit response");
        assert_eq!(
            std::mem::discriminant(&decoded),
            std::mem::discriminant(&response)
        );
    }
}

#[test]
fn root_capability_surface_uses_component_registry_authority() {
    let source = read_text(
        &workspace_root().join("crates/canic/src/macros/endpoints/root_capability/mod.rs"),
    );
    assert!(source.contains("RootCapabilityCallerPredicate"));
    assert!(source.contains("AuthorityRestoreApi::require_command_variant_allowed(false)"));
    assert!(source.contains("ComponentRpcApi::response_capability_v1_root(envelope)"));
    for method in ["acknowledge", "allocate", "create", "recycle", "cycles"] {
        assert!(source.contains(&format!("fn canic_root_capability_{method}(")));
    }
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
    let request = consent_message_request("transfer");
    let bytes = ic_platform::candid::contract::encode_one(&request).unwrap();
    assert_eq!(
        ic_platform::candid::decoder::decode_one::<ConsentMessageRequest>(&bytes).unwrap(),
        request
    );

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
fn icrc21_dispatcher_uses_the_registered_typed_handler() {
    assert!(matches!(
        Icrc21Dispatcher::consent_message(consent_message_request(
            "unregistered_protocol_surface_method"
        )),
        Err(Icrc21Error::UnsupportedCanisterCall(_))
    ));
    let method = "protocol_surface_transfer";
    Icrc21Dispatcher::register(method, |request| {
        Ok(ConsentInfo {
            consent_message: ConsentMessage::GenericDisplayMessage(request.method),
            metadata: request.user_preferences.metadata,
        })
    });

    let Ok(info) = Icrc21Dispatcher::consent_message(consent_message_request(method)) else {
        panic!("registered handler should return consent information");
    };
    assert_eq!(
        info.consent_message,
        ConsentMessage::GenericDisplayMessage(method.to_string())
    );
}

#[test]
fn wasm_store_canonical_did_parses() {
    let did_path = workspace_root().join("crates/canic/candid/wasm_store.did");
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
        vec![canic::protocol::ICRC10_SUPPORTED_STANDARDS,],
        "canonical Store exposes its current read, control, byte and ICRC-10 methods"
    );
}

#[test]
fn provisioning_origin_preserves_explicit_deadline_presence() {
    use canic::dto::component_provisioning::{
        ProvisioningFailureOrigin, ProvisioningFailureStage, ProvisioningRetryCategory,
    };
    for retry_at_ns in [Some(u64::MAX), None] {
        let origin = ProvisioningFailureOrigin {
            dependency: None,
            failed_at_ns: 123,
            retry_at_ns,
            stage: ProvisioningFailureStage::ComponentMembership,
            target: Principal::from_slice(&[8]),
            operation_id: [9; 32],
            diagnostic_code: 137,
            retry_category: ProvisioningRetryCategory::Backoff,
        };
        assert_wire_roundtrip(origin);
    }
}

#[test]
fn infrastructure_role_ingress_names_match_current_protocol() {
    assert_eq!(
        canic::protocol::cycles_capability::METHOD,
        "canic_request_cycles"
    );
    assert_eq!(
        canic::protocol::batch_control::prepare::METHOD,
        "canic_coordinator_provision_components"
    );
    assert_eq!(
        canic::protocol::funding_read::coordinator_status::METHOD,
        "canic_coordinator_funding"
    );
    assert_eq!(
        canic::protocol::inspection::METHOD,
        "canic_root_inspect_canister"
    );
    assert_eq!(
        canic::protocol::funding_read::root_status::METHOD,
        "canic_root_funding"
    );
    assert_eq!(
        canic::protocol::component_directory::MEMBER_PAGE,
        "canic_component_directory_page"
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
            && endpoint_source.contains("fn canic_install_delegation_proof(")
            && endpoint_source.contains("ActiveDelegationProof"),
        "managed proof installation and status must be issuer-profile gated"
    );
    assert!(
        endpoint_source.contains("access::auth::is_controller(caller)")
            && endpoint_source.contains("AuthApi::install_active_delegation_proof"),
        "active-proof installation must remain controller authorized"
    );
}

#[test]
fn root_delegation_methods_are_native_and_authorized() {
    assert_root_provisioning_facade_is_public();

    let source = read_text(&workspace_root().join("crates/canic/src/macros/endpoints/root.rs"));
    for variant in [
        "fn canic_root_delegation_proof(",
        "fn canic_root_chain_key_public_key(",
        "fn canic_root_configure_issuer(",
    ] {
        assert!(
            source.contains(variant),
            "Root command surface lacks {variant}"
        );
    }
    assert!(source.contains("fn canic_root_issuer_renewal("));
    assert!(source.contains("::canic::protocol::issuer_renewal::METHOD_ID"));
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
fn root_delegation_proof_dtos_roundtrip_through_wire() {
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

    assert_wire_roundtrip(issuer_policy_request);
    assert_wire_roundtrip(issuer_policy_response);
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

    assert_wire_roundtrip(renewal_status_request);
    assert_wire_roundtrip(renewal_status_response);
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
    assert_wire_roundtrip(chain_key_proof);
    assert_wire_roundtrip(batch_proof);
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

    let native =
        canic::dto::issuer_proof::ActiveDelegationProofStatusResponse::from(status.clone());
    assert_eq!(ActiveDelegationProofStatusResponse::from(native), status);
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

#[cfg(feature = "wire-schema")]
#[test]
fn runtime_introspection_has_complete_native_schemas() {
    use canic::dto::{
        runtime::CanicHealthStatus,
        runtime_observation::{CanicReadinessStatus, CanicRuntimeStatus},
    };
    for schema in [
        ic_wire::typescript::native_schema::<CanicHealthStatus>().unwrap(),
        ic_wire::typescript::native_schema::<CanicReadinessStatus>().unwrap(),
        ic_wire::typescript::native_schema::<CanicRuntimeStatus>().unwrap(),
    ] {
        assert_eq!(schema["k"], "record");
        assert!(!schema["fields"].as_array().unwrap().is_empty());
    }
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

#[cfg(feature = "control-plane")]
#[cfg(feature = "control-plane")]
mod capacity_import_surface {
    use canic::{dto::pool_import::PoolImportIdentity, protocol::import_control as wire};
    canic::canic_emit_root_command_endpoint!();
    #[test]
    fn capacity_import_generated_root_command_preserves_operation_identity() {
        let identity = PoolImportIdentity {
            sequence: u64::MAX - 1,
            plan_sha256: [42; 32],
        };
        let bytes = ic_wire::native::encode_request(
            wire::settle::METHOD_ID,
            &identity,
            wire::REQUEST_LIMITS,
        )
        .unwrap();
        let restored: PoolImportIdentity =
            ic_wire::native::decode_request(wire::settle::METHOD_ID, &bytes, wire::REQUEST_LIMITS)
                .unwrap();
        assert_eq!(restored, identity);
    }
}

mod lean_observability_relay {
    #[test]
    fn disabled_provider_returns_the_same_typed_error_without_collection() {
        let collected = std::cell::Cell::new(false);
        let result: Result<canic::dto::memory::MemoryAllocationsResponse, canic::Error> =
            canic::__canic_optional_observation!(any(), {
                collected.set(true);
                unreachable!()
            });
        assert!(!collected.get());
        assert_eq!(
            result.as_ref().unwrap_err().clone(),
            canic::Error::from_registered(canic::diagnostics::codes::REQUEST_INVALID)
        );
        let protocol = canic::protocol::observability_relay::memory_allocations::METHOD_ID;
        let bytes = ic_wire::native::encode_response(
            protocol,
            &result,
            canic::protocol::observability_relay::memory_allocations::RESPONSE_LIMITS,
        )
        .unwrap();
        let decoded: Result<canic::dto::memory::MemoryAllocationsResponse, canic::Error> =
            ic_wire::native::decode_response(
                protocol,
                &bytes,
                canic::protocol::observability_relay::memory_allocations::RESPONSE_LIMITS,
            )
            .unwrap();
        assert_eq!(decoded, result);
    }
}
