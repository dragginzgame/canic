// Category A - Internal runtime-configured tests (ConfigTestBuilder when needed).

use crate::{
    config::schema::CanisterKind,
    dto::topology::{AppIndexArgs, IndexEntryInput, SubnetIndexArgs},
    ids::CanisterRole,
    ops::storage::{
        index::{app::AppIndexOps, subnet::SubnetIndexOps},
        registry::subnet::SubnetRegistryOps,
    },
    storage::stable::index::{app::AppIndexRecord, subnet::SubnetIndexRecord},
    test::{
        config::ConfigTestBuilder,
        seams::{lock, p},
        support::import_test_env,
    },
    workflow::topology::index::query::SubnetIndexQuery,
};

#[test]
fn index_addressing_prefers_index_over_registry_duplicates() {
    let _guard = lock();

    for (pid, _) in SubnetRegistryOps::data().entries {
        let _ = SubnetRegistryOps::unregister(&pid);
    }
    SubnetIndexOps::import_trusted_partial(SubnetIndexRecord {
        entries: Vec::new(),
    })
    .expect("clear subnet index");

    let role = CanisterRole::new("seam_directory_role");
    let root_pid = p(10);
    let pid_a = p(11);
    let pid_b = p(12);

    let created_at = 1;
    SubnetRegistryOps::register_root(root_pid, created_at);
    SubnetRegistryOps::register_unchecked(pid_a, &role, root_pid, vec![], created_at)
        .expect("register first canister");
    SubnetRegistryOps::register_unchecked(pid_b, &role, root_pid, vec![], created_at)
        .expect("register second canister with same role");

    SubnetIndexOps::import_trusted_partial(SubnetIndexRecord {
        entries: vec![(role.clone(), pid_b)],
    })
    .expect("import subnet index");

    let resolved = SubnetIndexQuery::get(role.clone()).expect("index role missing");
    assert_eq!(resolved, pid_b);

    let duplicates = SubnetRegistryOps::data()
        .entries
        .into_iter()
        .filter(|(_, entry)| entry.role == role)
        .count();

    assert_eq!(duplicates, 2);
}

#[test]
fn index_addressing_does_not_fallback_to_registry() {
    let _guard = lock();

    for (pid, _) in SubnetRegistryOps::data().entries {
        let _ = SubnetRegistryOps::unregister(&pid);
    }
    SubnetIndexOps::import_trusted_partial(SubnetIndexRecord {
        entries: Vec::new(),
    })
    .expect("clear subnet index");

    let role = CanisterRole::new("seam_directory_no_fallback");
    let root_pid = p(13);
    let pid = p(14);
    let created_at = 1;

    SubnetRegistryOps::register_root(root_pid, created_at);
    SubnetRegistryOps::register_unchecked(pid, &role, root_pid, vec![], created_at)
        .expect("register canister");

    let resolved = SubnetIndexQuery::get(role.clone());
    assert!(resolved.is_none());

    let registry_count = SubnetRegistryOps::data()
        .entries
        .into_iter()
        .filter(|(_, entry)| entry.role == role)
        .count();
    assert_eq!(registry_count, 1);
}

fn install_index_service_test_config(service_role: &CanisterRole, singleton_role: &CanisterRole) {
    let _config = ConfigTestBuilder::new()
        .with_prime_canister_kind(service_role.clone(), CanisterKind::Service)
        .with_prime_canister_kind(singleton_role.clone(), CanisterKind::Singleton)
        .with_app_index(service_role.clone())
        .install();
    import_test_env(service_role.clone(), crate::ids::SubnetRole::PRIME, p(20));
}

fn clear_app_and_subnet_indexes() {
    AppIndexOps::import_trusted_partial(AppIndexRecord {
        entries: Vec::new(),
    })
    .expect("clear app index");
    SubnetIndexOps::import_trusted_partial(SubnetIndexRecord {
        entries: Vec::new(),
    })
    .expect("clear subnet index");
}

#[test]
fn incomplete_index_imports_reject_roles_outside_configured_service_sets() {
    let _guard = lock();

    let service_role = CanisterRole::new("project_hub");
    let singleton_role = CanisterRole::new("project_ledger");
    let service_pid = p(21);
    let singleton_pid = p(22);

    install_index_service_test_config(&service_role, &singleton_role);
    clear_app_and_subnet_indexes();

    AppIndexOps::import_args_allow_incomplete(AppIndexArgs(vec![IndexEntryInput {
        role: service_role.clone(),
        pid: service_pid,
    }]))
    .expect("configured app service role should import");
    SubnetIndexOps::import_args_allow_incomplete(SubnetIndexArgs(vec![IndexEntryInput {
        role: service_role.clone(),
        pid: service_pid,
    }]))
    .expect("configured subnet service role should import");

    let app_err = AppIndexOps::import_args_allow_incomplete(AppIndexArgs(vec![IndexEntryInput {
        role: singleton_role.clone(),
        pid: singleton_pid,
    }]))
    .expect_err("app index should reject roles outside explicit app_index");
    assert!(
        app_err.to_string().contains("unexpected roles"),
        "expected unexpected app role error, got: {app_err}"
    );

    let subnet_err =
        SubnetIndexOps::import_args_allow_incomplete(SubnetIndexArgs(vec![IndexEntryInput {
            role: singleton_role.clone(),
            pid: singleton_pid,
        }]))
        .expect_err("subnet index should reject non-service roles");
    assert!(
        subnet_err.to_string().contains("unexpected roles"),
        "expected unexpected subnet role error, got: {subnet_err}"
    );

    AppIndexOps::import(AppIndexRecord {
        entries: vec![(service_role.clone(), service_pid)],
    })
    .expect("full app index import should accept exact configured role set");
    SubnetIndexOps::import(SubnetIndexRecord {
        entries: vec![(service_role.clone(), service_pid)],
    })
    .expect("full subnet index import should accept exact configured role set");

    let app_full_err = AppIndexOps::import(AppIndexRecord {
        entries: vec![
            (service_role.clone(), service_pid),
            (singleton_role.clone(), singleton_pid),
        ],
    })
    .expect_err("full app index import should reject roles outside explicit app_index");
    assert!(
        app_full_err.to_string().contains("unexpected roles"),
        "expected unexpected full app role error, got: {app_full_err}"
    );

    let subnet_full_err = SubnetIndexOps::import(SubnetIndexRecord {
        entries: vec![
            (service_role.clone(), service_pid),
            (singleton_role, singleton_pid),
        ],
    })
    .expect_err("full subnet index import should reject non-service roles");
    assert!(
        subnet_full_err.to_string().contains("unexpected roles"),
        "expected unexpected full subnet role error, got: {subnet_full_err}"
    );

    assert_eq!(AppIndexOps::get(&service_role), Some(service_pid));
    assert_eq!(SubnetIndexOps::get(&service_role), Some(service_pid));
}

#[test]
fn local_index_filters_drop_roles_outside_configured_service_sets() {
    let _guard = lock();

    let service_role = CanisterRole::new("project_hub");
    let singleton_role = CanisterRole::new("project_ledger");
    let service_pid = p(21);
    let singleton_pid = p(22);

    install_index_service_test_config(&service_role, &singleton_role);

    let filtered_app = AppIndexOps::filter_args_for_local_config(AppIndexArgs(vec![
        IndexEntryInput {
            role: service_role.clone(),
            pid: service_pid,
        },
        IndexEntryInput {
            role: singleton_role.clone(),
            pid: singleton_pid,
        },
    ]))
    .expect("filter app index for local config");
    assert_eq!(filtered_app.0.len(), 1);
    assert_eq!(&filtered_app.0[0].role, &service_role);

    let filtered_subnet = SubnetIndexOps::filter_args_for_local_config(SubnetIndexArgs(vec![
        IndexEntryInput {
            role: service_role.clone(),
            pid: service_pid,
        },
        IndexEntryInput {
            role: singleton_role,
            pid: singleton_pid,
        },
    ]))
    .expect("filter subnet index for local config");
    assert_eq!(filtered_subnet.0.len(), 1);
    assert_eq!(&filtered_subnet.0[0].role, &service_role);
}
