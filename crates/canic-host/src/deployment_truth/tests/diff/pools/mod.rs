use super::super::*;

#[test]
fn deployment_diff_blocks_missing_expected_pool_canister() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let inventory = sample_matching_inventory();

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "pool_canister_missing")
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "pool_canister"
            && item.subject == "user_shards:user_shard"
            && item.expected.as_deref() == Some("pool-canister")
            && item.observed.is_none()
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_planned_pool_subject() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-a".to_string()),
        role: Some("user_shard".to_string()),
    });
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-b".to_string()),
        role: Some("user_shard".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "planned_pool_conflict"
                && finding.subject.as_deref() == Some("user_shards:user_shard"))
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "planned_pool_conflict"
            && item.subject == "user_shards:user_shard"
            && item
                .observed
                .as_deref()
                .is_some_and(|observed| observed.contains("pool-a") && observed.contains("pool-b"))
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_blocks_conflicting_planned_pool_id() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "directory".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("project_instance".to_string()),
    });

    let diff = compare_plan_to_inventory(&plan, &sample_matching_inventory());

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "planned_pool_id_conflict"
                && finding.subject.as_deref() == Some("pool-canister"))
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "planned_pool_id_conflict"
            && item.subject == "pool-canister"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("directory:project_instance")
                    && observed.contains("user_shards:user_shard")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_duplicate_identical_planned_pool() {
    let mut plan = sample_plan();
    let planned = ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    };
    plan.expected_pool.push(planned.clone());
    plan.expected_pool.push(planned);
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "duplicate_planned_pool"
                && finding.subject.as_deref() == Some("user_shards:user_shard"))
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "planned_pool_duplicate"
            && item.subject == "user_shards:user_shard"
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_blocks_unsafe_pool_control_class() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::UserControlled,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "unsafe_pool_control_class")
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "pool_control_class"
            && item.subject == "user_shards:user_shard"
            && item.expected.as_deref() == Some("CanicManagedPool")
            && item.observed.as_deref() == Some("UserControlled")
    }));
}

#[test]
fn deployment_diff_blocks_pool_canister_id_mismatch() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("planned-pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "observed-pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "pool_canister_id_mismatch")
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "pool_canister_id"
            && item.subject == "user_shards:user_shard"
            && item.expected.as_deref() == Some("planned-pool-canister")
            && item.observed.as_deref() == Some("observed-pool-canister")
    }));
    assert!(
        diff.warnings
            .iter()
            .all(|finding| finding.code != "extra_pool_canister_observed")
    );
}

#[test]
fn deployment_diff_blocks_conflicting_pool_identities_for_same_canister_id() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "directory".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("project_instance".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "pool_canister_id_conflict"
                && finding.subject.as_deref() == Some("pool-canister"))
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "pool_canister_id_conflict"
            && item.subject == "pool-canister"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("directory:project_instance")
                    && observed.contains("user_shards:user_shard")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_exact_duplicate_pool_observation() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    let observed = ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    };
    inventory.observed_pool.push(observed.clone());
    inventory.observed_pool.push(observed);

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "duplicate_pool_canister_observed"
                && finding.subject.as_deref() == Some("pool-canister"))
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "pool_canister_duplicate"
            && item.subject == "pool-canister"
            && item.expected.as_deref() == Some("user_shards:user_shard")
            && item.observed.as_deref() == Some("2")
            && item.severity == SafetySeverityV1::Warning
    }));
}

#[test]
fn deployment_diff_blocks_cross_surface_role_conflict_for_same_canister_id() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: Some("shared-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "shared-canister".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::DeploymentControlled,
        controllers: vec!["aaaaa-aa".to_string()],
        module_hash: Some("module".to_string()),
        status: Some("Running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("subnet_registry+icp_canister_status".to_string()),
    });
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user_shards".to_string(),
        canister_id: "shared-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Blocked);
    assert!(
        diff.hard_failures
            .iter()
            .any(|finding| finding.code == "canister_pool_role_conflict"
                && finding.subject.as_deref() == Some("shared-canister"))
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "canister_pool_role_conflict"
            && item.subject == "shared-canister"
            && item.observed.as_deref().is_some_and(|observed| {
                observed.contains("canister=user_hub")
                    && observed.contains("pool=user_shards:user_shard")
            })
            && item.severity == SafetySeverityV1::HardFailure
    }));
}

#[test]
fn deployment_diff_warns_for_extra_pool_canister() {
    let plan = sample_plan();
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "directory".to_string(),
        canister_id: "extra-pool-canister".to_string(),
        role: Some("project_instance".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });

    let diff = compare_plan_to_inventory(&plan, &inventory);

    assert_eq!(diff.resume_safety.status, SafetyStatusV1::Warning);
    assert!(diff.hard_failures.is_empty());
    assert!(
        diff.warnings
            .iter()
            .any(|finding| finding.code == "extra_pool_canister_observed")
    );
    assert!(diff.pool_diff.iter().any(|item| {
        item.category == "pool_extra"
            && item.subject == "directory:project_instance"
            && item.observed.as_deref() == Some("extra-pool-canister")
            && item.severity == SafetySeverityV1::Warning
    }));
}
