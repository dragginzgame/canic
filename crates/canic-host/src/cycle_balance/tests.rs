use super::*;

fn binding() -> crate::protocol_binding::ResolvedProtocolBinding {
    crate::protocol_binding::ResolvedProtocolBinding {
        binding: crate::protocol_binding::RegistryProtocolBinding {
            release_identity: "test".to_string(),
            role: canic_core::ids::CanisterRole::new("test"),
            capabilities: std::collections::BTreeSet::new(),
            candid_sha256: [1; 32],
            protocol_profile_digest: canic_core::role_contract::ProtocolProfileDigest::from_bytes(
                [2; 32],
            ),
        },
        candid_path: std::path::PathBuf::from("test.did"),
    }
}

#[test]
fn local_target_preserves_direct_replica_failure() {
    let icp = IcpCli::new("unused-icp-command", Some("local".to_string()));
    let error = query_cycle_balance(&icp, "not a principal", "local", None, &binding())
        .expect_err("local target must report its direct replica failure");

    assert!(matches!(
        error,
        CycleBalanceQueryError::Replica(ReplicaQueryError::Query(_))
    ));
}

#[test]
fn non_local_target_preserves_icp_command_failure() {
    let icp = IcpCli::new("/canic-test/missing-icp", Some("ic".to_string()));
    let error = query_cycle_balance(&icp, "not a principal", "ic", None, &binding())
        .expect_err("non-local target must report its ICP command failure");

    assert!(matches!(
        error,
        CycleBalanceQueryError::Icp(IcpCommandError::MissingCli { .. })
    ));
}
