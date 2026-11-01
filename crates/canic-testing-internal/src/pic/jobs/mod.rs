//! Installed proof of one application-owned Jobs queue under Canic lifecycle authority.
use crate::pic::{application_probe::*, artifacts::INTERNAL_TEST_RELEASE_BUILD_ID, lifecycle};
use canic::{Error, testing::ManagedAppFixture};
use ic_platform::principal::ExternalPrincipal as Principal;
use ic_testkit::{
    fake_principal,
    pic::{WireCallSpec, query_wire, update_wire},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Deserialize, Eq, PartialEq)]
enum State {
    Pending,
    Running,
    Uncertain,
    Completed,
    Failed,
    Paused,
    Cancelled,
}
#[derive(Debug, Deserialize, Eq, PartialEq)]
struct Job {
    id: u128,
    state: State,
    sequence: u64,
}
#[derive(Debug, Deserialize, Eq, PartialEq)]
struct View {
    total: u64,
    receipts: u64,
    jobs: Vec<Job>,
}
#[derive(Serialize)]
struct Enqueue {
    delta: u64,
    delay_secs: u64,
}
#[derive(Serialize)]
struct Reconcile {
    job: u128,
    sequence: u64,
}
#[derive(Debug, Deserialize, Serialize)]
struct Evidence {
    job: u128,
    sequence: u64,
    delta: u64,
}

fn update<Request: Serialize, Response: serde::de::DeserializeOwned>(
    fixture: &ManagedAppFixture,
    caller: Principal,
    spec: WireCallSpec,
    request: &Request,
) -> Result<Response, Error> {
    update_wire(fixture.pic(), fixture.app(), caller, spec, request).expect("Jobs Wire transport")
}
const fn owner() -> Principal {
    Principal::from_slice(&[17; 29])
}

fn view(fixture: &ManagedAppFixture) -> View {
    let result: Result<View, Error> = query_wire(
        fixture.pic(),
        fixture.app(),
        owner(),
        MANAGED_JOBS_STATUS,
        &(),
    )
    .expect("Jobs query transport");
    result.expect("admitted valid Jobs queue")
}
fn wait_for(fixture: &ManagedAppFixture, job: u128, state: State) -> View {
    for _ in 0..30 {
        let observed = view(fixture);
        if observed
            .jobs
            .iter()
            .any(|entry| entry.id == job && entry.state == state)
        {
            return observed;
        }
        fixture.pic().advance_time(Duration::from_secs(1));
        fixture.pic().tick();
        fixture.pic().tick();
    }
    panic!("Jobs queue did not reach {state:?}: {:?}", view(fixture));
}
fn enqueue(fixture: &ManagedAppFixture, delta: u64, delay_secs: u64) -> u128 {
    update(
        fixture,
        owner(),
        MANAGED_JOBS_ENQUEUE,
        &Enqueue { delta, delay_secs },
    )
    .expect("enqueue immutable job")
}
fn fault(fixture: &ManagedAppFixture, mode: u8) {
    let result: Result<(), Error> = update(fixture, owner(), MANAGED_JOBS_FAULT, &mode);
    result.expect("select private fixture fault");
}
fn reconcile(fixture: &ManagedAppFixture, job: u128, sequence: u64) -> Result<(), Error> {
    update(
        fixture,
        owner(),
        MANAGED_JOBS_RECONCILE,
        &Reconcile { job, sequence },
    )
}

fn install_fixture() -> ManagedAppFixture {
    let owner = owner();
    let other = fake_principal(18);
    let wasm = lifecycle::managed_lifecycle_wasm();
    println!(
        "Jobs qualification installed Wasm SHA256: {:02x?}",
        canic_core::cdk::utils::hash::sha256_bytes(&wasm)
    );
    let mut admitted = vec![owner, other];
    admitted.sort_unstable();
    let input = canic::testing::ManagedAppQualificationInput::new(
        include_str!("../../../../../canisters/test/managed_lifecycle_probe/canic.toml"),
        "test",
        "test",
        INTERNAL_TEST_RELEASE_BUILD_ID.1,
        admitted,
        wasm,
    );
    let fixture =
        canic::testing::install_managed_app(input).expect("install canonical managed Jobs probe");
    let prepared: Result<Result<u128, Error>, _> = update_wire(
        fixture.pic(),
        fixture.app(),
        owner,
        MANAGED_JOBS_ENQUEUE,
        &Enqueue {
            delta: 7,
            delay_secs: 0,
        },
    );
    assert!(
        matches!(prepared, Err(ic_testkit::pic::WireCallError::Call { .. })),
        "prepared runtime must reject before application mutation"
    );
    fixture
        .configure_and_wait_until_active(30)
        .expect("activate managed Jobs probe");
    let denied: Result<u128, Error> = update(
        &fixture,
        other,
        MANAGED_JOBS_ENQUEUE,
        &Enqueue {
            delta: 7,
            delay_secs: 0,
        },
    );
    assert_eq!(
        denied.unwrap_err().code(),
        canic::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
    );
    assert!(view(&fixture).jobs.is_empty());

    fixture
}

fn qualify_rollback(fixture: &ManagedAppFixture) -> u128 {
    fault(fixture, 1);
    let first = enqueue(fixture, 7, 0);
    let rolled_back: Result<Result<(), Error>, _> = update_wire(
        fixture.pic(),
        fixture.app(),
        owner(),
        MANAGED_JOBS_DRAIN,
        &(),
    );
    assert!(
        matches!(
            rolled_back,
            Err(ic_testkit::pic::WireCallError::Call { .. })
        ),
        "the intent-writing dispatch must actually trap"
    );
    assert_eq!(
        view(fixture),
        View {
            total: 0,
            receipts: 0,
            jobs: vec![Job {
                id: first,
                state: State::Pending,
                sequence: 0
            }]
        }
    );
    fault(fixture, 0);
    // Same-release restoration reconstructs the volatile wakeup from Pending demand.
    fixture
        .upgrade_same_release(Duration::from_mins(5))
        .expect("restore rolled-back intent and wakeup");
    let later = enqueue(fixture, 3, 3600);
    let completed = wait_for(fixture, first, State::Completed);
    assert_eq!((completed.total, completed.receipts), (7, 1));

    later
}

fn qualify_lost_reply(fixture: &ManagedAppFixture) -> u128 {
    fault(fixture, 2);
    let lost = enqueue(fixture, 11, 0);
    let mut observed = wait_for(fixture, lost, State::Running);
    for _ in 0..30 {
        if observed.total == 18 {
            break;
        }
        fixture.pic().tick();
        observed = view(fixture);
    }
    assert_eq!((observed.total, observed.receipts), (18, 2));
    // Ensure the reply continuation has run and trapped before reconstructing heap state.
    for _ in 0..5 {
        fixture.pic().tick();
    }
    assert_eq!(
        view(fixture)
            .jobs
            .iter()
            .find(|entry| entry.id == lost)
            .unwrap()
            .state,
        State::Running
    );
    lost
}

fn qualify_restoration(fixture: &ManagedAppFixture, lost: u128, later: u128) {
    fixture
        .upgrade_same_release(Duration::from_mins(5))
        .expect("restore committed Running intent");
    let restored = view(fixture);
    assert_eq!((restored.total, restored.receipts), (18, 2));
    assert_eq!(
        restored
            .jobs
            .iter()
            .find(|entry| entry.id == lost)
            .unwrap()
            .state,
        State::Uncertain
    );
    for _ in 0..5 {
        fixture.pic().advance_time(Duration::from_secs(1));
        fixture.pic().tick();
    }
    assert_eq!(
        view(fixture),
        restored,
        "uncertainty never grants redelivery"
    );
    assert_eq!(
        reconcile(fixture, lost, 2).unwrap_err().code(),
        canic::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    reconcile(fixture, lost, 1).expect("exact retained receipt resolves uncertainty");
    reconcile(fixture, lost, 1).expect("duplicate disposition is idempotent");
    let forged: Result<Evidence, Error> = update(
        fixture,
        owner(),
        MANAGED_JOBS_EFFECT,
        &Evidence {
            job: lost,
            sequence: 1,
            delta: 99,
        },
    );
    assert_eq!(
        forged.unwrap_err().code(),
        canic::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    fixture.pic().advance_time(Duration::from_secs(3600));
    let completed = wait_for(fixture, later, State::Completed);
    assert_eq!((completed.total, completed.receipts), (21, 3));
    assert!(
        completed
            .jobs
            .iter()
            .all(|entry| entry.state == State::Completed && entry.sequence == 1)
    );
}

fn qualify_invalid_restore(fixture: &ManagedAppFixture) {
    let result: Result<(), Error> = update(fixture, owner(), MANAGED_JOBS_CORRUPT, &());
    result.expect("private fault corrupts retained frame");
    assert!(
        matches!(
            fixture.upgrade_same_release(Duration::from_mins(5)),
            Err(canic::testing::ManagedAppQualificationError::Install(_))
        ),
        "corrupt restoration must refuse without reset"
    );
    let invalid: Result<View, Error> = query_wire(
        fixture.pic(),
        fixture.app(),
        owner(),
        MANAGED_JOBS_STATUS,
        &(),
    )
    .expect("old code remains installed after refused upgrade");
    assert_eq!(
        invalid.unwrap_err().code(),
        canic::diagnostics::codes::STATE_INVALID.raw_code()
    );
}

#[test]
fn managed_jobs_queue_preserves_attempts_across_upgrade_and_lost_reply() {
    let fixture = install_fixture();
    let later = qualify_rollback(&fixture);
    let lost = qualify_lost_reply(&fixture);
    qualify_restoration(&fixture, lost, later);
    qualify_invalid_restore(&fixture);
}

pub(super) fn governed_cases() -> Vec<crate::pic::GovernedTestCase> {
    crate::pic::cases::registered![(
        "application-owned Jobs rollback and lost-reply recovery",
        managed_jobs_queue_preserves_attempts_across_upgrade_and_lost_reply
    )]
}
