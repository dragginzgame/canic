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
    install_lifecycle_boundary_fixture, invalid_init_args, upgrade_args,
};
use ic_testkit::pic::{CandidCallExt, CanisterInstallExt, PocketIc, RetryPolicy};
use std::time::Duration;

const INSTALL_CODE_RETRY_LIMIT: usize = 4;
const INSTALL_CODE_COOLDOWN: Duration = Duration::from_mins(5);

#[derive(CandidType)]
enum CanisterStatusRequest {
    Operation(OperationStatusRequest),
    Readiness,
}

#[derive(CandidType, Deserialize)]
enum CanisterStatusResponse {
    Operation(CanisterOperationStatusResponse),
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
    assert_prepared_and_not_ready(&fixture.pic, canic_id);
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

        assert_prepared_and_not_ready(&fixture.pic, canic_id);
        fixture
            .pic
            .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    }
}

fn assert_prepared_and_not_ready(pic: &PocketIc, canister_id: Principal) {
    let status: Result<CanisterStatusResponse, Error> = pic
        .query_candid_as(
            canister_id,
            Principal::from_slice(&[1; 29]),
            CANIC_STATUS,
            (CanisterStatusRequest::Operation(OperationStatusRequest {
                operation_id: [0x43; 32],
            }),),
        )
        .expect("query Prepared Fleet activation status");
    let CanisterStatusResponse::Operation(CanisterOperationStatusResponse::ConfigureRuntime(
        status,
    )) = status.expect("Prepared activation status")
    else {
        panic!("managed Canister returned a differently correlated operation status");
    };
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
