use super::*;

#[test]
fn raw_receivers_do_not_require_fleet_state() {
    let target = target_options();
    for receiver in ["aaaaa-aa", "external-ledger-account"] {
        assert_eq!(
            resolve(&target, Path::new("/missing-canic-workspace"), receiver)
                .expect("preserve external receiver"),
            receiver
        );
    }
}

#[test]
fn compact_targets_require_exactly_two_nonempty_parts() {
    assert_eq!(
        split_fleet_target("demo/app").unwrap(),
        Some(("demo", "app"))
    );
    for receiver in ["/app", "demo/", "/", "demo/app/extra"] {
        std::assert_matches!(
            resolve(
                &target_options(),
                Path::new("/missing-canic-workspace"),
                receiver
            ),
            Err(FleetRecipientError::InvalidRecipient)
        );
    }
}

#[test]
fn fleet_lookup_preserves_selected_environment_and_identity() {
    std::assert_matches!(
        resolve(&target_options(), Path::new("/missing-canic-workspace"), "demo/app"),
        Err(FleetRecipientError::CurrentFleet(CurrentFleetInventoryError::NotConverged {
            environment,
            fleet,
        })) if environment == "fixture" && fleet == "demo"
    );
}

#[test]
fn resolves_root_principals_and_unique_roles() {
    let registry = vec![registry_entry("child-principal", "app")];
    for name in ["root", "root-principal"] {
        assert_eq!(
            resolve_canister_or_role("demo", name, "root-principal", &registry).unwrap(),
            "root-principal"
        );
    }
    for name in ["child-principal", "app"] {
        assert_eq!(
            resolve_canister_or_role("demo", name, "root-principal", &registry).unwrap(),
            "child-principal"
        );
    }
}

#[test]
fn ambiguous_roles_require_an_exact_principal_and_unknown_targets_fail() {
    let registry = vec![
        registry_entry("shard-a", "app"),
        registry_entry("shard-b", "app"),
    ];
    std::assert_matches!(
        resolve_canister_or_role("demo", "app", "root-principal", &registry),
        Err(FleetRecipientError::AmbiguousRole { fleet, role })
            if fleet == "demo" && role == "app"
    );
    std::assert_matches!(
        resolve_canister_or_role("demo", "unknown", "root-principal", &registry),
        Err(FleetRecipientError::UnknownTarget { fleet, target })
            if fleet == "demo" && target == "unknown"
    );
    assert_eq!(
        resolve_canister_or_role("demo", "shard-b", "root-principal", &registry).unwrap(),
        "shard-b"
    );
}

fn target_options() -> IcpTargetOptions {
    IcpTargetOptions {
        environment: "fixture".to_string(),
        icp: "icp".to_string(),
    }
}

fn registry_entry(pid: &str, role: &str) -> RegistryEntry {
    RegistryEntry {
        pid: pid.to_string(),
        role: Some(role.to_string()),
        parent_pid: None,
        module_hash: None,
        protocol_binding: None,
    }
}
