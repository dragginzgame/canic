mod browser_transport;
mod issuer_bounds;
mod issuer_capacity;
mod issuer_permission;
mod issuer_restore;
mod issuer_retention;
mod operator_inspection;

use canic::{
    Error,
    dto::{
        fleet_admission::{FleetAdmissionProjectionPhase, FleetAdmissionProjectionStatusResponse},
        metrics::QueryPerfSample,
        page::PageRequest,
    },
};

// This corruption fixture deliberately edits raw stored bytes outside runtime custody.
use canic_tests::pic::{
    ActiveComponentRegistryFixture, setup_active_component_registry,
    setup_fresh_active_component_registry, upgrade_args,
};
use ic_platform::{
    agent::{Identity, Secp256k1Identity},
    principal::ExternalPrincipal as Principal,
};
use ic_storage::{VectorMemory, adapter::StableCell as Cell};
use ic_testkit::pic::{CanisterInstallExt, RetryPolicy};
use std::{cell::RefCell, rc::Rc, time::Duration};
use {ic_testkit::pocket_ic::PocketIc, ic_testkit::pocket_ic::common::rest::BlobCompression};

const MAX_LOCAL_AUTHORIZATION_INSTRUCTIONS: u64 = 1_000_000;
const INSTALL_CODE_RETRY_LIMIT: usize = 4;
const INSTALL_CODE_COOLDOWN: Duration = Duration::from_mins(5);

// Deterministic test-only identity; it has no authority outside this fresh PocketIC runtime.
const TEST_IDENTITY_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIGEAgEAMBAGByqGSM49AgEGBSuBBAAKBG0wawIBAQQgCDLudkRxUeRDhnUp2pvL
xLDICLIoNCa1sQdMgz5Y14GhRANCAASA7zusnWjPN0y8nJlD4YAEOpTEYu+CcCdO
VwidXc26G4+/g7dUbMwbN4E3d3bpxHEP31M+2by6jY67MqFKKroR
-----END PRIVATE KEY-----";

#[derive(Clone, Copy, serde::Serialize)]
enum LocalAuthorizationDenialProbe {
    Anonymous,
    AuthorityUnavailable,
    CallerMismatch,
}

impl LocalAuthorizationDenialProbe {
    const ALL: [Self; 3] = [
        Self::Anonymous,
        Self::CallerMismatch,
        Self::AuthorityUnavailable,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Anonymous => "anonymous",
            Self::AuthorityUnavailable => "authority_unavailable",
            Self::CallerMismatch => "caller_mismatch",
        }
    }
}

// Acquire the shared snapshot baseline only for cases whose mutations remain
// inside its captured Coordinator, Root, Store, issuer and verifier set. These
// cases create no gateway, canister or external resource; the recipe restores
// canister state and cycles, resets unused pool assets, preserves monotonic
// simulator time and revalidates the active authority graph before reuse.
fn setup_snapshot_resettable_active_component_registry() -> ActiveComponentRegistryFixture {
    setup_active_component_registry()
}

fn retry_same_release_upgrade(
    pic: &PocketIc,
    canister_id: Principal,
    wasm: Vec<u8>,
    sender: Principal,
) {
    let args = upgrade_args();
    let policy = RetryPolicy {
        max_attempts: std::num::NonZeroUsize::new(INSTALL_CODE_RETRY_LIMIT)
            .expect("non-zero install attempt limit"),
        cooldown: INSTALL_CODE_COOLDOWN,
    };
    pic.retry_install_code(policy, || {
        pic.upgrade_canister(canister_id, wasm.clone(), args.clone(), Some(sender))
    })
    .expect("same-release upgrade must succeed after bounded rate-limit retries");
}

#[test]
fn selected_issuer_corruption_rejects_same_release_restore() {
    let fixture = setup_fresh_active_component_registry();
    let issuer = fixture.issuer.canister_id;
    let stable_memory: VectorMemory = Rc::new(RefCell::new(
        fixture
            .pic()
            .get_stable_memory(issuer)
            .expect("PocketIC get_stable_memory"),
    ));
    let key = "canic.core.auth.delegated_token_issuer.state.v1";
    let mut runtime = ic_storage::MemoryRuntime::new(stable_memory.clone()).unwrap();
    let source = ic_storage::SealedDeclarationSnapshot::new(vec![
        ic_storage::MemoryRequest::new("canic-core", key).unwrap(),
    ])
    .unwrap();
    let grants = canic::memory::grants::framework_grants(Vec::new()).unwrap();
    runtime
        .bootstrap(&source, &grants, &ic_storage::GenericAllocationPolicy)
        .unwrap();
    let allocation = runtime
        .open_memory(&ic_storage::StableKey::parse(key).unwrap())
        .unwrap();
    let mut issuer_cell = Cell::<Vec<u8>, _>::load(
        allocation,
        canic_contracts::serialization::STORAGE_LIMITS.bytes,
    )
    .unwrap();
    issuer_cell.set(vec![0xff]).unwrap();
    drop(issuer_cell);
    drop(runtime);
    let bytes = Rc::try_unwrap(stable_memory)
        .expect("exclusive stable-memory editor")
        .into_inner();
    fixture
        .pic()
        .set_stable_memory(issuer, bytes, BlobCompression::NoCompression)
        .expect("PocketIC set_stable_memory");
    let error = fixture
        .pic()
        .upgrade_canister(
            issuer,
            fixture.issuer_wasm(),
            upgrade_args(),
            Some(fixture.root),
        )
        .expect_err("selected issuer corruption must fail synchronous restoration");
    let ic_testkit::pocket_ic::ClientError::Canister(error) = error else {
        panic!("expected a canister rejection");
    };
    drop(fixture);
    assert_eq!(
        error.error_code,
        ic_testkit::pocket_ic::ErrorCode::CanisterCalledTrap
    );
}

#[test]
fn fleet_admission_projection_is_durable_bounded_and_separate_from_application_sessions() {
    let fixture = setup_snapshot_resettable_active_component_registry();
    let target = fixture.verifier.canister_id;
    let root = fixture.root;
    let controller = Principal::from_slice(&[0x61; 29]);
    let unauthorized = Principal::from_slice(&[0x62; 29]);
    let admitted = Principal::from_slice(&[1; 29]);

    let initial = fleet_admission_status_as(&fixture, target, root).expect("initial Root status");
    assert_eq!(initial.principals.entries, vec![admitted]);
    assert_eq!(initial.principals.total, 1);
    assert_eq!(initial.generation, 1);
    assert_eq!(initial.phase, FleetAdmissionProjectionPhase::Open);
    assert_eq!(initial.maximum_page_size, 128);
    assert_eq!(
        initial.target,
        canic::ids::ManagedCanisterBinding::Component(fixture.verifier.clone())
    );
    assert_eq!(fleet_admission_probe_as(&fixture, target, admitted), Ok(()));
    assert!(
        generic_application_subject_as(&fixture, target, admitted).is_err(),
        "Fleet admission must not create a application session"
    );

    fixture
        .pic()
        .set_controllers(target, Some(root), vec![controller])
        .expect("move management authority to the independent controller fixture");
    fleet_admission_status_as(&fixture, target, root)
        .expect("stable Root binding must retain projection inspection authority");
    fleet_admission_status_as(&fixture, target, controller)
        .expect("current controller must have projection inspection authority");
    assert_eq!(
        fleet_admission_status_as(&fixture, target, unauthorized)
            .expect_err("unrelated caller must not inspect projection")
            .code(),
        canic::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
    );

    fixture
        .pic()
        .set_controllers(target, Some(controller), vec![root])
        .expect("return management authority to the Fleet Root");
    retry_same_release_upgrade(fixture.pic(), target, fixture.verifier_wasm(), root);

    let restored = fleet_admission_status_as(&fixture, target, root).expect("restored Root status");
    assert_eq!(restored.generation, initial.generation);
    assert_eq!(restored.policy_digest, initial.policy_digest);
    assert_eq!(restored.projection_digest, initial.projection_digest);
    assert_eq!(restored.principals.entries, vec![admitted]);
    assert_eq!(restored.phase, FleetAdmissionProjectionPhase::Open);
    assert_eq!(fleet_admission_probe_as(&fixture, target, admitted), Ok(()));
    drop(fixture);
}

fn generic_application_subject_as(
    fixture: &canic_tests::pic::ActiveComponentRegistryFixture,
    canister_id: ic_platform::principal::ExternalPrincipal,
    caller: ic_platform::principal::ExternalPrincipal,
) -> Result<ic_platform::principal::ExternalPrincipal, Error> {
    ic_testkit::pic::query_wire::<_, Result<ic_platform::Principal, Error>>(
        fixture.pic(),
        canister_id,
        caller,
        canic_tests::pic::issuer_probe::SUBJECT,
        &(),
    )
    .map(|reply| reply.map(Into::into))
    .expect("query Candid harness call")
}

fn fleet_admission_status_as(
    fixture: &canic_tests::pic::ActiveComponentRegistryFixture,
    canister_id: ic_platform::principal::ExternalPrincipal,
    caller: ic_platform::principal::ExternalPrincipal,
) -> Result<FleetAdmissionProjectionStatusResponse, Error> {
    let response: Result<canic_contracts::dto::admission_control::ManagedStatus, Error> =
        ic_testkit::pic::query_wire(
            fixture.pic(),
            canister_id,
            caller,
            ic_testkit::pic::WireCallSpec {
                method: canic_contracts::protocol::CANIC_ADMISSION_STATUS,
                method_id: canic_contracts::protocol::admission_control::managed_status::METHOD_ID,
                request_limits: canic_contracts::protocol::admission_control::REQUEST_LIMITS,
                response_limits: canic_contracts::protocol::admission_control::RESPONSE_LIMITS,
            },
            &PageRequest {
                limit: u64::MAX,
                offset: 0,
            },
        )
        .expect("native managed admission status transport");
    response.map(Into::into)
}

fn fleet_admission_probe_as(
    fixture: &canic_tests::pic::ActiveComponentRegistryFixture,
    canister_id: ic_platform::principal::ExternalPrincipal,
    caller: ic_platform::principal::ExternalPrincipal,
) -> Result<(), Error> {
    ic_testkit::pic::query_wire(
        fixture.pic(),
        canister_id,
        caller,
        canic_tests::pic::issuer_probe::ADMISSION,
        &(),
    )
    .expect("query Candid harness call")
}

#[test]
fn local_authorization_sdk_denial_partition_is_bounded() {
    let fixture = setup_snapshot_resettable_active_component_registry();
    let caller = native_agent_subject();
    let observations = LocalAuthorizationDenialProbe::ALL.map(|probe| {
        let sample: Result<QueryPerfSample<String>, Error> = ic_testkit::pic::query_wire(
            fixture.pic(),
            fixture.verifier.canister_id,
            match probe {
                LocalAuthorizationDenialProbe::Anonymous => Principal::anonymous(),
                _ => caller,
            },
            canic_tests::pic::issuer_probe::DENIAL,
            &probe,
        )
        .expect("query Candid harness call");
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
    eprintln!("SDK caller-gate observations: {observations:?}");
}

fn native_agent_subject() -> ic_platform::principal::ExternalPrincipal {
    Secp256k1Identity::from_pem(TEST_IDENTITY_PEM.as_bytes())
        .expect("test PEM identity must parse")
        .sender()
        .expect("test identity must have a sender")
}

// Stable editors follow protected current ledger bindings, never assumed IDs.
fn allocated_memory_id(memory: &VectorMemory, key: &str) -> u64 {
    let runtime = ic_storage::MemoryRuntime::new(memory.clone()).expect("existing Storage layout");
    runtime
        .allocation_id(&ic_storage::StableKey::parse(key).unwrap())
        .unwrap()
        .get()
}

#[test]
fn allocation_lookup_preserves_stable_bytes() {
    let memory = VectorMemory::default();
    let key = "fixture.lookup.v1";
    let declarations = ic_storage::SealedDeclarationSnapshot::new(vec![
        ic_storage::MemoryRequest::new("fixture", key).unwrap(),
    ])
    .unwrap();
    let pool = ic_storage::AllocationGrants::new(vec![
        ic_storage::MemoryAuthority::new("fixture", "fixture.").unwrap(),
    ])
    .unwrap();
    let mut runtime = ic_storage::MemoryRuntime::new(memory.clone()).unwrap();
    runtime
        .bootstrap(&declarations, &pool, &ic_storage::GenericAllocationPolicy)
        .unwrap();
    let stable_key = ic_storage::StableKey::parse(key).expect("fixture stable key");
    let slot = runtime.allocation_id(&stable_key).unwrap().get();
    drop(runtime);
    let before = memory.borrow().clone();
    assert_eq!(allocated_memory_id(&memory, key), slot);
    assert_eq!(*memory.borrow(), before);
}

/// Installed controller approval, issuer possession and receiver session journey.
/// Credentials come from the installed issuer and actual authenticated account calls.
#[test]
fn portable_sessions_use_fresh_root_enrollment_and_preserve_logout_after_restore() {
    let fixture = setup_fresh_active_component_registry();
    let pic = fixture.pic();
    let target = fixture.verifier.canister_id;
    let issuer = fixture.issuer.canister_id;
    let subject = ic_platform::Principal::from_slice(&[43]);
    let browser = portable_browser(3);
    let registration = enroll_portable_fixture(&fixture);
    let input = portable_fixture_input(&fixture, 3);
    let spec = portable_spec(
        "canic_establish_portable_session",
        ic_auth_wire::EstablishSession::ID,
        ic_auth_wire::EstablishSession::REQUEST_LIMITS,
        ic_auth_wire::EstablishSession::RESPONSE_LIMITS,
    );
    let view: Result<ic_auth_wire::SessionView, ic_auth_wire::model::AdmissionRefusal> =
        ic_testkit::pic::update_wire(pic, target, browser, spec, &input).unwrap();
    let view = view.unwrap();
    assert_eq!(
        (view.issuer, view.subject, view.caller),
        (issuer.into(), subject, browser.into())
    );
    assert_eq!(
        generic_application_subject_as(&fixture, target, browser),
        Ok(subject.into())
    );
    let sample: Result<QueryPerfSample<ic_platform::Principal>, Error> =
        ic_testkit::pic::query_wire(
            pic,
            target,
            browser,
            canic_tests::pic::issuer_probe::ALLOW_PERF,
            &(),
        )
        .unwrap();
    let sample = sample.unwrap();
    assert_eq!(sample.value, subject);
    assert!(
        sample.local_instructions < MAX_LOCAL_AUTHORIZATION_INSTRUCTIONS,
        "successful guard: {} instructions",
        sample.local_instructions
    );
    assert_eq!(portable_read(pic, target, browser), Ok(view.clone()));
    let observed = operator_inspection::qualify_receiver(&fixture, browser, subject);
    let other_device = admit_independent_portable_logins(&fixture, &input, browser);
    operator_inspection::qualify(&fixture, &observed, browser);
    let missing = Principal::from_slice(&[99]);
    assert!(generic_application_subject_as(&fixture, target, missing).is_err());
    pic.advance_time(Duration::from_secs(7))
        .expect("PocketIC advance_time");
    pic.tick().expect("PocketIC tick");
    let retry: Result<ic_auth_wire::SessionView, ic_auth_wire::model::AdmissionRefusal> =
        ic_testkit::pic::update_wire(pic, target, browser, spec, &input).unwrap();
    assert_eq!(
        retry,
        Ok(view.clone()),
        "lost-reply retry retains the original lease after proof expiry"
    );
    retry_same_release_upgrade(pic, target, fixture.verifier_wasm(), fixture.root);
    for _ in 0..360 {
        pic.advance_time(Duration::from_secs(1))
            .expect("PocketIC advance_time");
        pic.tick().expect("PocketIC tick");
        if portable_read(pic, target, browser).is_ok() {
            break;
        }
    }
    assert_eq!(
        portable_read(pic, target, browser),
        Ok(view),
        "fresh post-restore Root exchange must retain the session's original expiry"
    );
    let logout = portable_spec(
        "canic_logout_portable_session",
        ic_auth_wire::Logout::ID,
        ic_auth_wire::Logout::REQUEST_LIMITS,
        ic_auth_wire::Logout::RESPONSE_LIMITS,
    );
    let ack: ic_auth_wire::model::LogoutResult =
        ic_testkit::pic::update_wire(pic, target, browser, logout, &()).unwrap();
    assert!(matches!(ack, ic_auth_wire::model::LogoutResult::Revoked(_)));
    assert!(portable_read(pic, target, browser).is_err());
    assert!(portable_read(pic, fixture.issuer.canister_id, browser).is_ok());
    assert!(portable_read(pic, target, other_device).is_ok());
    retry_same_release_upgrade(pic, target, fixture.verifier_wasm(), fixture.root);
    pic.advance_time(Duration::from_secs(2))
        .expect("PocketIC advance_time");
    for _ in 0..20 {
        pic.tick().expect("PocketIC tick");
    }
    assert!(portable_read(pic, target, browser).is_err());
    let denied: Result<ic_auth_wire::SessionView, ic_auth_wire::model::AdmissionRefusal> =
        ic_testkit::pic::update_wire(pic, target, browser, spec, &input).unwrap();
    assert!(denied.is_err(), "durable logout must survive restoration");
    retire_portable_fixture(&fixture, registration, other_device);
    drop(fixture);
}

fn portable_browser(device: u8) -> Principal {
    ic_auth::portable::browser_principal(
        &ic_platform::ed25519::SigningKey::from_bytes(&[device; 32])
            .verifying_key()
            .to_bytes(),
    )
    .into()
}

#[test]
fn volatile_portable_issuer_key_preparation_registration_and_restore() {
    use canic::dto::portable_enrollment::{
        PortableIssuerCustody, PortableIssuerEnrollment, PortableIssuerPreparedKey,
        PortableIssuerRegistrationRefusal as Refusal, PreparePortableIssuerKey,
        RetirePortableIssuer,
    };
    use canic::protocol::portable_enrollment::registration as wire;
    let fixture = setup_fresh_active_component_registry();
    let pic = fixture.pic();
    let issuer = fixture.issuer.canister_id;
    let prepare = registration_spec(
        "test_prepare_volatile_portable_issuer",
        0x5445_5354_0000_0809,
    );
    let request = PreparePortableIssuerKey {
        operation_id: [21; 32],
        custody: PortableIssuerCustody::TrustedVolatileLocal,
    };
    let denied: Result<Result<PortableIssuerPreparedKey, Refusal>, _> =
        ic_testkit::pic::update_wire(pic, issuer, portable_browser(3), prepare, &request);
    assert!(denied.is_err(), "only installed Root may prepare a key");
    let mut tee = request.clone();
    tee.custody = PortableIssuerCustody::TeeRequired;
    let denied: Result<PortableIssuerPreparedKey, Refusal> =
        ic_testkit::pic::update_wire(pic, issuer, fixture.root, prepare, &tee).unwrap();
    assert_eq!(denied, Err(Refusal::Custody));
    let prepared = prepare_volatile_key(&fixture, &request).unwrap();
    assert_eq!(
        prepare_volatile_key(&fixture, &request),
        Ok(prepared.clone())
    );
    let (challenge, enrollment) = register_volatile_key(&fixture, &prepared);
    let (credential_epoch, credential) = qualify_issuer_transport(&fixture);
    let register = registration_spec(
        "test_register_volatile_portable_issuer",
        0x5445_5354_0000_080a,
    );
    let retry: Result<PortableIssuerEnrollment, Refusal> =
        ic_testkit::pic::update_wire(pic, issuer, fixture.root, register, &challenge).unwrap();
    assert_eq!(retry, Ok(enrollment));
    let retire = registration_spec(wire::retire::METHOD, wire::retire::METHOD_ID);
    let retired: Result<(), Refusal> = ic_testkit::pic::update_wire(
        pic,
        fixture.root,
        Principal::anonymous(),
        retire,
        &RetirePortableIssuer {
            issuer: issuer.into(),
        },
    )
    .unwrap();
    retired.unwrap();
    retry_same_release_upgrade(pic, issuer, fixture.issuer_wasm(), fixture.root);
    assert_eq!(
        reconcile_fixture(
            &fixture,
            credential_epoch,
            [3; 32],
            Principal::from_slice(&[43])
        )
        .unwrap(),
        canic::dto::portable_issuance::PortableCredentialIssuanceStatus::Issued(credential)
    );
    assert_eq!(
        prepare_volatile_key(&fixture, &request),
        Err(Refusal::Unavailable)
    );
    let restored: Result<PortableIssuerEnrollment, Refusal> =
        ic_testkit::pic::update_wire(pic, issuer, fixture.root, register, &challenge).unwrap();
    assert_eq!(restored, Err(Refusal::Unavailable));
    let next = prepare_volatile_key(
        &fixture,
        &PreparePortableIssuerKey {
            operation_id: [22; 32],
            custody: PortableIssuerCustody::TrustedVolatileLocal,
        },
    )
    .unwrap();
    assert_ne!(next.public_key, prepared.public_key);
    register_volatile_key(&fixture, &next);
    drop(fixture);
}

fn qualify_issuer_transport(
    fixture: &ActiveComponentRegistryFixture,
) -> (u64, canic::dto::portable_issuance::IssuedPortableCredential) {
    use canic::dto::portable_issuance::{
        PortableCredentialIssuanceRefusal as E, PortableCredentialIssuanceStatus as Status,
    };
    let account = Principal::from_slice(&[43]);
    let request = portable_issue_request(fixture, 3);
    assert_eq!(
        issue_fixture(fixture, request.clone(), Principal::anonymous()),
        Err(E::Denied)
    );
    assert_eq!(
        issue_fixture(fixture, request.clone(), portable_browser(3)),
        Err(E::Denied)
    );
    let mut unauthorized = request.clone();
    unauthorized.grants[0].scopes = vec!["write".into()];
    assert_eq!(
        issue_fixture(fixture, unauthorized, account),
        Err(E::Denied)
    );
    assert_eq!(
        reconcile_fixture(fixture, request.epoch, request.operation_id, account).unwrap(),
        Status::Unresolved
    );
    let issued = issue_fixture(fixture, request.clone(), account).unwrap();
    let credential: ic_auth_protocol_types::PortableCredential =
        issued.credential.clone().try_into().unwrap();
    let claims = ic_auth::portable::decode_claims(&credential.claims).unwrap();
    assert_eq!(claims.account_subject, account.into());
    assert_eq!(claims.browser_public_key, request.browser_public_key);
    assert_eq!(claims.admission_expires_at_ns, issued.admission_deadline_ns);
    assert_eq!(
        issue_fixture(fixture, request.clone(), account),
        Ok(issued.clone())
    );
    assert_eq!(
        reconcile_fixture(
            fixture,
            request.epoch,
            request.operation_id,
            portable_browser(3)
        )
        .unwrap(),
        Status::Unresolved
    );
    fixture
        .pic()
        .advance_time(Duration::from_secs(7))
        .expect("PocketIC advance_time");
    fixture.pic().tick().expect("PocketIC tick");
    assert_eq!(
        reconcile_fixture(fixture, request.epoch, request.operation_id, account).unwrap(),
        Status::Issued(issued.clone())
    );
    assert_eq!(
        issue_fixture(fixture, request.clone(), account),
        Ok(issued.clone())
    );
    (request.epoch, issued)
}

fn prepare_volatile_key(
    fixture: &ActiveComponentRegistryFixture,
    request: &canic::dto::portable_enrollment::PreparePortableIssuerKey,
) -> Result<
    canic::dto::portable_enrollment::PortableIssuerPreparedKey,
    canic::dto::portable_enrollment::PortableIssuerRegistrationRefusal,
> {
    let spec = registration_spec(
        "test_prepare_volatile_portable_issuer",
        0x5445_5354_0000_0809,
    );
    ic_testkit::pic::update_wire(
        fixture.pic(),
        fixture.issuer.canister_id,
        fixture.root,
        spec,
        request,
    )
    .unwrap()
}

fn register_volatile_key(
    fixture: &ActiveComponentRegistryFixture,
    prepared: &canic::dto::portable_enrollment::PortableIssuerPreparedKey,
) -> (
    canic::dto::portable_enrollment::PortableIssuerChallenge,
    canic::dto::portable_enrollment::PortableIssuerEnrollment,
) {
    register_volatile_key_with_grants(fixture, prepared, None)
}

fn register_volatile_key_with_grants(
    fixture: &ActiveComponentRegistryFixture,
    prepared: &canic::dto::portable_enrollment::PortableIssuerPreparedKey,
    grants: Option<Vec<canic::dto::portable_enrollment::PortableEnrollmentGrant>>,
) -> (
    canic::dto::portable_enrollment::PortableIssuerChallenge,
    canic::dto::portable_enrollment::PortableIssuerEnrollment,
) {
    use canic::dto::portable_enrollment::{
        PortableIssuerChallenge, PortableIssuerEnrollment, PortableIssuerRegistrationLookup,
        PortableIssuerRegistrationRefusal as Refusal, PortableIssuerRegistrationStatus,
    };
    use canic::protocol::portable_enrollment::registration as wire;
    let pic = fixture.pic();
    // Give install-code admission its full rate window before the finite challenge.
    pic.advance_time(Duration::from_mins(6))
        .expect("PocketIC advance_time");
    pic.tick().expect("PocketIC tick");
    let mut request = portable_registration_request(
        fixture,
        pic.get_time()
            .expect("PocketIC get_time")
            .as_nanos_since_unix_epoch(),
        prepared.public_key,
    );
    request.operation_id = prepared.operation_id;
    request.approval.public_key = prepared.public_key;
    if let Some(grants) = grants {
        request.approval.grants = grants;
    }
    let prepare = registration_spec(wire::prepare::METHOD, wire::prepare::METHOD_ID);
    let challenge: Result<PortableIssuerChallenge, Refusal> =
        ic_testkit::pic::update_wire(pic, fixture.root, Principal::anonymous(), prepare, &request)
            .unwrap();
    let challenge = challenge.unwrap();
    let register = registration_spec(
        "test_register_volatile_portable_issuer",
        0x5445_5354_0000_080a,
    );
    let enrollment: Result<PortableIssuerEnrollment, Refusal> = ic_testkit::pic::update_wire(
        pic,
        fixture.issuer.canister_id,
        fixture.root,
        register,
        &challenge,
    )
    .unwrap();
    let enrollment = enrollment.unwrap();
    assert_eq!(
        enrollment.key,
        canic::dto::portable_enrollment::PortableIssuerKey::Ed25519 {
            public_key: prepared.public_key
        }
    );
    let lookup = registration_spec(
        "test_lookup_volatile_portable_issuer",
        0x5445_5354_0000_080b,
    );
    let observed: Result<PortableIssuerRegistrationStatus, Refusal> = ic_testkit::pic::update_wire(
        pic,
        fixture.issuer.canister_id,
        fixture.root,
        lookup,
        &PortableIssuerRegistrationLookup {
            operation_id: prepared.operation_id,
        },
    )
    .unwrap();
    assert_eq!(
        observed,
        Ok(PortableIssuerRegistrationStatus::Registered(
            enrollment.clone()
        ))
    );
    (challenge, enrollment)
}

fn enroll_portable_fixture(
    fixture: &ActiveComponentRegistryFixture,
) -> canic::dto::portable_enrollment::PreparePortableIssuer {
    use canic::dto::portable_enrollment::{
        PortableIssuerCustody, PortableIssuerEnrollment, PortableIssuerRegistrationLookup,
        PortableIssuerRegistrationRefusal, PortableIssuerRegistrationStatus, PreparePortableIssuer,
        PreparePortableIssuerKey, RegisterPortableIssuer,
    };
    use canic::protocol::portable_enrollment::registration as wire;
    let pic = fixture.pic();
    let issuer = fixture.issuer.canister_id;
    let browser = portable_browser(3);
    // Clear the install-code rate window before creating the finite challenge.
    pic.advance_time(Duration::from_mins(6))
        .expect("PocketIC advance_time");
    pic.tick().expect("PocketIC tick");
    let now = pic
        .get_time()
        .expect("PocketIC get_time")
        .as_nanos_since_unix_epoch();
    let key = prepare_volatile_key(
        fixture,
        &PreparePortableIssuerKey {
            operation_id: [21; 32],
            custody: PortableIssuerCustody::TrustedVolatileLocal,
        },
    )
    .unwrap();
    let request: PreparePortableIssuer =
        portable_registration_request(fixture, now, key.public_key);
    let challenge = prepare_registration_across_root_restore(fixture, &request, browser);
    let complete = registration_spec(wire::complete::METHOD, wire::complete::METHOD_ID);
    let invalid = RegisterPortableIssuer {
        operation_id: request.operation_id,
        signature: vec![0; 64],
    };
    let denied: Result<Result<PortableIssuerEnrollment, PortableIssuerRegistrationRefusal>, _> =
        ic_testkit::pic::update_wire(pic, fixture.root, browser, complete, &invalid);
    assert!(
        denied.is_err(),
        "a browser cannot complete issuer registration"
    );
    let invalid_proof: Result<PortableIssuerEnrollment, PortableIssuerRegistrationRefusal> =
        ic_testkit::pic::update_wire(pic, fixture.root, issuer, complete, &invalid).unwrap();
    assert_eq!(
        invalid_proof,
        Err(PortableIssuerRegistrationRefusal::Possession)
    );
    let producer = registration_spec(
        "test_register_volatile_portable_issuer",
        0x5445_5354_0000_080a,
    );
    let enrollment: Result<PortableIssuerEnrollment, PortableIssuerRegistrationRefusal> =
        ic_testkit::pic::update_wire(pic, issuer, fixture.root, producer, &challenge).unwrap();
    let enrollment = enrollment.unwrap();
    assert_eq!(enrollment.issuer, issuer.into());
    assert_eq!(enrollment.expires_at_ns, request.approval.expires_at_ns);
    let retry: Result<PortableIssuerEnrollment, PortableIssuerRegistrationRefusal> =
        ic_testkit::pic::update_wire(pic, issuer, fixture.root, producer, &challenge).unwrap();
    assert_eq!(retry.unwrap(), enrollment);
    let lookup = registration_spec(
        "test_lookup_volatile_portable_issuer",
        0x5445_5354_0000_080b,
    );
    let observed: Result<PortableIssuerRegistrationStatus, PortableIssuerRegistrationRefusal> =
        ic_testkit::pic::update_wire(
            pic,
            issuer,
            fixture.root,
            lookup,
            &PortableIssuerRegistrationLookup {
                operation_id: request.operation_id,
            },
        )
        .unwrap();
    assert_eq!(
        observed.unwrap(),
        PortableIssuerRegistrationStatus::Registered(enrollment)
    );
    await_portable_receivers(fixture, browser);
    request
}

fn prepare_registration_across_root_restore(
    fixture: &ActiveComponentRegistryFixture,
    request: &canic::dto::portable_enrollment::PreparePortableIssuer,
    browser: Principal,
) -> canic::dto::portable_enrollment::PortableIssuerChallenge {
    use canic::{
        dto::portable_enrollment::{
            PortableIssuerChallenge, PortableIssuerRegistrationRefusal as E,
        },
        protocol::portable_enrollment::registration as wire,
    };
    let pic = fixture.pic();
    let prepare = registration_spec(wire::prepare::METHOD, wire::prepare::METHOD_ID);
    let denied: Result<Result<PortableIssuerChallenge, E>, _> =
        ic_testkit::pic::update_wire(pic, fixture.root, browser, prepare, request);
    assert!(denied.is_err(), "only controllers may prepare authority");
    let prepared: Result<PortableIssuerChallenge, E> =
        ic_testkit::pic::update_wire(pic, fixture.root, Principal::anonymous(), prepare, request)
            .unwrap();
    let challenge = prepared.unwrap();
    assert_eq!(challenge.authority, fixture.root.into());
    assert_eq!(
        challenge.fleet,
        fixture.issuer.authority.binding.fleet.fleet
    );
    assert_eq!(challenge.issuer, fixture.issuer.canister_id.into());
    let retry: Result<PortableIssuerChallenge, E> =
        ic_testkit::pic::update_wire(pic, fixture.root, Principal::anonymous(), prepare, request)
            .unwrap();
    assert_eq!(retry, Ok(challenge.clone()));
    retry_same_release_upgrade(
        pic,
        fixture.root,
        fixture.root_wasm(),
        Principal::anonymous(),
    );
    let restored: Result<PortableIssuerChallenge, E> =
        ic_testkit::pic::update_wire(pic, fixture.root, Principal::anonymous(), prepare, request)
            .unwrap();
    assert_eq!(
        restored,
        Ok(challenge.clone()),
        "Root restoration retains the original challenge and deadline"
    );
    challenge
}

fn await_portable_receivers(fixture: &ActiveComponentRegistryFixture, browser: Principal) {
    let pic = fixture.pic();
    let target = fixture.verifier.canister_id;
    for _ in 0..30 {
        pic.advance_time(Duration::from_secs(1))
            .expect("PocketIC advance_time");
        pic.tick().expect("PocketIC tick");
        if [target, fixture.issuer.canister_id]
            .into_iter()
            .all(|receiver| {
                matches!(
                    portable_read(pic, receiver, browser),
                    Err(ic_auth_wire::model::SessionReadRefusal::MissingOrExpired)
                )
            })
        {
            break;
        }
    }
    assert_eq!(
        portable_read(pic, target, browser),
        Err(ic_auth_wire::model::SessionReadRefusal::MissingOrExpired),
        "direct authenticated Root pull must activate the receiver"
    );
}

fn portable_registration_request(
    fixture: &ActiveComponentRegistryFixture,
    now: u64,
    public_key: [u8; 32],
) -> canic::dto::portable_enrollment::PreparePortableIssuer {
    use canic::dto::portable_enrollment::{
        PortableEnrollmentGrant, PortableIssuerApproval, PortableIssuerCustody,
        PreparePortableIssuer,
    };
    PreparePortableIssuer {
        operation_id: [21; 32],
        approval: PortableIssuerApproval {
            issuer: fixture.issuer.canister_id.into(),
            public_key,
            custody: PortableIssuerCustody::TrustedVolatileLocal,
            not_before_ns: now,
            expires_at_ns: now + 3_600_000_000_000,
            grants: vec![PortableEnrollmentGrant {
                role: "issuer".into(),
                scopes: vec![
                    "app00:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".into(),
                ],
            }],
            governance_digest: [7; 32],
            governance_expires_at_ns: now + 3_600_000_000_000,
        },
    }
}

fn retire_portable_fixture(
    fixture: &ActiveComponentRegistryFixture,
    mut registration: canic::dto::portable_enrollment::PreparePortableIssuer,
    browser: Principal,
) {
    use canic::{
        dto::portable_enrollment::{
            PortableIssuerChallenge, PortableIssuerRegistrationRefusal, RetirePortableIssuer,
        },
        protocol::portable_enrollment::registration as wire,
    };
    let pic = fixture.pic();
    let retire = registration_spec(wire::retire::METHOD, wire::retire::METHOD_ID);
    let request = RetirePortableIssuer {
        issuer: registration.approval.issuer,
    };
    let denied: Result<Result<(), PortableIssuerRegistrationRefusal>, _> =
        ic_testkit::pic::update_wire(pic, fixture.root, browser, retire, &request);
    assert!(
        denied.is_err(),
        "only controllers may retire issuer authority"
    );
    let retired: Result<(), PortableIssuerRegistrationRefusal> =
        ic_testkit::pic::update_wire(pic, fixture.root, Principal::anonymous(), retire, &request)
            .unwrap();
    retired.unwrap();
    registration.operation_id = [22; 32];
    let prepare = registration_spec(wire::prepare::METHOD, wire::prepare::METHOD_ID);
    let reenrollment: Result<PortableIssuerChallenge, PortableIssuerRegistrationRefusal> =
        ic_testkit::pic::update_wire(
            pic,
            fixture.root,
            Principal::anonymous(),
            prepare,
            &registration,
        )
        .unwrap();
    assert_eq!(
        reenrollment,
        Err(PortableIssuerRegistrationRefusal::Retired)
    );
    for _ in 0..120 {
        pic.advance_time(Duration::from_secs(1))
            .expect("PocketIC advance_time");
        pic.tick().expect("PocketIC tick");
        if portable_read(pic, fixture.verifier.canister_id, browser).is_err() {
            break;
        }
    }
    assert!(
        portable_read(pic, fixture.verifier.canister_id, browser).is_err(),
        "retirement closes existing sessions within the enrollment bound"
    );
}

fn portable_fixture_input(
    fixture: &ActiveComponentRegistryFixture,
    device: u8,
) -> ic_auth_wire::SessionInput {
    let issued = issue_fixture(
        fixture,
        portable_issue_request(fixture, device),
        Principal::from_slice(&[43]),
    )
    .unwrap();
    ic_auth_wire::SessionInput {
        credential: issued.credential,
        scopes: ic_wire::List::new(vec![
            ic_wire::Text::new(
                "app00:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".into(),
            )
            .unwrap(),
        ])
        .unwrap(),
        ttl_secs: Some(1800),
    }
}

fn portable_issue_request(
    fixture: &ActiveComponentRegistryFixture,
    device: u8,
) -> canic::dto::portable_issuance::IssuePortableCredential {
    let epoch = issuer_context(fixture).epoch;
    canic::dto::portable_issuance::IssuePortableCredential {
        epoch,
        operation_id: [device; 32],
        login_epoch: epoch,
        login_id: [device; 32],
        browser_public_key: ic_platform::ed25519::SigningKey::from_bytes(&[device; 32])
            .verifying_key()
            .to_bytes(),
        grants: vec![canic::dto::portable_enrollment::PortableEnrollmentGrant {
            role: "issuer".into(),
            scopes: vec!["app00:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".into()],
        }],
    }
}

fn issue_fixture(
    fixture: &ActiveComponentRegistryFixture,
    request: canic::dto::portable_issuance::IssuePortableCredential,
    account: Principal,
) -> Result<
    canic::dto::portable_issuance::IssuedPortableCredential,
    canic::dto::portable_issuance::PortableCredentialIssuanceRefusal,
> {
    ic_testkit::pic::update_wire(
        fixture.pic(),
        fixture.issuer.canister_id,
        account,
        portable_spec(
            "test_issue_portable_credential",
            0x5445_5354_0000_080c,
            canic::protocol::portable_issuance::REQUEST_LIMITS,
            canic::protocol::portable_issuance::RESPONSE_LIMITS,
        ),
        &request,
    )
    .unwrap()
}

fn reconcile_fixture(
    fixture: &ActiveComponentRegistryFixture,
    epoch: u64,
    operation_id: [u8; 32],
    account: Principal,
) -> Result<
    canic::dto::portable_issuance::PortableCredentialIssuanceStatus,
    canic::dto::portable_issuance::PortableCredentialIssuanceRefusal,
> {
    ic_testkit::pic::update_wire(
        fixture.pic(),
        fixture.issuer.canister_id,
        account,
        portable_spec(
            "test_reconcile_portable_credential",
            0x5445_5354_0000_080d,
            canic::protocol::portable_issuance::REQUEST_LIMITS,
            canic::protocol::portable_issuance::RESPONSE_LIMITS,
        ),
        &canic::dto::portable_issuance::ReconcilePortableCredential {
            epoch,
            operation_id,
        },
    )
    .unwrap()
}

fn issuer_context(
    fixture: &ActiveComponentRegistryFixture,
) -> canic::dto::portable_issuance::PortableIssuanceContext {
    let result: Result<_, canic::dto::portable_issuance::PortableCredentialIssuanceRefusal> =
        ic_testkit::pic::query_wire(
            fixture.pic(),
            fixture.issuer.canister_id,
            Principal::anonymous(),
            portable_spec(
                "test_portable_issuance_context",
                0x5445_5354_0000_080e,
                canic::protocol::portable_issuance::REQUEST_LIMITS,
                canic::protocol::portable_issuance::RESPONSE_LIMITS,
            ),
            &(),
        )
        .unwrap();
    result.unwrap()
}

const fn registration_spec(method: &'static str, method_id: u64) -> ic_testkit::pic::WireCallSpec {
    portable_spec(
        method,
        method_id,
        canic::protocol::portable_enrollment::registration::REQUEST_LIMITS,
        canic::protocol::portable_enrollment::registration::RESPONSE_LIMITS,
    )
}

fn admit_independent_portable_logins(
    fixture: &ActiveComponentRegistryFixture,
    input: &ic_auth_wire::SessionInput,
    browser: Principal,
) -> Principal {
    let spec = portable_spec(
        "canic_establish_portable_session",
        ic_auth_wire::EstablishSession::ID,
        ic_auth_wire::EstablishSession::REQUEST_LIMITS,
        ic_auth_wire::EstablishSession::RESPONSE_LIMITS,
    );
    let other_receiver: Result<ic_auth_wire::SessionView, ic_auth_wire::model::AdmissionRefusal> =
        ic_testkit::pic::update_wire(
            fixture.pic(),
            fixture.issuer.canister_id,
            browser,
            spec,
            input,
        )
        .unwrap();
    other_receiver.unwrap();
    let other_device = portable_browser(4);
    let other_login: Result<ic_auth_wire::SessionView, ic_auth_wire::model::AdmissionRefusal> =
        ic_testkit::pic::update_wire(
            fixture.pic(),
            fixture.verifier.canister_id,
            other_device,
            spec,
            &portable_fixture_input(fixture, 4),
        )
        .unwrap();
    assert_eq!(other_login.unwrap().caller, other_device.into());
    other_device
}

const fn portable_spec(
    method: &'static str,
    method_id: u64,
    request_limits: ic_wire::Limits,
    response_limits: ic_wire::Limits,
) -> ic_testkit::pic::WireCallSpec {
    ic_testkit::pic::WireCallSpec {
        method,
        method_id,
        request_limits,
        response_limits,
    }
}

fn portable_read(
    pic: &PocketIc,
    target: Principal,
    caller: Principal,
) -> Result<ic_auth_wire::SessionView, ic_auth_wire::model::SessionReadRefusal> {
    ic_testkit::pic::query_wire(
        pic,
        target,
        caller,
        portable_spec(
            "canic_portable_session_read",
            ic_auth_wire::ReadSession::ID,
            ic_auth_wire::ReadSession::REQUEST_LIMITS,
            ic_auth_wire::ReadSession::RESPONSE_LIMITS,
        ),
        &(),
    )
    .expect("portable session read Wire transport")
}
