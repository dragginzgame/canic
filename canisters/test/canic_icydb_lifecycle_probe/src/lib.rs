//! Managed Canic plus published-IcyDB synchronous lifecycle composition probe.

#![expect(clippy::unused_async)]
#![expect(
    clippy::redundant_pub_crate,
    reason = "the published IcyDB actor uses crate-visible generated participant bindings"
)]

use candid::CandidType;
use std::cell::{Cell, RefCell};

icydb::start!(participant);

/// Lifecycle phase most recently reconstructed on the current heap.
#[derive(CandidType, Clone, Copy, Debug, Eq, PartialEq)]
enum ProbeLifecycleHook {
    Init,
    PostUpgrade,
}

/// Current published-IcyDB startup phase, projected without leaking its types.
#[derive(CandidType, Clone, Copy, Debug, Eq, PartialEq)]
enum ProbeDatabaseStartup {
    Failed,
    Ready,
    Recovering,
}

/// Typed result for one exact composition observation.
#[derive(CandidType, Clone, Copy, Debug, Eq, PartialEq)]
enum ProbeEvidence {
    Missing,
    Observed,
}

impl ProbeEvidence {
    const fn from_observation(observed: bool) -> Self {
        if observed {
            Self::Observed
        } else {
            Self::Missing
        }
    }
}

/// Observable composition evidence retained on the current heap.
#[derive(CandidType, Clone, Debug, Eq, PartialEq)]
struct LifecycleCompositionSnapshot {
    hook: ProbeLifecycleHook,
    participant_runs: u32,
    icydb_row_observed_after_participant: ProbeEvidence,
    icydb_row_live: ProbeEvidence,
    canic_row_observed_during_callback: ProbeEvidence,
    icydb_row_observed_during_canic_callback: ProbeEvidence,
    canic_setup_runs: u32,
    canic_install_runs: u32,
    canic_upgrade_runs: u32,
    database_startup: ProbeDatabaseStartup,
    database_access: ProbeEvidence,
}

#[derive(Clone, Copy)]
struct LifecycleCompositionRecord {
    hook: ProbeLifecycleHook,
    participant_runs: u32,
    icydb_row_observed_after_participant: bool,
    canic_row_observed_during_callback: bool,
    icydb_row_observed_during_canic_callback: bool,
    canic_setup_runs: u32,
    canic_install_runs: u32,
    canic_upgrade_runs: u32,
}

impl LifecycleCompositionRecord {
    const fn new(hook: ProbeLifecycleHook) -> Self {
        Self {
            hook,
            participant_runs: 0,
            icydb_row_observed_after_participant: false,
            canic_row_observed_during_callback: false,
            icydb_row_observed_during_canic_callback: false,
            canic_setup_runs: 0,
            canic_install_runs: 0,
            canic_upgrade_runs: 0,
        }
    }
}

thread_local! {
    static COMPOSITION: RefCell<LifecycleCompositionRecord> =
        const { RefCell::new(LifecycleCompositionRecord::new(ProbeLifecycleHook::Init)) };
    static LIFECYCLE_RUNNING: Cell<bool> = const { Cell::new(false) };
}

canic::start!(lifecycle_participant(
    init = crate::after_canic_init,
    post_upgrade = crate::after_canic_post_upgrade,
),);

async fn canic_setup() {
    record_canic_callback(|record| {
        record.canic_setup_runs = record.canic_setup_runs.saturating_add(1);
    });
}

async fn canic_install(_: Option<Vec<u8>>) {
    record_canic_callback(|record| {
        record.canic_install_runs = record.canic_install_runs.saturating_add(1);
    });
}

async fn canic_upgrade() {
    record_canic_callback(|record| {
        record.canic_upgrade_runs = record.canic_upgrade_runs.saturating_add(1);
    });
}

fn after_canic_init() {
    run_icydb_participant(
        ProbeLifecycleHook::Init,
        crate::__icydb_lifecycle_participant::init,
    );
}

fn after_canic_post_upgrade() {
    run_icydb_participant(
        ProbeLifecycleHook::PostUpgrade,
        crate::__icydb_lifecycle_participant::post_upgrade,
    );
    if option_env!("CANIC_TEST_ICYDB_PARTICIPANT_TRAP").is_some() {
        ic_cdk::trap("Canic/IcyDB lifecycle participant requested a test trap");
    }
}

fn run_icydb_participant(hook: ProbeLifecycleHook, participant: fn() -> ()) {
    if LIFECYCLE_RUNNING.replace(true) {
        ic_cdk::trap("Canic/IcyDB lifecycle composition re-entered");
    }
    COMPOSITION.with_borrow_mut(|record| *record = LifecycleCompositionRecord::new(hook));
    participant();
    let icydb_row_observed = timer_row_exists("icydb", "startup", "recovery");
    COMPOSITION.with_borrow_mut(|record| {
        record.participant_runs = record.participant_runs.saturating_add(1);
        record.icydb_row_observed_after_participant = icydb_row_observed;
    });
    LIFECYCLE_RUNNING.set(false);
}

fn record_canic_callback(update: impl FnOnce(&mut LifecycleCompositionRecord)) {
    let canic_row_observed = ["canic:user:init", "canic:user:post_upgrade"]
        .into_iter()
        .any(|name| timer_name_exists("canic", name));
    let icydb_row_observed = timer_row_exists("icydb", "startup", "recovery");
    COMPOSITION.with_borrow_mut(|record| {
        if record.participant_runs != 1 {
            ic_cdk::trap("Canic lifecycle callback ran before the IcyDB participant");
        }
        record.canic_row_observed_during_callback |= canic_row_observed;
        record.icydb_row_observed_during_canic_callback |= icydb_row_observed;
        update(record);
    });
}

fn timer_name_exists(owner: &str, name: &str) -> bool {
    ic_timers::timer_inventory().is_ok_and(|inventory| {
        inventory
            .timers()
            .iter()
            .any(|timer| timer.identity().owner() == owner && timer.identity().name() == name)
    })
}

fn timer_row_exists(owner: &str, subsystem: &str, name: &str) -> bool {
    ic_timers::timer_inventory().is_ok_and(|inventory| {
        inventory.timers().iter().any(|timer| {
            let identity = timer.identity();
            identity.owner() == owner
                && identity.subsystem() == subsystem
                && identity.name() == name
        })
    })
}

fn database_startup() -> ProbeDatabaseStartup {
    match startup_state() {
        Ok(icydb::db::DatabaseStartupState::Ready) => ProbeDatabaseStartup::Ready,
        Ok(icydb::db::DatabaseStartupState::Recovering) => ProbeDatabaseStartup::Recovering,
        Err(_) => ProbeDatabaseStartup::Failed,
    }
}

fn database_ready() -> bool {
    icydb::db::with_request_execution(|| db().map(|_| ())).is_ok()
}

#[ic_cdk::query]
fn lifecycle_composition_snapshot() -> LifecycleCompositionSnapshot {
    let record = COMPOSITION.with_borrow(|record| *record);
    LifecycleCompositionSnapshot {
        hook: record.hook,
        participant_runs: record.participant_runs,
        icydb_row_observed_after_participant: ProbeEvidence::from_observation(
            record.icydb_row_observed_after_participant,
        ),
        icydb_row_live: ProbeEvidence::from_observation(timer_row_exists(
            "icydb", "startup", "recovery",
        )),
        canic_row_observed_during_callback: ProbeEvidence::from_observation(
            record.canic_row_observed_during_callback,
        ),
        icydb_row_observed_during_canic_callback: ProbeEvidence::from_observation(
            record.icydb_row_observed_during_canic_callback,
        ),
        canic_setup_runs: record.canic_setup_runs,
        canic_install_runs: record.canic_install_runs,
        canic_upgrade_runs: record.canic_upgrade_runs,
        database_startup: database_startup(),
        database_access: ProbeEvidence::from_observation(database_ready()),
    }
}

canic::finish!();
