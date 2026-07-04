// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use canic::{
    cdk::types::Principal,
    dto::{
        canister::CanisterInfo,
        env::EnvSnapshotResponse,
        error::ErrorCode,
        log::LogEntry,
        page::{Page, PageRequest},
        runtime::{
            CanicHealthStatus, CanicReadinessStatus, CanicRuntimeStatus, HealthStatus,
            RUNTIME_INTROSPECTION_SCHEMA_VERSION, ReadinessStatus, RuntimeFieldVisibility,
            RuntimeStatus,
        },
        state::{AppStateResponse, SubnetStateResponse},
        topology::AppRegistryResponse,
    },
    ids::{CanisterRole, SubnetRole},
    protocol,
};
use ic_testkit::pic::Pic;

// Query the authoritative root subnet registry once and unwrap the canonical response shape.
fn query_subnet_registry(
    pic: &Pic,
    root_id: Principal,
) -> Vec<canic::dto::topology::SubnetRegistryEntry> {
    let registry: Result<canic::dto::topology::SubnetRegistryResponse, canic::Error> =
        pic.query_call_or_panic(root_id, protocol::CANIC_SUBNET_REGISTRY, ());
    registry.expect("query registry application").0
}

/// Assert that the registry contains the expected roles with the expected parents.
///
/// # Panics
///
/// Panics if the registry query fails, if an expected role is missing, or if a
/// registry parent does not match the expected parent.
pub fn assert_registry_parents(
    pic: &Pic,
    root_id: Principal,
    expected: &[(CanisterRole, Option<Principal>)],
) {
    let registry = query_subnet_registry(pic, root_id);

    for (role, expected_parent) in expected {
        let entry = registry
            .iter()
            .find(|entry| &entry.role == role)
            .unwrap_or_else(|| panic!("missing {role} entry in registry"));

        assert_eq!(
            entry.record.parent_pid, *expected_parent,
            "unexpected parent for {role}"
        );
    }
}

/// Assert that a child canister exposes a correct EnvSnapshotResponse.
///
/// # Panics
///
/// Panics if the env query fails or if the returned environment fields do not
/// match the expected role, parent, root, prime root, or subnet data.
pub fn assert_child_env(
    pic: &Pic,
    child_pid: Principal,
    role: CanisterRole,
    expected_parent_id: Principal,
    root_id: Principal,
) {
    let env: Result<EnvSnapshotResponse, canic::Error> =
        pic.query_call_or_panic(child_pid, protocol::CANIC_ENV, ());
    let env = env.expect("query env application");

    assert_eq!(
        env.canister_role,
        Some(role.clone()),
        "env canister role for {role}"
    );
    assert_eq!(
        env.parent_pid,
        Some(expected_parent_id),
        "env parent for {role}"
    );
    assert_eq!(env.root_pid, Some(root_id), "env root for {role}");
    assert_eq!(
        env.prime_root_pid,
        Some(root_id),
        "env prime root for {role}"
    );
    assert_eq!(
        env.subnet_role,
        Some(SubnetRole::PRIME),
        "env subnet role for {role}"
    );
    assert!(
        env.subnet_pid.is_some(),
        "env subnet pid should be set for {role}"
    );
}

/// Assert that every registered child exposes env fields matching the registry.
///
/// # Panics
///
/// Panics if the registry query fails, if a registered non-root child has no
/// parent, or if any child environment does not match the registry.
pub fn assert_child_envs_match_registry(pic: &Pic, root_id: Principal) {
    let registry = query_subnet_registry(pic, root_id);

    for entry in registry {
        if entry.role.is_root() || entry.role.is_wasm_store() {
            continue;
        }

        let expected_parent_id = entry.record.parent_pid.unwrap_or_else(|| {
            panic!(
                "registered non-root canister {} ({}) must have a parent",
                entry.pid, entry.role
            )
        });

        assert_child_env(pic, entry.pid, entry.role, expected_parent_id, root_id);
    }
}

/// Assert that the CANIC_CANISTER_CHILDREN endpoint matches the registry.
///
/// # Panics
///
/// Panics if registry or child-list queries fail, if the registry has no root
/// children, or if the endpoint response differs from the registry projection.
pub fn assert_children_match_registry(pic: &Pic, root_id: Principal) {
    // 1. Query authoritative registry
    let registry = query_subnet_registry(pic, root_id);

    // 2. Build expected children from registry (topology-only)
    let mut expected: Vec<CanisterInfo> = registry
        .iter()
        .filter(|entry| entry.record.parent_pid == Some(root_id))
        .map(|entry| CanisterInfo {
            pid: entry.pid,
            role: entry.role.clone(),
            parent_pid: entry.record.parent_pid,
            module_hash: None, // ignored for topology comparison
            created_at: 0,     // ignored for topology comparison
        })
        .collect();

    assert!(
        !expected.is_empty(),
        "registry should contain root children"
    );

    // 3. Query children endpoint
    let page: Result<Page<CanisterInfo>, canic::Error> = pic.query_call_or_panic(
        root_id,
        protocol::CANIC_CANISTER_CHILDREN,
        (PageRequest {
            limit: 100,
            offset: 0,
        },),
    );
    let mut page = page.expect("query canister children application");

    // 4. Normalize actual entries (ignore lifecycle metadata)
    for entry in &mut page.entries {
        entry.module_hash = None;
        entry.created_at = 0;
    }

    // 5. Normalize ordering (endpoint order is not significant)
    expected.sort_by(|a, b| a.role.cmp(&b.role));
    page.entries.sort_by(|a, b| a.role.cmp(&b.role));

    // 6. Assert invariants
    assert_eq!(page.total, expected.len() as u64, "reported total mismatch");

    assert_eq!(
        page.entries, expected,
        "child list from endpoint must match registry"
    );
}

/// Assert that root serves state snapshots and ordinary children do not export them.
///
/// # Panics
///
/// Panics if root state queries fail, if non-controller root state queries are
/// accepted, or if ordinary children expose root-only state endpoints.
pub fn assert_state_endpoints_are_root_only(pic: &Pic, root_id: Principal, child_pid: Principal) {
    let app_state: Result<AppStateResponse, canic::Error> =
        pic.query_call_or_panic(root_id, protocol::CANIC_APP_STATE, ());
    app_state.expect("root app state application");

    let subnet_state: Result<SubnetStateResponse, canic::Error> =
        pic.query_call_or_panic(root_id, protocol::CANIC_SUBNET_STATE, ());
    subnet_state.expect("root subnet state application");

    let non_controller = Principal::from_slice(&[251; 29]);
    let denied_app_state: Result<AppStateResponse, canic::Error> =
        pic.query_call_as_or_panic(root_id, non_controller, protocol::CANIC_APP_STATE, ());
    let Err(denied_app_state) = denied_app_state else {
        panic!("non-controller app state query must be denied")
    };
    assert_eq!(denied_app_state.code, ErrorCode::Unauthorized);

    let denied_subnet_state: Result<SubnetStateResponse, canic::Error> =
        pic.query_call_as_or_panic(root_id, non_controller, protocol::CANIC_SUBNET_STATE, ());
    let Err(denied_subnet_state) = denied_subnet_state else {
        panic!("non-controller subnet state query must be denied")
    };
    assert_eq!(denied_subnet_state.code, ErrorCode::Unauthorized);

    let child_app_state: Result<Result<AppStateResponse, canic::Error>, _> =
        pic.query_call(child_pid, protocol::CANIC_APP_STATE, ());
    let Err(err) = child_app_state else {
        panic!("child app state endpoint should be absent")
    };
    assert_missing_method(&err, protocol::CANIC_APP_STATE);

    let child_subnet_state: Result<Result<SubnetStateResponse, canic::Error>, _> =
        pic.query_call(child_pid, protocol::CANIC_SUBNET_STATE, ());
    let Err(err) = child_subnet_state else {
        panic!("child subnet state endpoint should be absent")
    };
    assert_missing_method(&err, protocol::CANIC_SUBNET_STATE);
}

/// Assert default root diagnostic endpoint exposure and controller gating.
///
/// # Panics
///
/// Panics if expected root diagnostic queries fail, if non-controller
/// diagnostic queries are accepted, or if the default memory-ledger endpoint is
/// present.
pub fn assert_root_diagnostics_are_controller_gated(pic: &Pic, root_id: Principal) {
    let health: Result<CanicHealthStatus, canic::Error> =
        pic.query_call_or_panic(root_id, protocol::CANIC_HEALTH, ());
    let health = health.expect("root health application");
    assert_eq!(health.schema_version, RUNTIME_INTROSPECTION_SCHEMA_VERSION);
    assert_eq!(health.status, HealthStatus::Healthy);
    assert!(
        health.observed_at_ns.is_some(),
        "health should include runtime observation time"
    );
    assert!(
        health
            .checks
            .iter()
            .any(|check| check.code == "canister_responsive"),
        "health should report responsive canister check"
    );

    let readiness: Result<CanicReadinessStatus, canic::Error> =
        pic.query_call_or_panic(root_id, protocol::CANIC_READINESS, ());
    let readiness = readiness.expect("root readiness application");
    assert_eq!(
        readiness.schema_version,
        RUNTIME_INTROSPECTION_SCHEMA_VERSION
    );
    assert_eq!(readiness.role.as_deref(), Some(CanisterRole::ROOT.as_str()));
    assert_eq!(readiness.status, ReadinessStatus::Ready);
    assert!(
        readiness.blockers.is_empty(),
        "ready root should not report readiness blockers"
    );

    let runtime_status: Result<CanicRuntimeStatus, canic::Error> =
        pic.query_call_or_panic(root_id, protocol::CANIC_RUNTIME_STATUS, ());
    let runtime_status = runtime_status.expect("root runtime status application");
    assert_eq!(
        runtime_status.schema_version,
        RUNTIME_INTROSPECTION_SCHEMA_VERSION
    );
    assert_eq!(runtime_status.canister_id, root_id);
    assert_eq!(
        runtime_status.role.as_deref(),
        Some(CanisterRole::ROOT.as_str())
    );
    assert_eq!(runtime_status.root, Some(root_id));
    assert_eq!(runtime_status.readiness.status, ReadinessStatus::Ready);
    assert_eq!(runtime_status.status, RuntimeStatus::Ok);
    assert!(
        runtime_status
            .visibility
            .iter()
            .any(|entry| entry.field == "topology"
                && entry.visibility == RuntimeFieldVisibility::ControllerOnly),
        "runtime status should classify topology as controller-only"
    );
    assert!(
        runtime_status
            .state
            .as_ref()
            .is_some_and(|state| !state.domains.is_empty()),
        "root runtime status should expose state metadata, not persisted values"
    );

    let app_registry: Result<AppRegistryResponse, canic::Error> =
        pic.query_call_or_panic(root_id, protocol::CANIC_APP_REGISTRY, ());
    app_registry.expect("root app registry application");

    let logs: Result<Page<LogEntry>, canic::Error> = pic.query_call_or_panic(
        root_id,
        protocol::CANIC_LOG,
        (
            Option::<String>::None,
            Option::<String>::None,
            Option::<canic::__internal::core::log::Level>::None,
            PageRequest {
                limit: 10,
                offset: 0,
            },
        ),
    );
    logs.expect("root log application");

    let memory_ledger: Result<Result<(), canic::Error>, _> =
        pic.query_call(root_id, protocol::CANIC_MEMORY_LEDGER, ());
    let Err(err) = memory_ledger else {
        panic!("default root memory ledger endpoint should be absent")
    };
    assert_missing_method(&err, protocol::CANIC_MEMORY_LEDGER);

    let non_controller = Principal::from_slice(&[252; 29]);
    let denied_health: Result<CanicHealthStatus, canic::Error> =
        pic.query_call_as_or_panic(root_id, non_controller, protocol::CANIC_HEALTH, ());
    let Err(denied_health) = denied_health else {
        panic!("non-controller health query must be denied")
    };
    assert_eq!(denied_health.code, ErrorCode::Unauthorized);

    let denied_readiness: Result<CanicReadinessStatus, canic::Error> =
        pic.query_call_as_or_panic(root_id, non_controller, protocol::CANIC_READINESS, ());
    let Err(denied_readiness) = denied_readiness else {
        panic!("non-controller readiness query must be denied")
    };
    assert_eq!(denied_readiness.code, ErrorCode::Unauthorized);

    let denied_runtime_status: Result<CanicRuntimeStatus, canic::Error> =
        pic.query_call_as_or_panic(root_id, non_controller, protocol::CANIC_RUNTIME_STATUS, ());
    let Err(denied_runtime_status) = denied_runtime_status else {
        panic!("non-controller runtime status query must be denied")
    };
    assert_eq!(denied_runtime_status.code, ErrorCode::Unauthorized);

    let denied_app_registry: Result<AppRegistryResponse, canic::Error> =
        pic.query_call_as_or_panic(root_id, non_controller, protocol::CANIC_APP_REGISTRY, ());
    let Err(denied_app_registry) = denied_app_registry else {
        panic!("non-controller app registry query must be denied")
    };
    assert_eq!(denied_app_registry.code, ErrorCode::Unauthorized);

    let denied_log: Result<Page<LogEntry>, canic::Error> = pic.query_call_as_or_panic(
        root_id,
        non_controller,
        protocol::CANIC_LOG,
        (
            Option::<String>::None,
            Option::<String>::None,
            Option::<canic::__internal::core::log::Level>::None,
            PageRequest {
                limit: 10,
                offset: 0,
            },
        ),
    );
    let Err(denied_log) = denied_log else {
        panic!("non-controller log query must be denied")
    };
    assert_eq!(denied_log.code, ErrorCode::Unauthorized);
}

/// Assert default child runtime introspection exposure and controller gating.
///
/// # Panics
///
/// Panics if the expected child runtime introspection queries fail, if returned
/// child identity/topology fields are inconsistent, or if non-controller
/// runtime introspection queries are accepted.
pub fn assert_child_runtime_introspection_is_controller_gated(
    pic: &Pic,
    child_pid: Principal,
    role: &CanisterRole,
    expected_parent_id: Principal,
    root_id: Principal,
) {
    let health: Result<CanicHealthStatus, canic::Error> =
        pic.query_call_as_or_panic(child_pid, root_id, protocol::CANIC_HEALTH, ());
    let health = health.expect("child health application");
    assert_eq!(health.schema_version, RUNTIME_INTROSPECTION_SCHEMA_VERSION);
    assert_eq!(health.status, HealthStatus::Healthy);
    assert!(
        health.observed_at_ns.is_some(),
        "child health should include runtime observation time"
    );

    let readiness: Result<CanicReadinessStatus, canic::Error> =
        pic.query_call_as_or_panic(child_pid, root_id, protocol::CANIC_READINESS, ());
    let readiness = readiness.expect("child readiness application");
    assert_eq!(
        readiness.schema_version,
        RUNTIME_INTROSPECTION_SCHEMA_VERSION
    );
    assert_eq!(readiness.role.as_deref(), Some(role.as_str()));
    assert_eq!(readiness.status, ReadinessStatus::Ready);
    assert!(
        readiness.blockers.is_empty(),
        "ready child should not report readiness blockers"
    );

    let runtime_status: Result<CanicRuntimeStatus, canic::Error> =
        pic.query_call_as_or_panic(child_pid, root_id, protocol::CANIC_RUNTIME_STATUS, ());
    let runtime_status = runtime_status.expect("child runtime status application");
    assert_eq!(
        runtime_status.schema_version,
        RUNTIME_INTROSPECTION_SCHEMA_VERSION
    );
    assert_eq!(runtime_status.canister_id, child_pid);
    assert_eq!(runtime_status.role.as_deref(), Some(role.as_str()));
    assert_eq!(runtime_status.root, Some(root_id));
    assert_eq!(runtime_status.readiness.status, ReadinessStatus::Ready);
    assert_eq!(runtime_status.status, RuntimeStatus::Ok);

    let topology = runtime_status
        .topology
        .as_ref()
        .expect("child runtime status should include topology metadata");
    assert_eq!(topology.root, Some(root_id));
    assert_eq!(topology.parent, Some(expected_parent_id));

    let non_controller = Principal::from_slice(&[253; 29]);
    let denied_health: Result<CanicHealthStatus, canic::Error> =
        pic.query_call_as_or_panic(child_pid, non_controller, protocol::CANIC_HEALTH, ());
    let Err(denied_health) = denied_health else {
        panic!("non-controller child health query must be denied")
    };
    assert_eq!(denied_health.code, ErrorCode::Unauthorized);

    let denied_readiness: Result<CanicReadinessStatus, canic::Error> =
        pic.query_call_as_or_panic(child_pid, non_controller, protocol::CANIC_READINESS, ());
    let Err(denied_readiness) = denied_readiness else {
        panic!("non-controller child readiness query must be denied")
    };
    assert_eq!(denied_readiness.code, ErrorCode::Unauthorized);

    let denied_runtime_status: Result<CanicRuntimeStatus, canic::Error> = pic
        .query_call_as_or_panic(
            child_pid,
            non_controller,
            protocol::CANIC_RUNTIME_STATUS,
            (),
        );
    let Err(denied_runtime_status) = denied_runtime_status else {
        panic!("non-controller child runtime status query must be denied")
    };
    assert_eq!(denied_runtime_status.code, ErrorCode::Unauthorized);
}

// Match PocketIC missing-method failures without depending on one exact transport string.
fn assert_missing_method(err: &ic_testkit::pic::PicCallError, method: &str) {
    assert_eq!(err.kind(), ic_testkit::pic::PicCallErrorKind::Transport);

    let message = err.message();

    assert!(
        message.contains(method),
        "missing-method error should mention {method}: {message}"
    );
    assert!(
        message.contains("not found")
            || message.contains("has no method")
            || message.contains("has no query method")
            || message.contains("has no update method")
            || message.contains("unknown method")
            || message.contains("did not find method"),
        "expected missing-method transport failure for {method}, got: {message}"
    );
}
