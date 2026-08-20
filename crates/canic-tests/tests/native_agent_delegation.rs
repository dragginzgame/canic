use candid::{CandidType, Decode, Deserialize, Encode, Principal};
use canic::{
    Error,
    dto::{
        auth::{
            ApplicationSessionAuditResponse, ApplicationSessionCommand,
            ApplicationSessionCommandResponse, ApplicationSessionRequest, ApplicationSessionStatus,
            ApplicationSessionView, AuthRequestMetadata, DelegatedToken, DelegatedTokenGetRequest,
            DelegatedTokenPrepareRequest, DelegatedTokenPrepareResponse, DelegationAudience,
            InactiveApplicationSession, RootIssuerPolicyResponse, RootIssuerPolicyUpsertRequest,
            RootIssuerRenewalBatchStatus, RootIssuerRenewalStatusRequest,
            RootIssuerRenewalStatusResponse, RootIssuerRenewalTemplateResponse,
            RootIssuerRenewalTemplateUpsertRequest,
        },
        metrics::{MetricEntry, MetricValue, MetricsKind, QueryPerfSample},
        page::{Page, PageRequest},
        role::MetricsStatusRequest,
        runtime_whitelist::{
            RuntimeWhitelistCommand, RuntimeWhitelistMutationOutcome,
            RuntimeWhitelistMutationRequest, RuntimeWhitelistMutationResponse,
            RuntimeWhitelistStatusResponse,
        },
    },
    ids::{CanisterRole, FleetKey, cap},
    protocol,
};
use canic_testing_internal::pic::{
    managed_test_init_identity, report_canister_diagnostics, role_grant,
    setup_fresh_active_component_registry, upgrade_args,
};
use ic_agent::{Agent, Identity, identity::Secp256k1Identity};
use ic_stable_structures::{
    Cell, VectorMemory,
    memory_manager::{MemoryId, MemoryManager},
};
use ic_testkit::pic::{CandidCallExt, CanisterInstallExt, PocketIcTimeExt};
use pocket_ic::common::rest::BlobCompression;
use serde::Serialize;
use std::{cell::RefCell, rc::Rc, time::Duration};

const AUTH_STATE_MEMORY_ID: u8 = 34;
const MAX_ACTIVE_APPLICATION_SESSIONS: usize = 2_048;
const MAX_APPLICATION_REPLAY_RECORDS: usize = 4_096;
const MAX_APPLICATION_SESSION_CLEANUP_REMOVALS: u64 = 128;
const MAX_APPLICATION_SESSION_STABLE_BYTES: usize = 8 * 1024 * 1024;
const MAX_LOCAL_AUTHORIZATION_INSTRUCTIONS: u64 = 1_000_000;
const INSTALL_CODE_COOLDOWN: Duration = Duration::from_mins(5);
const ROOT_PROOF_PROVISION_ATTEMPTS: usize = 10;

// Deterministic test-only identity; it has no authority outside this fresh PocketIC runtime.
const TEST_IDENTITY_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIGEAgEAMBAGByqGSM49AgEGBSuBBAAKBG0wawIBAQQgCDLudkRxUeRDhnUp2pvL
xLDICLIoNCa1sQdMgz5Y14GhRANCAASA7zusnWjPN0y8nJlD4YAEOpTEYu+CcCdO
VwidXc26G4+/g7dUbMwbN4E3d3bpxHEP31M+2by6jY67MqFKKroR
-----END PRIVATE KEY-----";

fn maximum_application_scopes() -> Vec<String> {
    (0..16)
        .map(|scope| format!("app{scope:02}:{}", "x".repeat(58)))
        .collect()
}

fn delegated_grant_scopes() -> Vec<String> {
    let mut scopes = maximum_application_scopes();
    scopes.push(cap::VERIFY.to_string());
    scopes
}

#[derive(CandidType)]
enum RootCommand {
    UpsertIssuerPolicy(RootIssuerPolicyUpsertRequest),
    UpsertIssuerRenewalTemplate(RootIssuerRenewalTemplateUpsertRequest),
}

#[derive(CandidType, Deserialize)]
enum RootCommandResponse {
    UpsertIssuerPolicy(RootIssuerPolicyResponse),
    UpsertIssuerRenewalTemplate(RootIssuerRenewalTemplateResponse),
}

#[derive(CandidType)]
enum RootStatusRequest {
    IssuerRenewal(RootIssuerRenewalStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum RootStatusResponse {
    IssuerRenewal(RootIssuerRenewalStatusResponse),
}

#[derive(CandidType)]
#[expect(
    clippy::large_enum_variant,
    reason = "the fixture mirrors the exact generated managed command Candid"
)]
enum CanisterCommand {
    ApplicationSession(ApplicationSessionCommand),
    PrepareDelegatedToken(DelegatedTokenPrepareRequest),
}

#[derive(CandidType, Deserialize)]
enum CanisterCommandResponse {
    ApplicationSession(ApplicationSessionCommandResponse),
    PrepareDelegatedToken(DelegatedTokenPrepareResponse),
}

#[derive(CandidType)]
enum CanisterStatusRequest {
    ApplicationSession,
    ApplicationSessionAudit(PageRequest),
    DelegatedToken(DelegatedTokenGetRequest),
    Metrics(MetricsStatusRequest),
}

#[derive(CandidType, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the fixture mirrors the exact generated managed status Candid"
)]
enum CanisterStatusResponse {
    ApplicationSession(ApplicationSessionStatus),
    ApplicationSessionAudit(ApplicationSessionAuditResponse),
    DelegatedToken(DelegatedToken),
    Metrics(Page<MetricEntry>),
}

#[derive(CandidType)]
enum RuntimeWhitelistManagedCommand {
    RuntimeWhitelist(RuntimeWhitelistCommand),
}

#[derive(CandidType, Deserialize)]
enum RuntimeWhitelistManagedCommandResponse {
    RuntimeWhitelist(RuntimeWhitelistMutationResponse),
}

#[derive(CandidType)]
enum RuntimeWhitelistManagedStatusRequest {
    RuntimeWhitelist(PageRequest),
}

#[derive(CandidType, Deserialize)]
enum RuntimeWhitelistManagedStatusResponse {
    RuntimeWhitelist(RuntimeWhitelistStatusResponse),
}

#[derive(CandidType, Clone, Copy)]
enum LocalAuthorizationDenialProbe {
    Anonymous,
    AuthorityUnavailable,
    CallerMismatch,
    Disabled,
    Expired,
    InadmissibleSubject,
    MissingScope,
    MissingSession,
    StaleAuthority,
}

impl LocalAuthorizationDenialProbe {
    const ALL: [Self; 9] = [
        Self::Anonymous,
        Self::CallerMismatch,
        Self::Disabled,
        Self::AuthorityUnavailable,
        Self::MissingSession,
        Self::Expired,
        Self::StaleAuthority,
        Self::InadmissibleSubject,
        Self::MissingScope,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Anonymous => "anonymous",
            Self::AuthorityUnavailable => "authority_unavailable",
            Self::CallerMismatch => "caller_mismatch",
            Self::Disabled => "disabled",
            Self::Expired => "expired",
            Self::InadmissibleSubject => "inadmissible_subject",
            Self::MissingScope => "missing_scope",
            Self::MissingSession => "missing_session",
            Self::StaleAuthority => "stale_authority",
        }
    }
}

#[derive(Serialize)]
struct ApplicationSessionRecordFixture {
    transport_caller: candid::Principal,
    authenticated_subject: candid::Principal,
    issuer: candid::Principal,
    fleet: FleetKey,
    role: CanisterRole,
    scopes: Vec<String>,
    authority_generation: u64,
    established_at_ns: u64,
    expires_at_ns: u64,
    proof_fingerprint: [u8; 32],
    establishment_request_hash: [u8; 32],
}

#[derive(Serialize)]
struct ApplicationReplayRecordFixture {
    proof_fingerprint: [u8; 32],
    transport_caller: candid::Principal,
    authenticated_subject: candid::Principal,
    authority_generation: u64,
    remove_at_ns: u64,
}

#[test]
fn pem_backed_native_agent_prepares_retrieves_and_presents_delegated_token() {
    let mut fixture = setup_fresh_active_component_registry();
    let gateway_url = fixture.start_http_gateway();
    configure_issuer(&fixture);
    provision_delegation_proof(&fixture);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("native-agent runtime must build");
    let (subject, view, establish_request) =
        runtime.block_on(run_native_agent_journey(&fixture, &gateway_url));
    drop(runtime);
    let audit = root_application_session_audit(&fixture);
    assert_eq!(audit.policy.fleet, managed_test_init_identity().fleet.fleet);
    assert_eq!(audit.policy.role, fixture.verifier.role);
    assert_eq!(audit.policy.authority_generation, 0);
    assert_eq!(audit.policy.allowed_scopes, maximum_application_scopes());
    assert_eq!(audit.sessions.total, 1);
    assert_eq!(audit.sessions.entries.len(), 1);
    assert_eq!(audit.sessions.entries[0].transport_caller, subject);
    assert_eq!(
        audit.sessions.entries[0].status,
        ApplicationSessionStatus::Active(view)
    );

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("native-agent cleanup runtime must build");
    runtime.block_on(clear_application_session_journey(
        &fixture,
        &gateway_url,
        establish_request,
    ));
    drop(runtime);
    drop(fixture);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one ordered PocketIC journey proves runtime-whitelist authority and recovery"
)]
fn runtime_whitelist_is_durable_bounded_and_separate_from_application_sessions() {
    let fixture = setup_fresh_active_component_registry();
    let target = fixture.verifier.canister_id;
    let root = fixture.root;
    let controller = Principal::from_slice(&[0x61; 29]);
    let unauthorized = Principal::from_slice(&[0x62; 29]);
    let seeded = Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai")
        .expect("frozen whitelist fixture principal");

    let initial = runtime_whitelist_status_as(&fixture, target, root).expect("initial Root status");
    assert_eq!(initial.principals.entries, vec![seeded]);
    assert_eq!(initial.principals.total, 1);
    assert_eq!(initial.revision, 0);
    assert_eq!(initial.maximum_principals, 256);
    assert_eq!(runtime_whitelist_probe_as(&fixture, target, seeded), Ok(()));
    assert!(
        generic_application_subject_as(&fixture, target, seeded).is_err(),
        "runtime whitelist membership must not create a 0.105 application session"
    );

    fixture
        .pic()
        .set_controllers(target, Some(root), vec![controller])
        .expect("move management authority to the independent controller fixture");
    runtime_whitelist_status_as(&fixture, target, root)
        .expect("stable Root binding must retain whitelist inspection authority");
    runtime_whitelist_status_as(&fixture, target, controller)
        .expect("current controller must have whitelist inspection authority");
    assert_eq!(
        runtime_whitelist_status_as(&fixture, target, unauthorized)
            .expect_err("unrelated caller must not inspect membership")
            .code(),
        canic::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
    );

    let remove = RuntimeWhitelistCommand::Remove(RuntimeWhitelistMutationRequest {
        principal: seeded,
        expected_revision: 0,
        operation_id: [0x71; 32],
    });
    let removed = runtime_whitelist_command_as(&fixture, target, root, remove.clone())
        .expect("stable Root removes the seeded principal");
    assert_eq!(removed.outcome, RuntimeWhitelistMutationOutcome::Removed);
    assert_eq!(removed.revision, 1);
    assert_eq!(
        runtime_whitelist_command_as(&fixture, target, root, remove.clone())
            .expect("response-loss retry returns the exact accepted result"),
        removed
    );
    let after_remove =
        runtime_whitelist_status_as(&fixture, target, root).expect("status after removal");
    assert!(after_remove.principals.entries.is_empty());
    assert_eq!(after_remove.principals.total, 0);
    assert!(runtime_whitelist_probe_as(&fixture, target, seeded).is_err());

    let conflicting_reuse = RuntimeWhitelistCommand::Add(RuntimeWhitelistMutationRequest {
        principal: seeded,
        expected_revision: 1,
        operation_id: [0x71; 32],
    });
    assert_eq!(
        runtime_whitelist_command_as(&fixture, target, root, conflicting_reuse)
            .expect_err("operation ID reuse for another request must reject")
            .code(),
        canic::diagnostics::codes::REQUEST_CONFLICT.raw_code()
    );
    let stale_revision = RuntimeWhitelistCommand::Add(RuntimeWhitelistMutationRequest {
        principal: seeded,
        expected_revision: 0,
        operation_id: [0x72; 32],
    });
    assert_eq!(
        runtime_whitelist_command_as(&fixture, target, root, stale_revision)
            .expect_err("stale revision must reject")
            .code(),
        canic::diagnostics::codes::VERSION_CONFLICT.raw_code()
    );
    assert_eq!(
        runtime_whitelist_status_as(&fixture, target, root)
            .expect("rejections leave state readable")
            .revision,
        1
    );

    fixture
        .pic()
        .set_controllers(target, Some(controller), vec![root])
        .expect("return management authority to the Fleet Root");
    fixture
        .pic()
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    fixture
        .pic()
        .upgrade_canister(target, fixture.verifier_wasm(), upgrade_args(), Some(root))
        .expect("same-release upgrade restores runtime-whitelist authority");

    let restored =
        runtime_whitelist_status_as(&fixture, target, root).expect("restored Root status");
    assert_eq!(restored.revision, 1);
    assert!(restored.principals.entries.is_empty());
    assert_eq!(
        runtime_whitelist_command_as(&fixture, target, root, remove)
            .expect("retained exact operation survives restoration"),
        removed
    );

    let added = runtime_whitelist_command_as(
        &fixture,
        target,
        root,
        RuntimeWhitelistCommand::Add(RuntimeWhitelistMutationRequest {
            principal: seeded,
            expected_revision: 1,
            operation_id: [0x73; 32],
        }),
    )
    .expect("Root re-adds the principal without rebuilding");
    assert_eq!(added.outcome, RuntimeWhitelistMutationOutcome::Added);
    assert_eq!(added.revision, 2);
    assert_eq!(runtime_whitelist_probe_as(&fixture, target, seeded), Ok(()));
    assert_eq!(root_application_session_audit(&fixture).sessions.total, 0);
    drop(fixture);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one ordered PocketIC journey proves the complete multi-target recovery boundary"
)]
fn multi_target_sessions_preserve_controller_separation_and_same_release_recovery() {
    let mut fixture = setup_fresh_active_component_registry();
    let gateway_url = fixture.start_http_gateway();
    configure_issuer(&fixture);
    provision_delegation_proof(&fixture);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("native-agent runtime must build");
    let identity = Secp256k1Identity::from_pem(TEST_IDENTITY_PEM.as_bytes())
        .expect("test PEM identity must parse");
    let subject = identity.sender().expect("test identity must have a sender");
    let agent = Agent::builder()
        .with_url(&gateway_url)
        .with_identity(identity)
        .build()
        .expect("native agent must build");
    let (request, issuer_view, verifier_view) = runtime.block_on(async {
        agent
            .fetch_root_key()
            .await
            .expect("PocketIC root key must be available");
        let request = ApplicationSessionRequest {
            delegated_token: prepare_native_delegated_token(&fixture, &agent, 44).await,
            requested_scopes: maximum_application_scopes(),
            requested_ttl_secs: Some(1_800),
        };
        let issuer_view =
            establish_application_session(&agent, fixture.issuer.canister_id, request.clone())
                .await;
        let verifier_view =
            establish_application_session(&agent, fixture.verifier.canister_id, request.clone())
                .await;
        assert_eq!(issuer_view.authenticated_subject, subject);
        assert_eq!(verifier_view.authenticated_subject, subject);
        for target in [fixture.issuer.canister_id, fixture.verifier.canister_id] {
            assert_eq!(
                generic_application_subject(&agent, target)
                    .await
                    .expect("each target-local session must authorize its generic consumer"),
                subject
            );
        }
        (request, issuer_view, verifier_view)
    });

    for target in [fixture.issuer.canister_id, fixture.verifier.canister_id] {
        let controller_result: Result<candid::Principal, Error> = fixture
            .pic()
            .query_candid_as_or_panic(target, fixture.root, "issuer_application_subject", ());
        assert!(
            controller_result.is_err(),
            "controllership must not imply local application authority"
        );
        let audit = root_application_session_audit_for(&fixture, target);
        assert_eq!(audit.sessions.total, 1);
        assert_eq!(audit.sessions.entries[0].transport_caller, subject);
    }

    fixture.pic().advance_time(Duration::from_secs(11));
    fixture.pic().tick();
    runtime.block_on(async {
        for (target, view) in [
            (fixture.issuer.canister_id, &issuer_view),
            (fixture.verifier.canister_id, &verifier_view),
        ] {
            assert_eq!(
                application_session_status(&agent, target).await,
                ApplicationSessionStatus::Active(view.clone()),
                "proof expiry must not expire the independently bounded session"
            );
            assert_eq!(
                establish_application_session(&agent, target, request.clone()).await,
                *view,
                "an exact active receipt retry must remain available after proof expiry"
            );
        }
    });

    fixture
        .pic()
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    let component_wasm = fixture.verifier_wasm();
    for target in [fixture.issuer.canister_id, fixture.verifier.canister_id] {
        fixture
            .pic()
            .upgrade_canister(
                target,
                component_wasm.clone(),
                upgrade_args(),
                Some(fixture.root),
            )
            .expect("same-release target upgrade must reconstruct local authorization");
    }
    for (target, view) in [
        (fixture.issuer.canister_id, &issuer_view),
        (fixture.verifier.canister_id, &verifier_view),
    ] {
        assert_eq!(
            generic_application_subject_as(&fixture, target, subject)
                .expect("reconstructed session must authorize its generic consumer"),
            subject
        );
        assert_eq!(
            application_session_status_as(&fixture, target, subject),
            ApplicationSessionStatus::Active(view.clone())
        );
        assert_eq!(
            establish_application_session_as(&fixture, target, subject, request.clone()),
            *view,
            "same-release reconstruction must retain the exact retry receipt"
        );
    }
    for target in [fixture.issuer.canister_id, fixture.verifier.canister_id] {
        assert_eq!(
            root_application_session_audit_for(&fixture, target)
                .sessions
                .total,
            1,
            "reconstruction must not duplicate target-local authority"
        );
    }

    assert_eq!(
        application_session_command_as(
            &fixture,
            fixture.verifier.canister_id,
            subject,
            ApplicationSessionCommand::Clear,
        )
        .expect("verifier-local clear must succeed"),
        ApplicationSessionCommandResponse::Cleared
    );
    assert!(
        generic_application_subject_as(&fixture, fixture.verifier.canister_id, subject).is_err(),
        "clearing one target must immediately remove its local authority"
    );
    assert_eq!(
        generic_application_subject_as(&fixture, fixture.issuer.canister_id, subject)
            .expect("clearing the verifier must not affect the issuer-local session"),
        subject
    );
    assert_eq!(
        root_application_session_audit_for(&fixture, fixture.verifier.canister_id)
            .sessions
            .total,
        0
    );

    let now_ns = fixture.pic().current_time_nanos();
    assert!(now_ns < issuer_view.expires_at_ns);
    fixture.pic().advance_time(
        Duration::from_nanos(issuer_view.expires_at_ns - now_ns) + Duration::from_secs(1),
    );
    fixture.pic().tick();
    assert!(
        generic_application_subject_as(&fixture, fixture.issuer.canister_id, subject).is_err(),
        "strict session expiry must deny the generic consumer"
    );
    let expired_status =
        application_session_status_as(&fixture, fixture.issuer.canister_id, subject);
    assert!(
        match expired_status {
            ApplicationSessionStatus::Inactive(InactiveApplicationSession::Expired {
                expired_at_ns,
            }) => expired_at_ns == issuer_view.expires_at_ns,
            ApplicationSessionStatus::Inactive(InactiveApplicationSession::Missing) => true,
            ApplicationSessionStatus::Active(_)
            | ApplicationSessionStatus::Inactive(
                InactiveApplicationSession::StaleFleet
                | InactiveApplicationSession::StaleRole
                | InactiveApplicationSession::StaleGeneration { .. }
                | InactiveApplicationSession::InadmissibleSubject,
            ) => false,
        },
        "the expired session must remain classified as expired or be removed by bounded cleanup"
    );
    assert_eq!(
        application_session_command_as(
            &fixture,
            fixture.issuer.canister_id,
            subject,
            ApplicationSessionCommand::Clear,
        )
        .expect("expired issuer-local session clear must succeed"),
        ApplicationSessionCommandResponse::Cleared
    );
    assert_eq!(
        root_application_session_audit_for(&fixture, fixture.issuer.canister_id)
            .sessions
            .total,
        0
    );
    for target in [fixture.issuer.canister_id, fixture.verifier.canister_id] {
        assert!(
            application_session_command_as(
                &fixture,
                target,
                subject,
                ApplicationSessionCommand::Establish(request.clone()),
            )
            .is_err(),
            "an expired proof must not recreate cleared target-local authority"
        );
    }
    for target in [fixture.issuer.canister_id, fixture.verifier.canister_id] {
        assert_eq!(
            root_application_session_audit_for(&fixture, target)
                .sessions
                .total,
            0
        );
    }
}

async fn establish_application_session(
    agent: &Agent,
    canister_id: candid::Principal,
    request: ApplicationSessionRequest,
) -> ApplicationSessionView {
    let response = application_session_command(
        agent,
        canister_id,
        ApplicationSessionCommand::Establish(request),
    )
    .await
    .expect("caller-bound application session must establish or retry exactly");
    let ApplicationSessionCommandResponse::Established(view) = response else {
        panic!("unexpected application-session response")
    };
    view
}

fn establish_application_session_as(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
    canister_id: candid::Principal,
    caller: candid::Principal,
    request: ApplicationSessionRequest,
) -> ApplicationSessionView {
    let response = application_session_command_as(
        fixture,
        canister_id,
        caller,
        ApplicationSessionCommand::Establish(request),
    )
    .expect("caller-bound application session must retry exactly");
    let ApplicationSessionCommandResponse::Established(view) = response else {
        panic!("unexpected application-session response")
    };
    view
}

fn application_session_command_as(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
    canister_id: candid::Principal,
    caller: candid::Principal,
    command: ApplicationSessionCommand,
) -> Result<ApplicationSessionCommandResponse, Error> {
    let response: Result<CanisterCommandResponse, Error> = fixture.pic().update_candid_as_or_panic(
        canister_id,
        caller,
        protocol::CANIC_COMMAND,
        (CanisterCommand::ApplicationSession(command),),
    );
    response.map(|response| match response {
        CanisterCommandResponse::ApplicationSession(response) => response,
        CanisterCommandResponse::PrepareDelegatedToken(_) => {
            panic!("unexpected delegated-token response")
        }
    })
}

fn application_session_status_as(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
    canister_id: candid::Principal,
    caller: candid::Principal,
) -> ApplicationSessionStatus {
    let response: Result<CanisterStatusResponse, Error> = fixture.pic().query_candid_as_or_panic(
        canister_id,
        caller,
        protocol::CANIC_STATUS,
        (CanisterStatusRequest::ApplicationSession,),
    );
    match response.expect("application-session status must succeed") {
        CanisterStatusResponse::ApplicationSession(status) => status,
        CanisterStatusResponse::ApplicationSessionAudit(_) => {
            panic!("unexpected application-session audit")
        }
        CanisterStatusResponse::DelegatedToken(_) => panic!("unexpected delegated-token status"),
        CanisterStatusResponse::Metrics(_) => panic!("unexpected metrics status"),
    }
}

fn generic_application_subject_as(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
    canister_id: candid::Principal,
    caller: candid::Principal,
) -> Result<candid::Principal, Error> {
    fixture
        .pic()
        .query_candid_as_or_panic(canister_id, caller, "issuer_application_subject", ())
}

fn runtime_whitelist_command_as(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
    canister_id: candid::Principal,
    caller: candid::Principal,
    command: RuntimeWhitelistCommand,
) -> Result<RuntimeWhitelistMutationResponse, Error> {
    let response: Result<RuntimeWhitelistManagedCommandResponse, Error> =
        fixture.pic().update_candid_as_or_panic(
            canister_id,
            caller,
            protocol::CANIC_COMMAND,
            (RuntimeWhitelistManagedCommand::RuntimeWhitelist(command),),
        );
    response.map(|response| match response {
        RuntimeWhitelistManagedCommandResponse::RuntimeWhitelist(response) => response,
    })
}

fn runtime_whitelist_status_as(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
    canister_id: candid::Principal,
    caller: candid::Principal,
) -> Result<RuntimeWhitelistStatusResponse, Error> {
    let response: Result<RuntimeWhitelistManagedStatusResponse, Error> =
        fixture.pic().query_candid_as_or_panic(
            canister_id,
            caller,
            protocol::CANIC_STATUS,
            (RuntimeWhitelistManagedStatusRequest::RuntimeWhitelist(
                PageRequest {
                    offset: 0,
                    limit: u64::MAX,
                },
            ),),
        );
    response.map(|response| match response {
        RuntimeWhitelistManagedStatusResponse::RuntimeWhitelist(response) => response,
    })
}

fn runtime_whitelist_probe_as(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
    canister_id: candid::Principal,
    caller: candid::Principal,
) -> Result<(), Error> {
    fixture.pic().query_candid_as_or_panic(
        canister_id,
        caller,
        "issuer_runtime_whitelist_probe",
        (),
    )
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one ordered PocketIC journey records the complete maximum-state resource boundary"
)]
fn maximum_application_session_resource_contract_is_bounded() {
    let fixture = setup_fresh_active_component_registry();
    let verifier_wasm = fixture.verifier_wasm();
    let verifier = fixture.verifier.canister_id;
    let subject = native_agent_subject();
    let now_ns = fixture.pic().current_time_nanos();

    let stable_bytes_before = fixture.pic().get_stable_memory(verifier).len();
    let auth_state_bytes = inject_application_authorization_state(
        fixture.pic(),
        verifier,
        fixture.issuer.canister_id,
        fixture.verifier.role.clone(),
        subject,
        now_ns,
        MAX_ACTIVE_APPLICATION_SESSIONS,
        MAX_APPLICATION_REPLAY_RECORDS,
    );
    assert!(auth_state_bytes <= MAX_APPLICATION_SESSION_STABLE_BYTES);
    let stable_bytes_at_maximum = fixture.pic().get_stable_memory(verifier).len();
    assert!(stable_bytes_at_maximum >= stable_bytes_before);

    fixture
        .pic()
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    fixture
        .pic()
        .upgrade_canister(verifier, verifier_wasm, upgrade_args(), Some(fixture.root))
        .expect("same-release verifier upgrade must restore maximum session state");

    let restored = root_application_session_audit(&fixture);
    assert_eq!(
        restored.sessions.total,
        MAX_ACTIVE_APPLICATION_SESSIONS as u64
    );
    assert_eq!(restored.policy.authority_generation, 0);
    assert_eq!(restored.policy.allowed_scopes, maximum_application_scopes());

    let runtime_after_restore = role_metrics(&fixture, MetricsKind::Runtime);
    let restore = count_and_instructions(
        &runtime_after_restore,
        &[
            "perf",
            "checkpoint",
            "canic_core::workflow::runtime",
            "application_session_restore",
        ],
    );
    assert_eq!(restore.0, 1, "maximum state must reconstruct exactly once");

    let authorized: Result<QueryPerfSample<Result<candid::Principal, Error>>, Error> = fixture
        .pic()
        .query_candid_as_or_panic(verifier, subject, "issuer_application_subject_perf", ());
    let authorized = authorized.expect("performance probe ingress must be admitted");
    assert_eq!(
        authorized
            .value
            .expect("maximum-state local authorization must succeed"),
        subject
    );
    assert!(
        authorized.local_instructions < MAX_LOCAL_AUTHORIZATION_INSTRUCTIONS,
        "maximum-state local authorization used {} instructions",
        authorized.local_instructions
    );

    fixture.pic().advance_time(Duration::from_secs(61));
    let mut cleanup_removed = 0;
    for _ in 0..10 {
        fixture.pic().tick();
        cleanup_removed = metric_count_or_default(
            &role_metrics(&fixture, MetricsKind::Security),
            &[
                "auth",
                "application_session",
                "cleanup",
                "completed",
                "expired",
            ],
        );
        if cleanup_removed > 0 {
            break;
        }
    }
    assert_eq!(cleanup_removed, MAX_APPLICATION_SESSION_CLEANUP_REMOVALS);
    let runtime_after_cleanup = role_metrics(&fixture, MetricsKind::Runtime);
    let cleanup = count_and_instructions(
        &runtime_after_cleanup,
        &[
            "perf",
            "checkpoint",
            "canic_core::workflow::runtime::intent",
            "application_session_cleanup",
        ],
    );
    assert_eq!(
        cleanup.0, 1,
        "one timer delivery must run one bounded batch"
    );
    assert_eq!(
        root_application_session_audit(&fixture).sessions.total,
        MAX_ACTIVE_APPLICATION_SESSIONS as u64,
        "replay cleanup must not remove unexpired sessions"
    );
    assert_eq!(
        fixture.pic().get_stable_memory(verifier).len(),
        stable_bytes_at_maximum,
        "bounded cleanup must not grow stable memory"
    );

    eprintln!(
        "0.105 B6 maximum resource observation: auth_state_bytes={auth_state_bytes} stable_memory_before={stable_bytes_before} stable_memory_at_maximum={stable_bytes_at_maximum} restore_instructions={} authorization_instructions={} cleanup_instructions={}",
        restore.1, authorized.local_instructions, cleanup.1,
    );
    drop(fixture);
}

#[test]
fn local_application_authorization_lookup_is_bounded_at_one_and_median_state() {
    let observations = [
        measure_application_authorization_at_state(1, 1),
        measure_application_authorization_at_state(
            MAX_ACTIVE_APPLICATION_SESSIONS / 2,
            MAX_APPLICATION_REPLAY_RECORDS / 2,
        ),
    ];
    eprintln!("0.105 B6 lookup observations: {observations:?}");
}

fn measure_application_authorization_at_state(
    session_count: usize,
    replay_count: usize,
) -> (usize, u64) {
    let fixture = setup_fresh_active_component_registry();
    let verifier = fixture.verifier.canister_id;
    let subject = native_agent_subject();
    let auth_state_bytes = inject_application_authorization_state(
        fixture.pic(),
        verifier,
        fixture.issuer.canister_id,
        fixture.verifier.role.clone(),
        subject,
        fixture.pic().current_time_nanos(),
        session_count,
        replay_count,
    );
    assert!(auth_state_bytes <= MAX_APPLICATION_SESSION_STABLE_BYTES);
    fixture
        .pic()
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    fixture
        .pic()
        .upgrade_canister(
            verifier,
            fixture.verifier_wasm(),
            upgrade_args(),
            Some(fixture.root),
        )
        .expect("same-release verifier upgrade must restore bounded session state");
    assert_eq!(
        root_application_session_audit(&fixture).sessions.total,
        session_count as u64
    );

    let authorized: Result<QueryPerfSample<Result<candid::Principal, Error>>, Error> = fixture
        .pic()
        .query_candid_as_or_panic(verifier, subject, "issuer_application_subject_perf", ());
    let authorized = authorized.expect("performance probe ingress must be admitted");
    assert_eq!(
        authorized
            .value
            .expect("bounded-state local authorization must succeed"),
        subject
    );
    assert!(
        authorized.local_instructions < MAX_LOCAL_AUTHORIZATION_INSTRUCTIONS,
        "{session_count}-session local authorization used {} instructions",
        authorized.local_instructions
    );
    let authorization_instructions = authorized.local_instructions;
    drop(fixture);
    (session_count, authorization_instructions)
}

#[test]
fn closed_local_authorization_denial_partition_is_bounded() {
    let fixture = setup_fresh_active_component_registry();
    let caller = native_agent_subject();
    let observations = LocalAuthorizationDenialProbe::ALL.map(|probe| {
        let sample: Result<QueryPerfSample<String>, Error> =
            fixture.pic().query_candid_as_or_panic(
                fixture.verifier.canister_id,
                caller,
                "issuer_application_denial_perf",
                (probe,),
            );
        let sample = sample.expect("denial performance probe must succeed");
        assert_eq!(sample.value, probe.label());
        assert!(
            sample.local_instructions < MAX_LOCAL_AUTHORIZATION_INSTRUCTIONS,
            "{} denial used {} instructions",
            probe.label(),
            sample.local_instructions
        );
        (probe.label(), sample.local_instructions)
    });
    eprintln!("0.105 B6 denial observations: {observations:?}");
}

async fn run_native_agent_journey(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
    gateway_url: &str,
) -> (
    candid::Principal,
    ApplicationSessionView,
    ApplicationSessionRequest,
) {
    let identity = Secp256k1Identity::from_pem(TEST_IDENTITY_PEM.as_bytes())
        .expect("test PEM identity must parse");
    let subject = identity.sender().expect("test identity must have a sender");
    let agent = Agent::builder()
        .with_url(gateway_url)
        .with_identity(identity)
        .build()
        .expect("native agent must build");
    agent
        .fetch_root_key()
        .await
        .expect("PocketIC root key must be available");

    let token = prepare_native_delegated_token(fixture, &agent, 42).await;
    assert_eq!(token.claims.presenter, subject);
    assert_eq!(token.claims.subject, subject);

    let presented_bytes = agent
        .update(&fixture.verifier.canister_id, "issuer_verify_token")
        .with_arg(Encode!(&token).expect("encode delegated-token presentation"))
        .call_and_wait()
        .await
        .expect("native delegated-token presentation ingress must succeed");
    let presented: Result<(), Error> = Decode!(&presented_bytes, Result<(), Error>)
        .expect("decode delegated-token presentation response");
    presented.expect("the same authenticated native agent must pass verification");

    let (first_view, _, warm_establishment_instructions) =
        exercise_application_session(fixture, &agent, subject, token).await;
    assert_eq!(first_view.authenticated_subject, subject);

    let cold_token = prepare_native_delegated_token(fixture, &agent, 43).await;
    let cold_request = ApplicationSessionRequest {
        delegated_token: cold_token,
        requested_scopes: maximum_application_scopes(),
        requested_ttl_secs: Some(1_799),
    };
    let before_cold = native_runtime_metrics(&agent, fixture.verifier.canister_id).await;
    let cold_established = application_session_command(
        &agent,
        fixture.verifier.canister_id,
        ApplicationSessionCommand::Establish(cold_request.clone()),
    )
    .await
    .expect("cold-proof application session must replace the first session");
    let after_cold = native_runtime_metrics(&agent, fixture.verifier.canister_id).await;
    let ApplicationSessionCommandResponse::Established(cold_view) = cold_established else {
        panic!("unexpected cold application-session response")
    };
    let cold_establishment = metric_delta(
        &before_cold,
        &after_cold,
        &["perf", "endpoint", "update", protocol::CANIC_COMMAND],
    );
    assert_eq!(cold_establishment.0, 1);
    let cold_verification = metric_delta(
        &before_cold,
        &after_cold,
        &[
            "perf",
            "checkpoint",
            "canic_core::ops::auth::token",
            "delegated_token_verify_embedded_proofs",
        ],
    );
    assert_eq!(cold_verification.0, 1);
    eprintln!(
        "0.105 B6 establishment observation: warm_instructions={warm_establishment_instructions} cold_instructions={} cold_embedded_proof_instructions={}",
        cold_establishment.1, cold_verification.1,
    );

    (subject, cold_view, cold_request)
}

async fn prepare_native_delegated_token(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
    agent: &Agent,
    request_id: u8,
) -> DelegatedToken {
    let prepare_request = DelegatedTokenPrepareRequest {
        metadata: Some(AuthRequestMetadata {
            request_id: [request_id; 32],
            ttl_ns: 60_000_000_000,
        }),
        aud: DelegationAudience::Fleet(managed_test_init_identity().fleet.fleet),
        grants: vec![role_grant(
            fixture.verifier.role.clone(),
            delegated_grant_scopes(),
        )],
        ttl_ns: 10_000_000_000,
        ext: None,
    };
    let prepared_bytes = agent
        .update(&fixture.issuer.canister_id, protocol::CANIC_COMMAND)
        .with_arg(
            Encode!(&CanisterCommand::PrepareDelegatedToken(prepare_request))
                .expect("encode delegated-token prepare command"),
        )
        .call_and_wait()
        .await
        .expect("native delegated-token prepare ingress must succeed");
    let prepared: Result<CanisterCommandResponse, Error> =
        Decode!(&prepared_bytes, Result<CanisterCommandResponse, Error>)
            .expect("decode delegated-token prepare response");
    let CanisterCommandResponse::PrepareDelegatedToken(prepared) =
        prepared.expect("delegated-token preparation must succeed")
    else {
        panic!("unexpected application-session response")
    };

    let token_bytes = agent
        .query(&fixture.issuer.canister_id, protocol::CANIC_STATUS)
        .with_arg(
            Encode!(&CanisterStatusRequest::DelegatedToken(
                DelegatedTokenGetRequest {
                    claims_hash: prepared.claims_hash,
                }
            ))
            .expect("encode delegated-token retrieval status"),
        )
        .call()
        .await
        .expect("native delegated-token retrieval ingress must succeed");
    let token: Result<CanisterStatusResponse, Error> =
        Decode!(&token_bytes, Result<CanisterStatusResponse, Error>)
            .expect("decode delegated-token retrieval response");
    let CanisterStatusResponse::DelegatedToken(token) =
        token.expect("delegated-token retrieval must succeed")
    else {
        panic!("unexpected application-session status")
    };
    token
}

async fn exercise_application_session(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
    agent: &Agent,
    subject: candid::Principal,
    token: DelegatedToken,
) -> (ApplicationSessionView, ApplicationSessionRequest, u64) {
    assert!(
        generic_application_subject(agent, fixture.verifier.canister_id)
            .await
            .is_err(),
        "proof verification alone must not authorize the tokenless application endpoint"
    );
    let establish_request = ApplicationSessionRequest {
        delegated_token: token,
        requested_scopes: maximum_application_scopes(),
        requested_ttl_secs: Some(1_800),
    };
    let before_warm = native_runtime_metrics(agent, fixture.verifier.canister_id).await;
    let established = application_session_command(
        agent,
        fixture.verifier.canister_id,
        ApplicationSessionCommand::Establish(establish_request.clone()),
    )
    .await
    .expect("caller-bound application session must establish");
    let after_warm = native_runtime_metrics(agent, fixture.verifier.canister_id).await;
    let ApplicationSessionCommandResponse::Established(first_view) = established else {
        panic!("unexpected application-session response")
    };
    let warm_establishment = metric_delta(
        &before_warm,
        &after_warm,
        &["perf", "endpoint", "update", protocol::CANIC_COMMAND],
    );
    assert_eq!(warm_establishment.0, 1);
    let warm_verification = metric_delta(
        &before_warm,
        &after_warm,
        &[
            "perf",
            "checkpoint",
            "canic_core::ops::auth::token",
            "delegated_token_verify_cached",
        ],
    );
    assert_eq!(warm_verification.0, 1);
    assert_eq!(first_view.authenticated_subject, subject);
    assert_eq!(first_view.scopes, maximum_application_scopes());
    assert_eq!(
        first_view.expires_at_ns - first_view.established_at_ns,
        1_800_000_000_000
    );
    assert_eq!(
        generic_application_subject(agent, fixture.verifier.canister_id)
            .await
            .expect("the established session must authorize the generic consumer"),
        subject
    );

    let status = application_session_status(agent, fixture.verifier.canister_id).await;
    assert_eq!(status, ApplicationSessionStatus::Active(first_view.clone()));
    assert!(
        application_session_audit(agent, fixture.verifier.canister_id)
            .await
            .is_err(),
        "the application caller must not inherit Fleet operator inspection authority"
    );
    let retried = application_session_command(
        agent,
        fixture.verifier.canister_id,
        ApplicationSessionCommand::Establish(establish_request.clone()),
    )
    .await
    .expect("exact receipt retry must return the current session");
    assert_eq!(
        retried,
        ApplicationSessionCommandResponse::Established(first_view.clone())
    );

    (first_view, establish_request, warm_establishment.1)
}

async fn clear_application_session_journey(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
    gateway_url: &str,
    establish_request: ApplicationSessionRequest,
) {
    let identity = Secp256k1Identity::from_pem(TEST_IDENTITY_PEM.as_bytes())
        .expect("test PEM identity must parse");
    let agent = Agent::builder()
        .with_url(gateway_url)
        .with_identity(identity)
        .build()
        .expect("native agent must build");
    agent
        .fetch_root_key()
        .await
        .expect("PocketIC root key must be available");

    assert_eq!(
        application_session_command(
            &agent,
            fixture.verifier.canister_id,
            ApplicationSessionCommand::Clear,
        )
        .await
        .expect("caller-scoped clear must succeed"),
        ApplicationSessionCommandResponse::Cleared
    );
    assert_eq!(
        application_session_status(&agent, fixture.verifier.canister_id).await,
        ApplicationSessionStatus::Inactive(InactiveApplicationSession::Missing)
    );
    assert!(
        generic_application_subject(&agent, fixture.verifier.canister_id)
            .await
            .is_err(),
        "clear must immediately deny the generic consumer"
    );
    assert!(
        application_session_command(
            &agent,
            fixture.verifier.canister_id,
            ApplicationSessionCommand::Establish(establish_request),
        )
        .await
        .is_err(),
        "cleared proof tombstone must prevent resurrection"
    );
}

async fn generic_application_subject(
    agent: &Agent,
    canister_id: candid::Principal,
) -> Result<candid::Principal, Error> {
    let bytes = agent
        .query(&canister_id, "issuer_application_subject")
        .with_arg(Encode!().expect("encode empty application guard request"))
        .call()
        .await
        .expect("generic application guard ingress must complete");
    Decode!(&bytes, Result<candid::Principal, Error>)
        .expect("decode generic application guard response")
}

async fn native_runtime_metrics(agent: &Agent, canister_id: candid::Principal) -> Vec<MetricEntry> {
    let bytes = agent
        .query(&canister_id, "issuer_runtime_metrics")
        .with_arg(Encode!().expect("encode empty runtime metrics request"))
        .call()
        .await
        .expect("runtime metrics fixture ingress must complete");
    Decode!(&bytes, Result<Page<MetricEntry>, Error>)
        .expect("decode runtime metrics fixture response")
        .expect("runtime metrics fixture query must succeed")
        .entries
}

async fn application_session_command(
    agent: &Agent,
    canister_id: candid::Principal,
    command: ApplicationSessionCommand,
) -> Result<ApplicationSessionCommandResponse, Error> {
    let bytes = agent
        .update(&canister_id, protocol::CANIC_COMMAND)
        .with_arg(
            Encode!(&CanisterCommand::ApplicationSession(command))
                .expect("encode application-session command"),
        )
        .call_and_wait()
        .await
        .expect("application-session command ingress must complete");
    let response: Result<CanisterCommandResponse, Error> =
        Decode!(&bytes, Result<CanisterCommandResponse, Error>)
            .expect("decode application-session command response");
    response.map(|response| match response {
        CanisterCommandResponse::ApplicationSession(response) => response,
        CanisterCommandResponse::PrepareDelegatedToken(_) => {
            panic!("unexpected delegated-token response")
        }
    })
}

async fn application_session_status(
    agent: &Agent,
    canister_id: candid::Principal,
) -> ApplicationSessionStatus {
    let bytes = agent
        .query(&canister_id, protocol::CANIC_STATUS)
        .with_arg(
            Encode!(&CanisterStatusRequest::ApplicationSession)
                .expect("encode application-session status"),
        )
        .call()
        .await
        .expect("application-session status ingress must succeed");
    let response: Result<CanisterStatusResponse, Error> =
        Decode!(&bytes, Result<CanisterStatusResponse, Error>)
            .expect("decode application-session status response");
    match response.expect("application-session status must succeed") {
        CanisterStatusResponse::ApplicationSession(status) => status,
        CanisterStatusResponse::ApplicationSessionAudit(_) => {
            panic!("unexpected application-session audit")
        }
        CanisterStatusResponse::DelegatedToken(_) => panic!("unexpected delegated-token status"),
        CanisterStatusResponse::Metrics(_) => panic!("unexpected metrics status"),
    }
}

async fn application_session_audit(
    agent: &Agent,
    canister_id: candid::Principal,
) -> Result<ApplicationSessionAuditResponse, Error> {
    let bytes = agent
        .query(&canister_id, protocol::CANIC_STATUS)
        .with_arg(
            Encode!(&CanisterStatusRequest::ApplicationSessionAudit(
                PageRequest {
                    offset: 0,
                    limit: 1,
                },
            ))
            .expect("encode application-session audit"),
        )
        .call()
        .await
        .expect("application-session audit ingress must complete");
    let response: Result<CanisterStatusResponse, Error> =
        Decode!(&bytes, Result<CanisterStatusResponse, Error>)
            .expect("decode application-session audit response");
    response.map(|response| match response {
        CanisterStatusResponse::ApplicationSessionAudit(audit) => audit,
        CanisterStatusResponse::ApplicationSession(_)
        | CanisterStatusResponse::DelegatedToken(_)
        | CanisterStatusResponse::Metrics(_) => {
            panic!("unexpected application-session audit response")
        }
    })
}

fn root_application_session_audit(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
) -> ApplicationSessionAuditResponse {
    root_application_session_audit_for(fixture, fixture.verifier.canister_id)
}

fn root_application_session_audit_for(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
    canister_id: candid::Principal,
) -> ApplicationSessionAuditResponse {
    let response: Result<CanisterStatusResponse, Error> = fixture.pic().query_candid_as_or_panic(
        canister_id,
        fixture.root,
        protocol::CANIC_STATUS,
        (CanisterStatusRequest::ApplicationSessionAudit(
            PageRequest {
                offset: 0,
                limit: 1,
            },
        ),),
    );
    match response.expect("Root-authorized application-session audit must succeed") {
        CanisterStatusResponse::ApplicationSessionAudit(audit) => audit,
        CanisterStatusResponse::ApplicationSession(_)
        | CanisterStatusResponse::DelegatedToken(_)
        | CanisterStatusResponse::Metrics(_) => {
            panic!("unexpected Root application-session audit response")
        }
    }
}

fn native_agent_subject() -> candid::Principal {
    Secp256k1Identity::from_pem(TEST_IDENTITY_PEM.as_bytes())
        .expect("test PEM identity must parse")
        .sender()
        .expect("test identity must have a sender")
}

#[expect(
    clippy::too_many_arguments,
    reason = "the fixture injector names every exact canonical state and authority input"
)]
fn inject_application_authorization_state(
    pic: &pocket_ic::PocketIc,
    canister_id: candid::Principal,
    issuer: candid::Principal,
    role: CanisterRole,
    native_subject: candid::Principal,
    now_ns: u64,
    session_count: usize,
    replay_count: usize,
) -> usize {
    let fleet = managed_test_init_identity().fleet.fleet;
    let session_expiry_ns = now_ns
        .checked_add(1_800_000_000_000)
        .expect("session expiry must fit");
    let replay_expiry_ns = now_ns
        .checked_add(60_000_000_000)
        .expect("replay expiry must fit");
    let sessions = (0..session_count)
        .map(|index| {
            let subject = if index == 0 {
                native_subject
            } else {
                fixture_principal(index as u64)
            };
            ApplicationSessionRecordFixture {
                transport_caller: subject,
                authenticated_subject: subject,
                issuer,
                fleet,
                role: role.clone(),
                scopes: maximum_application_scopes(),
                authority_generation: 0,
                established_at_ns: now_ns,
                expires_at_ns: session_expiry_ns,
                proof_fingerprint: fixture_hash(index as u64),
                establishment_request_hash: fixture_hash((replay_count + index) as u64),
            }
        })
        .collect::<Vec<_>>();
    let replays = (0..replay_count)
        .map(|index| {
            let subject = if index == 0 {
                native_subject
            } else {
                fixture_principal(index as u64)
            };
            ApplicationReplayRecordFixture {
                proof_fingerprint: fixture_hash(index as u64),
                transport_caller: subject,
                authenticated_subject: subject,
                authority_generation: 0,
                remove_at_ns: replay_expiry_ns,
            }
        })
        .collect::<Vec<_>>();

    let stable_memory: VectorMemory = Rc::new(RefCell::new(pic.get_stable_memory(canister_id)));
    let manager = MemoryManager::init(stable_memory.clone());
    let auth_memory = manager.get(MemoryId::new(AUTH_STATE_MEMORY_ID));
    let mut auth_cell = Cell::<Vec<u8>, _>::init(auth_memory, Vec::new());
    let mut auth_state: ciborium::Value =
        ciborium::from_reader(auth_cell.get().as_slice()).expect("decode current auth-state CBOR");
    replace_cbor_field(
        &mut auth_state,
        "application_sessions",
        ciborium::Value::serialized(&sessions).expect("encode maximum application sessions"),
    );
    replace_cbor_field(
        &mut auth_state,
        "application_replays",
        ciborium::Value::serialized(&replays).expect("encode maximum application replays"),
    );
    replace_cbor_field(
        &mut auth_state,
        "application_authority_generation",
        ciborium::Value::Integer(0.into()),
    );
    replace_cbor_field(
        &mut auth_state,
        "application_authority_binding",
        ciborium::Value::Null,
    );
    let mut auth_state_bytes = Vec::new();
    ciborium::into_writer(&auth_state, &mut auth_state_bytes)
        .expect("encode maximum auth-state CBOR");
    auth_cell.set(auth_state_bytes.clone());
    drop(auth_cell);
    drop(manager);
    let stable_memory = Rc::try_unwrap(stable_memory)
        .expect("stable-memory editor must own the final reference")
        .into_inner();
    pic.set_stable_memory(canister_id, stable_memory, BlobCompression::NoCompression);
    auth_state_bytes.len()
}

fn replace_cbor_field(state: &mut ciborium::Value, field: &str, replacement: ciborium::Value) {
    let ciborium::Value::Map(entries) = state else {
        panic!("auth-state CBOR must be a map")
    };
    let Some((_, value)) = entries
        .iter_mut()
        .find(|(key, _)| key.as_text() == Some(field))
    else {
        panic!("auth-state CBOR is missing {field}")
    };
    *value = replacement;
}

fn fixture_principal(id: u64) -> candid::Principal {
    let mut bytes = [0_u8; 29];
    bytes[..8].copy_from_slice(&id.to_be_bytes());
    bytes[28] = 2;
    candid::Principal::from_slice(&bytes)
}

fn fixture_hash(id: u64) -> [u8; 32] {
    let mut bytes = [0_u8; 32];
    bytes[..8].copy_from_slice(&id.to_be_bytes());
    bytes
}

fn role_metrics(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
    kind: MetricsKind,
) -> Vec<MetricEntry> {
    let response: Result<CanisterStatusResponse, Error> = fixture.pic().query_candid_as_or_panic(
        fixture.verifier.canister_id,
        fixture.root,
        protocol::CANIC_STATUS,
        (CanisterStatusRequest::Metrics(MetricsStatusRequest {
            kind,
            page: PageRequest {
                offset: 0,
                limit: 1_000,
            },
        }),),
    );
    match response.expect("Root-authorized metrics status must succeed") {
        CanisterStatusResponse::Metrics(page) => page.entries,
        CanisterStatusResponse::ApplicationSession(_)
        | CanisterStatusResponse::ApplicationSessionAudit(_)
        | CanisterStatusResponse::DelegatedToken(_) => panic!("unexpected metrics response"),
    }
}

fn count_and_instructions(entries: &[MetricEntry], labels: &[&str]) -> (u64, u64) {
    count_and_instructions_or_default(entries, labels)
        .unwrap_or_else(|| panic!("missing instruction metric {labels:?}"))
}

fn count_and_instructions_or_default(
    entries: &[MetricEntry],
    labels: &[&str],
) -> Option<(u64, u64)> {
    entries.iter().find_map(|entry| {
        (entry
            .labels
            .iter()
            .map(String::as_str)
            .eq(labels.iter().copied()))
        .then(|| match entry.value {
            MetricValue::CountAndU64 { count, value_u64 } => (count, value_u64),
            MetricValue::Count(_) | MetricValue::U128(_) => {
                panic!("instruction metric must carry count and instructions")
            }
        })
    })
}

fn metric_delta(before: &[MetricEntry], after: &[MetricEntry], labels: &[&str]) -> (u64, u64) {
    let before = count_and_instructions_or_default(before, labels).unwrap_or_default();
    let after = count_and_instructions(after, labels);
    (
        after.0.saturating_sub(before.0),
        after.1.saturating_sub(before.1),
    )
}

fn metric_count_or_default(entries: &[MetricEntry], labels: &[&str]) -> u64 {
    entries
        .iter()
        .find_map(|entry| {
            (entry
                .labels
                .iter()
                .map(String::as_str)
                .eq(labels.iter().copied()))
            .then(|| match entry.value {
                MetricValue::Count(count) => count,
                MetricValue::CountAndU64 { .. } | MetricValue::U128(_) => {
                    panic!("counter metric must carry count only")
                }
            })
        })
        .unwrap_or_default()
}

fn configure_issuer(fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture) {
    let audience = DelegationAudience::Fleet(managed_test_init_identity().fleet.fleet);
    let grants = vec![role_grant(
        fixture.verifier.role.clone(),
        delegated_grant_scopes(),
    )];
    let policy: Result<RootCommandResponse, Error> = fixture.pic().update_candid_or_panic(
        fixture.root,
        protocol::CANIC_COMMAND,
        (RootCommand::UpsertIssuerPolicy(
            RootIssuerPolicyUpsertRequest {
                issuer_pid: fixture.issuer.canister_id,
                enabled: true,
                allowed_audiences: vec![audience.clone()],
                allowed_grants: grants.clone(),
                max_cert_ttl_ns: 60_000_000_000,
                refresh_after_ratio_bps: 8_000,
            },
        ),),
    );
    let RootCommandResponse::UpsertIssuerPolicy(policy) =
        policy.expect("root issuer policy must be accepted")
    else {
        panic!("unexpected Root command response")
    };
    assert_eq!(policy.issuer.issuer_pid, fixture.issuer.canister_id);

    let template: Result<RootCommandResponse, Error> = fixture.pic().update_candid_or_panic(
        fixture.root,
        protocol::CANIC_COMMAND,
        (RootCommand::UpsertIssuerRenewalTemplate(
            RootIssuerRenewalTemplateUpsertRequest {
                issuer_pid: fixture.issuer.canister_id,
                enabled: true,
                aud: audience,
                grants,
                cert_ttl_ns: 60_000_000_000,
            },
        ),),
    );
    let RootCommandResponse::UpsertIssuerRenewalTemplate(template) =
        template.expect("root issuer renewal template must be accepted")
    else {
        panic!("unexpected Root command response")
    };
    assert_eq!(template.template.issuer_pid, fixture.issuer.canister_id);
}

fn provision_delegation_proof(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
) {
    for _ in 0..ROOT_PROOF_PROVISION_ATTEMPTS {
        let provisioned: Result<(), Error> = fixture.pic().update_candid_or_panic(
            fixture.root,
            "test_provision_chain_key_delegation_proof_for_issuer",
            (fixture.issuer.canister_id,),
        );
        match provisioned {
            Ok(()) => return,
            Err(err)
                if err.code() == canic::diagnostics::codes::SECURITY_UNAVAILABLE.raw_code() =>
            {
                fixture.pic().tick();
                if root_issuer_proof_is_installed(fixture) {
                    return;
                }
            }
            Err(err) => {
                report_proof_provisioning_diagnostics(fixture);
                panic!("root proof provisioning returned unexpected error: {err:?}");
            }
        }
    }

    report_proof_provisioning_diagnostics(fixture);
    panic!(
        "root proof provisioning remained unavailable after {ROOT_PROOF_PROVISION_ATTEMPTS} attempts"
    );
}

fn root_issuer_proof_is_installed(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
) -> bool {
    let status: Result<RootStatusResponse, Error> = fixture.pic().query_candid_or_panic(
        fixture.root,
        protocol::CANIC_STATUS,
        (RootStatusRequest::IssuerRenewal(
            RootIssuerRenewalStatusRequest {
                issuer_pid: fixture.issuer.canister_id,
            },
        ),),
    );
    let RootStatusResponse::IssuerRenewal(status) =
        status.expect("root issuer renewal status must be available");

    status.latest_batch.is_some_and(|batch| {
        batch.status == RootIssuerRenewalBatchStatus::Installed && batch.installed_at_ns.is_some()
    })
}

fn report_proof_provisioning_diagnostics(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
) {
    report_canister_diagnostics(
        fixture.pic(),
        fixture.root,
        candid::Principal::anonymous(),
        "native-agent root proof provisioning",
    );
    report_canister_diagnostics(
        fixture.pic(),
        fixture.issuer.canister_id,
        fixture.root,
        "native-agent issuer proof provisioning",
    );
}
