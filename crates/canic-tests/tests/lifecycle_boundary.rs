// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use canic_testing_internal::pic::{
    install_lifecycle_boundary_fixture, invalid_init_args, upgrade_args,
};
use std::time::Duration;

const READY_TICK_LIMIT: usize = 120;
const INSTALL_CODE_RETRY_LIMIT: usize = 4;
const INSTALL_CODE_COOLDOWN: Duration = Duration::from_secs(5 * 60);

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
        .reinstall_canister(
            canic_id,
            fixture.canic_wasm.clone(),
            invalid_init_args(),
            None,
        )
        .map_err(|err| err.to_string());
    let reinstall_err = fixture
        .pic
        .retry_install_code_err(
            INSTALL_CODE_RETRY_LIMIT,
            INSTALL_CODE_COOLDOWN,
            reinstall_err,
            || {
                fixture
                    .pic
                    .reinstall_canister(
                        canic_id,
                        fixture.canic_wasm.clone(),
                        invalid_init_args(),
                        None,
                    )
                    .map_err(|err| err.to_string())
            },
        )
        .expect_err("reinstall should fail");
    assert_phase_error("init", &reinstall_err);

    let authority_id = fixture.install_authority_canister();
    fixture
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);

    let upgrade_err = fixture
        .pic
        .upgrade_canister(
            authority_id,
            fixture.canic_wasm.clone(),
            upgrade_args(),
            None,
        )
        .map_err(|err| err.to_string());
    let upgrade_err = fixture
        .pic
        .retry_install_code_err(
            INSTALL_CODE_RETRY_LIMIT,
            INSTALL_CODE_COOLDOWN,
            upgrade_err,
            || {
                fixture
                    .pic
                    .upgrade_canister(
                        authority_id,
                        fixture.canic_wasm.clone(),
                        upgrade_args(),
                        None,
                    )
                    .map_err(|err| err.to_string())
            },
        )
        .expect_err("upgrade should fail");
    assert_phase_error("post_upgrade", &upgrade_err);
}

#[test]
fn non_root_post_upgrade_remains_ready_across_repeated_upgrades() {
    let fixture = install_lifecycle_boundary_fixture();
    let canic_id = fixture.install_canic_canister();
    fixture
        .pic
        .wait_for_ready(canic_id, READY_TICK_LIMIT, "install");
    fixture
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);

    for attempt in 1..=3 {
        fixture
            .pic
            .retry_install_code_ok(INSTALL_CODE_RETRY_LIMIT, INSTALL_CODE_COOLDOWN, || {
                fixture
                    .pic
                    .upgrade_canister(canic_id, fixture.canic_wasm.clone(), upgrade_args(), None)
                    .map_err(|err| err.to_string())
            })
            .unwrap_or_else(|err| panic!("upgrade attempt {attempt} should succeed: {err}"));

        fixture
            .pic
            .wait_for_ready(canic_id, READY_TICK_LIMIT, "post_upgrade");
        fixture
            .pic
            .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    }
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
        .upgrade_canister(
            authority_id,
            fixture.canic_wasm.clone(),
            upgrade_args(),
            None,
        )
        .map_err(|err| err.to_string());
    let upgrade_err = fixture
        .pic
        .retry_install_code_err(
            INSTALL_CODE_RETRY_LIMIT,
            INSTALL_CODE_COOLDOWN,
            upgrade_err,
            || {
                fixture
                    .pic
                    .upgrade_canister(
                        authority_id,
                        fixture.canic_wasm.clone(),
                        upgrade_args(),
                        None,
                    )
                    .map_err(|err| err.to_string())
            },
        )
        .expect_err("upgrade should fail for non-canic stable state");

    assert_phase_error("post_upgrade", &upgrade_err);
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
