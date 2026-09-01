// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use candid::{CandidType, Deserialize, Principal};
use canic::{
    Error,
    dto::{
        fleet_activation::FleetActivationPhase,
        role::{ComponentRuntimeOperationStatus, OperationStatusRequest},
        runtime::{CanicReadinessStatus, ReadinessStatus},
    },
    protocol::CANIC_STATUS,
};
use canic_testing_internal::pic::{
    install_lifecycle_boundary_fixture, invalid_init_args, lifecycle_participant_init_trap_wasm,
    lifecycle_participant_trap_wasm, managed_test_init_identity, upgrade_args,
};
use ic_testkit::pic::{CandidCallExt, CanisterInstallExt, PocketIc, RetryPolicy};
use std::{any::Any, time::Duration};

const INSTALL_CODE_RETRY_LIMIT: usize = 4;
const INSTALL_CODE_COOLDOWN: Duration = Duration::from_mins(5);

#[derive(CandidType)]
enum CanisterStatusRequest {
    Operation(OperationStatusRequest),
    Readiness,
}

#[derive(CandidType, Deserialize)]
enum CanisterStatusResponse {
    Operation(Box<CanisterOperationStatusResponse>),
    Readiness(CanicReadinessStatus),
}

#[derive(CandidType, Deserialize)]
enum CanisterOperationStatusResponse {
    ConfigureRuntime(ComponentRuntimeOperationStatus),
}

#[test]
fn lifecycle_boundary_traps_are_phase_correct() {
    let fixture = install_lifecycle_boundary_fixture();
    let install = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        fixture.install_canic_canister()
    }));
    assert!(install.is_ok(), "install panicked for canic canister");
    let canic_id = install.expect("install must return the canister id");
    fixture
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);

    let reinstall_err = fixture
        .pic
        .retry_install_code(install_retry_policy(), || {
            fixture.pic.reinstall_canister(
                canic_id,
                fixture.canic_wasm.clone(),
                invalid_init_args(),
                None,
            )
        })
        .expect_err("reinstall should fail");
    assert_phase_error("init", &reinstall_err);

    let authority_id = fixture.install_authority_canister();
    fixture
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);

    let upgrade_err = fixture
        .pic
        .retry_install_code(install_retry_policy(), || {
            fixture.pic.upgrade_canister(
                authority_id,
                fixture.canic_wasm.clone(),
                upgrade_args(),
                None,
            )
        })
        .expect_err("upgrade should fail");
    assert_phase_error("post_upgrade", &upgrade_err);
}

#[test]
fn prepared_non_root_remains_fenced_across_repeated_upgrades() {
    let fixture = install_lifecycle_boundary_fixture();
    let canic_id = fixture.install_canic_canister();
    assert_prepared_and_not_ready(&fixture.pic, canic_id, fixture.root);
    fixture
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);

    for attempt in 1..=3 {
        fixture
            .pic
            .retry_install_code(install_retry_policy(), || {
                fixture.pic.upgrade_canister(
                    canic_id,
                    fixture.canic_wasm.clone(),
                    upgrade_args(),
                    None,
                )
            })
            .unwrap_or_else(|err| panic!("upgrade attempt {attempt} should succeed: {err}"));

        assert_prepared_and_not_ready(&fixture.pic, canic_id, fixture.root);
        fixture
            .pic
            .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    }
}

#[test]
fn managed_cross_release_upgrade_fails_before_runtime_can_reuse_the_initial_release_set() {
    let fixture = install_lifecycle_boundary_fixture();
    let canic_id = fixture.install_canic_canister();
    let committed_module_hash = fixture
        .pic
        .canister_status(canic_id, None)
        .expect("query managed Wasm before cross-release upgrade")
        .module_hash;
    let retained_release_build_id = managed_test_init_identity().release_build_id.to_string();
    let foreign_wasm = replace_release_build_id(
        &fixture.canic_wasm,
        &retained_release_build_id,
        &"22".repeat(32),
    );
    fixture
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);

    fixture
        .pic
        .upgrade_canister(canic_id, foreign_wasm, upgrade_args(), None)
        .expect_err("managed cross-release upgrade must fail closed");
    assert_prepared_and_not_ready(&fixture.pic, canic_id, fixture.root);
    assert_eq!(
        fixture
            .pic
            .canister_status(canic_id, None)
            .expect("query managed Wasm after rejected cross-release upgrade")
            .module_hash,
        committed_module_hash,
        "failed cross-release upgrade must retain the committed same-release Wasm"
    );

    fixture
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    fixture
        .pic
        .retry_install_code(install_retry_policy(), || {
            fixture
                .pic
                .upgrade_canister(canic_id, fixture.canic_wasm.clone(), upgrade_args(), None)
        })
        .expect("same-release upgrade must remain valid after rejection");
    assert_prepared_and_not_ready(&fixture.pic, canic_id, fixture.root);
}

#[test]
fn lifecycle_participant_trap_rolls_back_before_corrected_retry() {
    let trap_wasm = lifecycle_participant_trap_wasm();
    let fixture = install_lifecycle_boundary_fixture();
    let canic_id = fixture.install_canic_canister();
    let committed_module_hash = fixture
        .pic
        .canister_status(canic_id, None)
        .expect("query committed managed Wasm before failed upgrade")
        .module_hash;
    fixture
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);

    let error = fixture
        .pic
        .upgrade_canister(canic_id, trap_wasm, upgrade_args(), None)
        .expect_err("the test lifecycle participant must trap");
    assert!(
        error
            .to_string()
            .contains("managed lifecycle participant requested a test trap"),
        "unexpected lifecycle participant failure: {error}"
    );
    assert_prepared_and_not_ready(&fixture.pic, canic_id, fixture.root);
    assert_eq!(
        fixture
            .pic
            .canister_status(canic_id, None)
            .expect("query managed Wasm after failed upgrade")
            .module_hash,
        committed_module_hash,
        "failed post-upgrade must retain the previously committed Wasm"
    );

    fixture
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    fixture
        .pic
        .retry_install_code(install_retry_policy(), || {
            fixture
                .pic
                .upgrade_canister(canic_id, fixture.canic_wasm.clone(), upgrade_args(), None)
        })
        .expect("corrected lifecycle participant retry should succeed");
    assert_prepared_and_not_ready(&fixture.pic, canic_id, fixture.root);
}

#[test]
fn init_participant_trap_leaves_empty_canister_before_corrected_retry() {
    let trap_wasm = lifecycle_participant_init_trap_wasm();
    let fixture = install_lifecycle_boundary_fixture();
    let uninstalled = fixture.create_uninstalled_canic_canister();

    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        fixture.pic.install_canister(
            uninstalled.canister_id,
            trap_wasm,
            uninstalled.init_args.clone(),
            None,
        );
    }))
    .expect_err("the init lifecycle participant must trap");
    let failure = panic_message(failure.as_ref());
    assert!(
        failure.contains("managed init lifecycle participant requested a test trap"),
        "unexpected init lifecycle participant failure: {failure}"
    );
    assert_eq!(
        fixture
            .pic
            .canister_status(uninstalled.canister_id, None)
            .expect("query empty canister after failed install")
            .module_hash,
        None,
        "failed init must leave the canister without a committed module"
    );

    fixture.pic.tick();
    assert_eq!(
        fixture
            .pic
            .canister_status(uninstalled.canister_id, None)
            .expect("query empty canister after a later round")
            .module_hash,
        None,
        "deferred work must not commit after a failed init"
    );

    fixture
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    fixture.pic.install_canister(
        uninstalled.canister_id,
        fixture.canic_wasm.clone(),
        uninstalled.init_args,
        None,
    );
    assert_prepared_and_not_ready(&fixture.pic, uninstalled.canister_id, fixture.root);
}

fn assert_prepared_and_not_ready(pic: &PocketIc, canister_id: Principal, root: Principal) {
    let status: Result<CanisterStatusResponse, Error> = pic
        .query_candid_as(
            canister_id,
            root,
            CANIC_STATUS,
            (CanisterStatusRequest::Operation(OperationStatusRequest {
                operation_id: [0x43; 32],
            }),),
        )
        .expect("query Prepared Fleet activation status");
    let CanisterStatusResponse::Operation(operation) = status.expect("Prepared activation status")
    else {
        panic!("managed Canister returned a differently correlated operation status");
    };
    let CanisterOperationStatusResponse::ConfigureRuntime(status) = *operation;
    assert_eq!(
        status.fleet_activation.phase,
        FleetActivationPhase::Prepared
    );
    let readiness: Result<CanisterStatusResponse, Error> = pic
        .query_candid(
            canister_id,
            CANIC_STATUS,
            (CanisterStatusRequest::Readiness,),
        )
        .expect("query managed Canister readiness");
    let CanisterStatusResponse::Readiness(readiness) =
        readiness.expect("Prepared readiness status")
    else {
        panic!("managed Canister returned a differently correlated readiness status");
    };
    assert_ne!(readiness.status, ReadinessStatus::Ready);
}

#[test]
fn non_root_post_upgrade_failure_reports_phase_error() {
    let fixture = install_lifecycle_boundary_fixture();
    let authority_id = fixture.install_authority_canister();
    fixture
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);

    let upgrade_err = fixture
        .pic
        .retry_install_code(install_retry_policy(), || {
            fixture.pic.upgrade_canister(
                authority_id,
                fixture.canic_wasm.clone(),
                upgrade_args(),
                None,
            )
        })
        .expect_err("upgrade should fail for non-canic stable state");

    assert_phase_error("post_upgrade", &upgrade_err);
}

fn install_retry_policy() -> RetryPolicy {
    RetryPolicy::try_new(INSTALL_CODE_RETRY_LIMIT, INSTALL_CODE_COOLDOWN)
        .expect("install retry policy")
}

fn replace_release_build_id(wasm: &[u8], retained: &str, replacement: &str) -> Vec<u8> {
    assert_eq!(retained.len(), replacement.len());
    assert_ne!(retained, replacement);
    let retained = retained.as_bytes();
    let replacement = replacement.as_bytes();
    let offsets = wasm
        .windows(retained.len())
        .enumerate()
        .filter_map(|(offset, bytes)| (bytes == retained).then_some(offset))
        .collect::<Vec<_>>();
    assert!(
        !offsets.is_empty(),
        "managed Wasm must contain its embedded release-build identity"
    );

    let mut foreign = wasm.to_vec();
    for offset in offsets {
        foreign[offset..offset + replacement.len()].copy_from_slice(replacement);
    }
    foreign
}

fn assert_phase_error(phase: &str, err: &impl ToString) {
    let message = err.to_string();
    assert!(
        message.contains(&format!("{phase}:")),
        "missing {phase} prefix: {message}"
    );
    assert!(
        !message.contains("Internal"),
        "unexpected internal error: {message}"
    );
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            payload
                .downcast_ref::<&str>()
                .map(|message| (*message).to_string())
        })
        .unwrap_or_else(|| "non-string panic payload".to_string())
}
