// Category C - Artifact / deployment test (embedded config).
// This test qualifies exact published-IcyDB lifecycle composition in PocketIC.

use candid::{CandidType, Deserialize, Principal};
use canic::{
    Error,
    dto::{
        component_registry::{ComponentRuntimeDirectoryPreparationRequest, ComponentRuntimePhase},
        role::{ComponentRuntimeOperationStatus, OperationReceipt, OperationStatusRequest},
    },
    protocol::{CANIC_COMMAND, CANIC_STATUS},
};
use canic_testing_internal::pic::{
    CanicIcydbLifecycleFixture, icydb_participant_trap_wasm, install_canic_icydb_lifecycle_fixture,
    upgrade_args,
};
use ic_testkit::pic::{CandidCallExt, CanisterInstallExt, PocketIc, RetryPolicy};
use std::time::Duration;

const INSTALL_CODE_RETRY_LIMIT: usize = 4;
const INSTALL_CODE_COOLDOWN: Duration = Duration::from_mins(5);
const STARTUP_PROGRESS_LIMIT: usize = 8;

#[derive(CandidType)]
enum CanisterCommand {
    ConfigureRuntime(ComponentRuntimeDirectoryPreparationRequest),
}

#[derive(CandidType, Deserialize)]
enum CanisterCommandResponse {
    OperationAccepted(OperationReceipt),
}

#[derive(CandidType)]
enum CanisterStatusRequest {
    Operation(OperationStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum CanisterStatusResponse {
    Operation(Box<CanisterOperationStatusResponse>),
}

#[derive(CandidType, Deserialize)]
enum CanisterOperationStatusResponse {
    ConfigureRuntime(ComponentRuntimeOperationStatus),
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
enum ProbeLifecycleHook {
    Init,
    PostUpgrade,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
enum ProbeDatabaseStartup {
    Failed,
    Ready,
    Recovering,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
enum ProbeEvidence {
    Missing,
    Observed,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
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

#[test]
fn managed_canic_and_published_icydb_share_lifecycle_and_timer_custody() {
    let trap_wasm = icydb_participant_trap_wasm();
    let fixture = install_canic_icydb_lifecycle_fixture();
    let (canister_id, directory_request) = fixture.install_composed_canister();

    let installed = composition_snapshot(&fixture.pic, canister_id);
    assert_eq!(installed.hook, ProbeLifecycleHook::Init);
    assert_inactive_participant_reconstructed(&installed);
    assert_eq!(installed.database_startup, ProbeDatabaseStartup::Recovering);
    drive_icydb_startup(&fixture.pic, canister_id);
    assert_prepared(
        &fixture.pic,
        canister_id,
        fixture.root,
        directory_request.operation_id,
    );
    prove_prepared_reconstruction_and_retry(
        &fixture,
        canister_id,
        directory_request.operation_id,
        trap_wasm,
    );

    configure_runtime(
        &fixture.pic,
        canister_id,
        fixture.root,
        directory_request.clone(),
    );
    let activated = wait_for_canic_install_callback(&fixture.pic, canister_id);
    assert_eq!(
        activated.canic_row_observed_during_callback,
        ProbeEvidence::Observed
    );
    assert_eq!(
        activated.icydb_row_observed_during_canic_callback,
        ProbeEvidence::Observed
    );
    assert_eq!(activated.canic_setup_runs, 1);
    assert_eq!(activated.canic_install_runs, 1);
    assert_eq!(activated.canic_upgrade_runs, 0);
    assert_active(
        &fixture.pic,
        canister_id,
        fixture.root,
        directory_request.operation_id,
    );

    fixture
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    upgrade(&fixture.pic, canister_id, &fixture.wasm);
    let active_upgrade = composition_snapshot(&fixture.pic, canister_id);
    assert_eq!(active_upgrade.hook, ProbeLifecycleHook::PostUpgrade);
    assert_eq!(active_upgrade.participant_runs, 1);
    assert_eq!(
        active_upgrade.icydb_row_observed_after_participant,
        ProbeEvidence::Observed
    );
    assert_eq!(active_upgrade.icydb_row_live, ProbeEvidence::Observed);

    let reconstructed = wait_for_canic_upgrade_callback(&fixture.pic, canister_id);
    assert_eq!(
        reconstructed.canic_row_observed_during_callback,
        ProbeEvidence::Observed
    );
    assert_eq!(
        reconstructed.icydb_row_observed_during_canic_callback,
        ProbeEvidence::Observed
    );
    assert_eq!(reconstructed.canic_setup_runs, 1);
    assert_eq!(reconstructed.canic_install_runs, 0);
    assert_eq!(reconstructed.canic_upgrade_runs, 1);
    drive_icydb_startup(&fixture.pic, canister_id);
}

fn prove_prepared_reconstruction_and_retry(
    fixture: &CanicIcydbLifecycleFixture,
    canister_id: Principal,
    operation_id: [u8; 32],
    trap_wasm: Vec<u8>,
) {
    fixture
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    upgrade(&fixture.pic, canister_id, &fixture.wasm);
    let prepared_upgrade = composition_snapshot(&fixture.pic, canister_id);
    assert_eq!(prepared_upgrade.hook, ProbeLifecycleHook::PostUpgrade);
    assert_inactive_participant_reconstructed(&prepared_upgrade);
    assert_prepared(&fixture.pic, canister_id, fixture.root, operation_id);

    fixture
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    let committed_snapshot = composition_snapshot(&fixture.pic, canister_id);
    let committed_module_hash = fixture
        .pic
        .canister_status(canister_id, None)
        .expect("query combined lifecycle Wasm before failed upgrade")
        .module_hash;
    let error = fixture
        .pic
        .upgrade_canister(canister_id, trap_wasm, upgrade_args(), None)
        .expect_err("the IcyDB participant path must trap after restoration");
    assert!(
        error
            .to_string()
            .contains("Canic/IcyDB lifecycle participant requested a test trap"),
        "unexpected combined participant failure: {error}"
    );
    assert_eq!(
        composition_snapshot(&fixture.pic, canister_id),
        committed_snapshot,
        "failed participant upgrade must roll heap and timer changes back"
    );
    assert_eq!(
        fixture
            .pic
            .canister_status(canister_id, None)
            .expect("query combined lifecycle Wasm after failed upgrade")
            .module_hash,
        committed_module_hash,
        "failed post-upgrade must retain the previously committed Wasm"
    );

    fixture
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    upgrade(&fixture.pic, canister_id, &fixture.wasm);
    assert_inactive_participant_reconstructed(&composition_snapshot(&fixture.pic, canister_id));
}

fn composition_snapshot(pic: &PocketIc, canister_id: Principal) -> LifecycleCompositionSnapshot {
    let result: Result<LifecycleCompositionSnapshot, Error> = pic
        .query_candid(canister_id, "lifecycle_composition_snapshot", ())
        .expect("query Canic/IcyDB lifecycle composition snapshot");
    result.expect("read Canic/IcyDB lifecycle composition snapshot")
}

fn assert_inactive_participant_reconstructed(snapshot: &LifecycleCompositionSnapshot) {
    assert_eq!(snapshot.participant_runs, 1);
    assert_eq!(
        snapshot.icydb_row_observed_after_participant,
        ProbeEvidence::Observed
    );
    assert_eq!(snapshot.icydb_row_live, ProbeEvidence::Observed);
    assert_eq!(
        snapshot.canic_row_observed_during_callback,
        ProbeEvidence::Missing
    );
    assert_eq!(
        snapshot.icydb_row_observed_during_canic_callback,
        ProbeEvidence::Missing
    );
    assert_eq!(snapshot.canic_setup_runs, 0);
    assert_eq!(snapshot.canic_install_runs, 0);
    assert_eq!(snapshot.canic_upgrade_runs, 0);
}

fn drive_icydb_startup(pic: &PocketIc, canister_id: Principal) {
    for _ in 0..STARTUP_PROGRESS_LIMIT {
        let snapshot = composition_snapshot(pic, canister_id);
        if snapshot.database_startup == ProbeDatabaseStartup::Ready
            && snapshot.database_access == ProbeEvidence::Observed
        {
            return;
        }
        assert_ne!(snapshot.database_startup, ProbeDatabaseStartup::Failed);
        pic.advance_time(Duration::from_secs(1));
        pic.tick();
        pic.tick();
    }
    panic!(
        "IcyDB startup did not become ready: {:?}",
        composition_snapshot(pic, canister_id)
    );
}

fn upgrade(pic: &PocketIc, canister_id: Principal, wasm: &[u8]) {
    pic.retry_install_code(install_retry_policy(), || {
        pic.upgrade_canister(canister_id, wasm.to_vec(), upgrade_args(), None)
    })
    .expect("same-release combined lifecycle upgrade");
}

fn configure_runtime(
    pic: &PocketIc,
    canister_id: Principal,
    root: Principal,
    request: ComponentRuntimeDirectoryPreparationRequest,
) {
    let operation_id = request.operation_id;
    let result: Result<CanisterCommandResponse, Error> = pic
        .update_candid_as(
            canister_id,
            root,
            CANIC_COMMAND,
            (CanisterCommand::ConfigureRuntime(request),),
        )
        .expect("configure combined managed runtime");
    let CanisterCommandResponse::OperationAccepted(receipt) =
        result.expect("combined managed runtime activation accepted");
    assert_eq!(receipt.operation_id, operation_id);
}

fn wait_for_canic_install_callback(
    pic: &PocketIc,
    canister_id: Principal,
) -> LifecycleCompositionSnapshot {
    wait_for_canic_callback(pic, canister_id, |snapshot| {
        snapshot.canic_install_runs == 1
    })
}

fn wait_for_canic_upgrade_callback(
    pic: &PocketIc,
    canister_id: Principal,
) -> LifecycleCompositionSnapshot {
    wait_for_canic_callback(pic, canister_id, |snapshot| {
        snapshot.canic_upgrade_runs == 1
    })
}

fn wait_for_canic_callback(
    pic: &PocketIc,
    canister_id: Principal,
    complete: impl Fn(&LifecycleCompositionSnapshot) -> bool,
) -> LifecycleCompositionSnapshot {
    for _ in 0..8 {
        let snapshot = composition_snapshot(pic, canister_id);
        if complete(&snapshot) {
            return snapshot;
        }
        pic.tick();
    }
    panic!(
        "Canic lifecycle callback did not run: {:?}",
        composition_snapshot(pic, canister_id)
    );
}

fn assert_prepared(
    pic: &PocketIc,
    canister_id: Principal,
    root: Principal,
    operation_id: [u8; 32],
) {
    assert_runtime_phase(
        pic,
        canister_id,
        root,
        operation_id,
        ComponentRuntimePhase::AwaitingDirectory,
    );
}

fn assert_active(pic: &PocketIc, canister_id: Principal, root: Principal, operation_id: [u8; 32]) {
    assert_runtime_phase(
        pic,
        canister_id,
        root,
        operation_id,
        ComponentRuntimePhase::Active,
    );
}

fn assert_runtime_phase(
    pic: &PocketIc,
    canister_id: Principal,
    root: Principal,
    operation_id: [u8; 32],
    expected: ComponentRuntimePhase,
) {
    let result: Result<CanisterStatusResponse, Error> = pic
        .query_candid_as(
            canister_id,
            root,
            CANIC_STATUS,
            (CanisterStatusRequest::Operation(OperationStatusRequest {
                operation_id,
            }),),
        )
        .expect("query combined managed runtime status");
    let CanisterStatusResponse::Operation(operation) =
        result.expect("combined managed runtime operation status");
    let CanisterOperationStatusResponse::ConfigureRuntime(status) = *operation;
    assert_eq!(status.runtime.phase, expected);
}

fn install_retry_policy() -> RetryPolicy {
    RetryPolicy::try_new(INSTALL_CODE_RETRY_LIMIT, INSTALL_CODE_COOLDOWN)
        .expect("install retry policy")
}
