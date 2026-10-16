use candid::{CandidType, Principal};
use canic::{
    Error,
    dto::{
        fleet_admission::{FleetAdmissionProjectionPhase, FleetAdmissionProjectionStatusResponse},
        metrics::QueryPerfSample,
        page::PageRequest,
    },
    protocol,
};
use canic_contracts::dto::wire::projection::admission::RemoteRootStatusRequest as FleetAdmissionManagedStatusRequest;
use canic_contracts::dto::wire::projection::fixture_baseline::ManagedAdmissionStatusResponseFragment as FleetAdmissionManagedStatusResponse;
use canic_core::cdk::structures::{
    VectorMemory,
    cell::Cell,
    memory::{MemoryId, MemoryManager},
};
use canic_testing_internal::pic::{
    ActiveComponentRegistryFixture, setup_active_component_registry,
    setup_fresh_active_component_registry, upgrade_args,
};
use ic_agent::{Identity, identity::Secp256k1Identity};
use ic_testkit::pic::{CandidCallExt, CanisterInstallExt, RetryPolicy};
use ic_testkit::pocket_ic::{PocketIc, common::rest::BlobCompression};
use std::{cell::RefCell, rc::Rc, time::Duration};

const MAX_LOCAL_AUTHORIZATION_INSTRUCTIONS: u64 = 1_000_000;
const INSTALL_CODE_RETRY_LIMIT: usize = 4;
const INSTALL_CODE_COOLDOWN: Duration = Duration::from_mins(5);

// Deterministic test-only identity; it has no authority outside this fresh PocketIC runtime.
const TEST_IDENTITY_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIGEAgEAMBAGByqGSM49AgEGBSuBBAAKBG0wawIBAQQgCDLudkRxUeRDhnUp2pvL
xLDICLIoNCa1sQdMgz5Y14GhRANCAASA7zusnWjPN0y8nJlD4YAEOpTEYu+CcCdO
VwidXc26G4+/g7dUbMwbN4E3d3bpxHEP31M+2by6jY67MqFKKroR
-----END PRIVATE KEY-----";

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
    let policy = RetryPolicy::new(
        std::num::NonZeroUsize::new(INSTALL_CODE_RETRY_LIMIT)
            .expect("non-zero install attempt limit"),
        INSTALL_CODE_COOLDOWN,
    );
    pic.retry_install_code(policy, || {
        pic.upgrade_canister(canister_id, wasm.clone(), args.clone(), Some(sender))
    })
    .expect("same-release upgrade must succeed after bounded rate-limit retries");
}

#[test]
fn selected_issuer_corruption_rejects_same_release_restore() {
    let fixture = setup_fresh_active_component_registry();
    let issuer = fixture.issuer.canister_id;
    let stable_memory: VectorMemory =
        Rc::new(RefCell::new(fixture.pic().get_stable_memory(issuer)));
    let manager = MemoryManager::init(stable_memory.clone());
    let mut issuer_cell = Cell::<Vec<u8>, _>::init(
        manager.get(MemoryId::new(allocated_memory_id(
            &stable_memory,
            "canic.core.auth.delegated_token_issuer.state.v1",
        ))),
        Vec::new(),
    );
    issuer_cell.set(vec![0xff]);
    drop(issuer_cell);
    drop(manager);
    let bytes = Rc::try_unwrap(stable_memory)
        .expect("exclusive stable-memory editor")
        .into_inner();
    fixture
        .pic()
        .set_stable_memory(issuer, bytes, BlobCompression::NoCompression);
    let error = fixture
        .pic()
        .upgrade_canister(
            issuer,
            fixture.issuer_wasm(),
            upgrade_args(),
            Some(fixture.root),
        )
        .expect_err("selected issuer corruption must fail synchronous restoration");
    drop(fixture);
    assert_eq!(
        error.error_code,
        ic_testkit::pic::ErrorCode::CanisterCalledTrap
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
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
    canister_id: candid::Principal,
    caller: candid::Principal,
) -> Result<candid::Principal, Error> {
    fixture
        .pic()
        .query_candid_as(
            canister_id,
            caller,
            "issuer_application_subject",
            (),
            canic::testing::CANDID_REPLY_LIMITS,
        )
        .expect("query Candid harness call")
}

fn fleet_admission_status_as(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
    canister_id: candid::Principal,
    caller: candid::Principal,
) -> Result<FleetAdmissionProjectionStatusResponse, Error> {
    let response: Result<FleetAdmissionManagedStatusResponse, Error> = fixture
        .pic()
        .query_candid_as(
            canister_id,
            caller,
            protocol::CANIC_ADMISSION_STATUS,
            (FleetAdmissionManagedStatusRequest::Admission(PageRequest {
                offset: 0,
                limit: u64::MAX,
            }),),
            canic::testing::CANDID_REPLY_LIMITS,
        )
        .expect("query Candid harness call");
    response.map(|response| match response {
        FleetAdmissionManagedStatusResponse::Admission(response) => response,
    })
}

fn fleet_admission_probe_as(
    fixture: &canic_testing_internal::pic::ActiveComponentRegistryFixture,
    canister_id: candid::Principal,
    caller: candid::Principal,
) -> Result<(), Error> {
    fixture
        .pic()
        .query_candid_as(
            canister_id,
            caller,
            "issuer_fleet_admission_probe",
            (),
            canic::testing::CANDID_REPLY_LIMITS,
        )
        .expect("query Candid harness call")
}

#[test]
fn closed_local_authorization_denial_partition_is_bounded() {
    let fixture = setup_snapshot_resettable_active_component_registry();
    let caller = native_agent_subject();
    let observations = LocalAuthorizationDenialProbe::ALL.map(|probe| {
        let sample: Result<QueryPerfSample<String>, Error> = fixture
            .pic()
            .query_candid_as(
                fixture.verifier.canister_id,
                caller,
                "issuer_application_denial_perf",
                (probe,),
                canic::testing::CANDID_REPLY_LIMITS,
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
    eprintln!("0.105 B6 denial observations: {observations:?}");
}

fn native_agent_subject() -> candid::Principal {
    Secp256k1Identity::from_pem(TEST_IDENTITY_PEM.as_bytes())
        .expect("test PEM identity must parse")
        .sender()
        .expect("test identity must have a sender")
}

// Stable editors follow protected current ledger bindings, never assumed IDs.
fn allocated_memory_id(memory: &VectorMemory, key: &str) -> u8 {
    let runtime = ic_memory::MemoryRuntime::new(memory.clone()).expect("existing memory layout");
    // Only inspect recovered bindings. Empty proposed inputs do not grant bootstrap authority.
    let declarations = ic_memory::SealedDeclarationSnapshot::new(Vec::new()).unwrap();
    let pool = ic_memory::MemoryAllocationPool::new(Vec::new(), Vec::new()).unwrap();
    let report = runtime.doctor_report(&declarations, &pool, &ic_memory::GenericAllocationPolicy);
    report
        .ledger
        .expect("current committed allocation authority")
        .records
        .iter()
        .find(|record| record.allocation.stable_key().as_str() == key)
        .expect("fixture allocation exists")
        .allocation
        .slot()
        .id()
}

#[test]
fn allocation_lookup_preserves_stable_bytes() {
    let memory = VectorMemory::default();
    let key = "fixture.lookup.v1";
    let declarations = ic_memory::SealedDeclarationSnapshot::new(vec![
        ic_memory::MemoryRequest::new("fixture", key).unwrap(),
    ])
    .unwrap();
    let pool = ic_memory::MemoryAllocationPool::new(
        vec![ic_memory::MemoryAuthority::new("fixture", "fixture.").unwrap()],
        Vec::new(),
    )
    .unwrap();
    let mut runtime = ic_memory::MemoryRuntime::new(memory.clone()).unwrap();
    runtime
        .bootstrap(&declarations, &pool, &ic_memory::GenericAllocationPolicy)
        .unwrap();
    let slot = runtime.memory_id(key).unwrap();
    drop(runtime);
    let before = memory.borrow().clone();
    assert_eq!(allocated_memory_id(&memory, key), slot);
    assert_eq!(*memory.borrow(), before);
}
