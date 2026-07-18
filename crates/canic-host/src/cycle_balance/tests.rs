use super::*;

#[test]
fn local_target_preserves_direct_replica_failure() {
    let icp = IcpCli::new("unused-icp-command", Some("local".to_string()));
    let error = query_cycle_balance(&icp, "not a principal", "local", None, None)
        .expect_err("local target must report its direct replica failure");

    assert!(matches!(
        error,
        CycleBalanceQueryError::Replica(ReplicaQueryError::Query(_))
    ));
}

#[test]
fn non_local_target_preserves_icp_command_failure() {
    let icp = IcpCli::new("/canic-test/missing-icp", Some("ic".to_string()));
    let error = query_cycle_balance(&icp, "not a principal", "ic", None, None)
        .expect_err("non-local target must report its ICP command failure");

    assert!(matches!(
        error,
        CycleBalanceQueryError::Icp(IcpCommandError::MissingCli { .. })
    ));
}
