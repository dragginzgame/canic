use super::super::*;

#[test]
fn live_root_status_observation_maps_status_controllers_and_module_hash() {
    let state = sample_install_state("demo", "aaaaa-aa");
    let report = IcpCanisterStatusReport {
        id: "aaaaa-aa".to_string(),
        name: Some("root".to_string()),
        status: "Running".to_string(),
        settings: Some(IcpCanisterStatusSettings {
            controllers: vec!["aaaaa-aa".to_string()],
            compute_allocation: Some("0".to_string()),
            memory_allocation: None,
            freezing_threshold: None,
            reserved_cycles_limit: None,
            wasm_memory_limit: None,
            wasm_memory_threshold: None,
            log_memory_limit: None,
        }),
        module_hash: Some("0xABCD".to_string()),
        memory_size: None,
        cycles: None,
        reserved_cycles: None,
        idle_cycles_burned_per_day: None,
    };

    let observed = observed_root_from_status(&state, &report);

    assert_eq!(observed.canister_id, "aaaaa-aa");
    assert_eq!(
        observed.control_class,
        CanisterControlClassV1::DeploymentControlled
    );
    assert_eq!(observed.controllers, vec!["aaaaa-aa"]);
    assert_eq!(observed.module_hash.as_deref(), Some("abcd"));
    assert_eq!(observed.status.as_deref(), Some("Running"));
    assert_eq!(
        observed.role_assignment_source,
        Some(
            RoleAssignmentSourceV1::IcpCanisterStatus
                .label()
                .to_string()
        )
    );
}

#[test]
fn registry_entries_map_configured_pool_roles_to_observed_pool() {
    let mut gaps = Vec::new();
    let entries = vec![
        RegistryEntry {
            pid: "root-id".to_string(),
            role: Some("root".to_string()),
            parent_pid: None,
            module_hash: None,
        },
        RegistryEntry {
            pid: "shard-id".to_string(),
            role: Some("user_shard".to_string()),
            parent_pid: Some("user_hub-id".to_string()),
            module_hash: Some("module".to_string()),
        },
        RegistryEntry {
            pid: "user_hub-id".to_string(),
            role: Some("user_hub".to_string()),
            parent_pid: Some("root-id".to_string()),
            module_hash: None,
        },
    ];
    let expectations = vec![ConfiguredPoolExpectation {
        pool: "user_shards".to_string(),
        canister_role: "user_shard".to_string(),
    }];

    let observed = registry_entries_to_observed_pool("root-id", &entries, &expectations, &mut gaps);

    assert_eq!(
        observed,
        vec![ObservedPoolCanisterV1 {
            pool: "user_shards".to_string(),
            canister_id: "shard-id".to_string(),
            role: Some("user_shard".to_string()),
            control_class: CanisterControlClassV1::CanicManagedPool,
        }]
    );
    assert!(gaps.is_empty());
}

#[test]
fn registry_entries_map_roles_to_observed_canisters_without_controller_authority() {
    let entries = vec![
        RegistryEntry {
            pid: "root-id".to_string(),
            role: Some("root".to_string()),
            parent_pid: None,
            module_hash: None,
        },
        RegistryEntry {
            pid: "user_hub-id".to_string(),
            role: Some("user_hub".to_string()),
            parent_pid: Some("root-id".to_string()),
            module_hash: Some("0xABCDEF".to_string()),
        },
    ];

    let observed = registry_entries_to_observed_canisters("root-id", &entries);

    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].canister_id, "user_hub-id");
    assert_eq!(observed[0].role.as_deref(), Some("user_hub"));
    assert_eq!(
        observed[0].control_class,
        CanisterControlClassV1::CanicManagedPool
    );
    assert!(observed[0].controllers.is_empty());
    assert_eq!(observed[0].module_hash.as_deref(), Some("abcdef"));
    assert_eq!(
        observed[0].role_assignment_source,
        Some(RoleAssignmentSourceV1::SubnetRegistry.label().to_string())
    );
}

#[test]
fn registry_observation_can_be_enriched_with_live_status() {
    let mut observed = registry_entries_to_observed_canisters(
        "root-id",
        &[RegistryEntry {
            pid: "user_hub-id".to_string(),
            role: Some("user_hub".to_string()),
            parent_pid: Some("root-id".to_string()),
            module_hash: Some("stale".to_string()),
        }],
    )
    .pop()
    .expect("registry observation");
    let report = IcpCanisterStatusReport {
        id: "user_hub-id".to_string(),
        name: Some("user_hub".to_string()),
        status: "Running".to_string(),
        settings: Some(IcpCanisterStatusSettings {
            controllers: vec!["root-id".to_string()],
            compute_allocation: Some("0".to_string()),
            memory_allocation: None,
            freezing_threshold: None,
            reserved_cycles_limit: None,
            wasm_memory_limit: None,
            wasm_memory_threshold: None,
            log_memory_limit: None,
        }),
        module_hash: Some("0xCAFE".to_string()),
        memory_size: None,
        cycles: None,
        reserved_cycles: None,
        idle_cycles_burned_per_day: None,
    };

    apply_live_status_to_registry_observation(&mut observed, &report);

    assert_eq!(
        observed.control_class,
        CanisterControlClassV1::CanicManagedPool
    );
    assert_eq!(observed.controllers, vec!["root-id"]);
    assert_eq!(observed.module_hash.as_deref(), Some("cafe"));
    assert_eq!(observed.status.as_deref(), Some("Running"));
    assert_eq!(
        observed.role_assignment_source,
        Some(
            RoleAssignmentSourceV1::SubnetRegistryAndIcpCanisterStatus
                .label()
                .to_string()
        )
    );
}

#[test]
fn observed_pool_control_uses_enriched_canister_status() {
    let mut observed_pool = vec![ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "shard-id".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    }];
    let observed_canisters = vec![ObservedCanisterV1 {
        canister_id: "shard-id".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::UnknownUnsafe,
        controllers: vec!["external-controller".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("root-id".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some(
            RoleAssignmentSourceV1::SubnetRegistryAndIcpCanisterStatus
                .label()
                .to_string(),
        ),
    }];

    apply_canister_control_to_observed_pool(&mut observed_pool, &observed_canisters);

    assert_eq!(
        observed_pool[0].control_class,
        CanisterControlClassV1::UnknownUnsafe
    );
}

#[test]
fn registry_entries_report_ambiguous_pool_role_mapping() {
    let mut gaps = Vec::new();
    let entries = vec![RegistryEntry {
        pid: "worker-id".to_string(),
        role: Some("worker".to_string()),
        parent_pid: Some("root-id".to_string()),
        module_hash: None,
    }];
    let expectations = vec![
        ConfiguredPoolExpectation {
            pool: "workers_a".to_string(),
            canister_role: "worker".to_string(),
        },
        ConfiguredPoolExpectation {
            pool: "workers_b".to_string(),
            canister_role: "worker".to_string(),
        },
    ];

    let observed = registry_entries_to_observed_pool("root-id", &entries, &expectations, &mut gaps);

    assert!(observed.is_empty());
    assert!(
        gaps.iter()
            .any(|gap| gap.key == "live_subnet_registry.pool.worker")
    );
}
