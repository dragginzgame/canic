use super::*;

fn report(id: &str, status: &str) -> IcpCanisterStatusReport {
    IcpCanisterStatusReport {
        id: id.to_string(),
        name: None,
        status: status.to_string(),
        settings: None,
        module_hash: None,
        memory_size: None,
        cycles: None,
        reserved_cycles: None,
        idle_cycles_burned_per_day: None,
    }
}

#[test]
fn backup_status_mapping_accepts_exact_lifecycle_states() {
    for (status, expected) in [
        ("Running", BackupRunnerCanisterStatus::Running),
        ("Stopped", BackupRunnerCanisterStatus::Stopped),
        ("Stopping", BackupRunnerCanisterStatus::Stopping),
    ] {
        assert_eq!(
            runner_canister_status("aaaaa-aa", &report("aaaaa-aa", status))
                .expect("map exact lifecycle state"),
            expected
        );
    }
}

#[test]
fn backup_status_mapping_rejects_wrong_identity_and_unknown_state() {
    let wrong_id = runner_canister_status("aaaaa-aa", &report("2vxsx-fae", "Stopped"))
        .expect_err("wrong canister identity must fail");
    let unknown = runner_canister_status("aaaaa-aa", &report("aaaaa-aa", "Deleted"))
        .expect_err("unknown lifecycle state must fail");

    assert_eq!(wrong_id.status, "icp-status");
    assert_eq!(unknown.status, "icp-status");
}
