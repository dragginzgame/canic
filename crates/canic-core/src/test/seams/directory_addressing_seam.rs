// Category A - Internal runtime-configured tests (ConfigTestBuilder when needed).

use crate::{
    config::schema::CanisterKind,
    dto::topology::{
        DirectoryEntryInput, DirectoryProvenance, FleetDirectoryInput, SubnetDirectoryInput,
    },
    ids::{AppId, CanisterRole, CanonicalNetworkId, FleetBinding, FleetId, FleetKey},
    ops::storage::{
        directory::{fleet::FleetDirectoryOps, subnet::SubnetDirectoryOps},
        registry::subnet::SubnetRegistryOps,
    },
    storage::stable::directory::{
        DirectoryEntryRecord, fleet::FleetDirectoryData, subnet::SubnetDirectoryData,
    },
    test::{
        config::ConfigTestBuilder,
        seams::{lock, p},
        support::import_test_env,
    },
    workflow::topology::directory::query::SubnetDirectoryQuery,
};

fn provenance() -> DirectoryProvenance {
    DirectoryProvenance {
        fleet: FleetBinding {
            fleet: FleetKey {
                network: CanonicalNetworkId::public_ic(),
                fleet_id: FleetId::from_generated_bytes([1; 32]),
            },
            app: AppId::from("app"),
        },
        source_root: p(20),
    }
}

fn fleet_input(entries: Vec<DirectoryEntryInput>) -> FleetDirectoryInput {
    FleetDirectoryInput {
        provenance: provenance(),
        entries,
    }
}

fn subnet_input(entries: Vec<DirectoryEntryInput>) -> SubnetDirectoryInput {
    SubnetDirectoryInput {
        provenance: provenance(),
        entries,
    }
}

#[test]
fn directory_addressing_prefers_index_over_registry_duplicates() {
    let _guard = lock();

    for entry in SubnetRegistryOps::data().entries {
        let _ = SubnetRegistryOps::unregister(&entry.pid);
    }
    SubnetDirectoryOps::import_trusted_partial(SubnetDirectoryData {
        entries: Vec::new(),
    })
    .expect("clear Subnet Directory");

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

    SubnetDirectoryOps::import_trusted_partial(SubnetDirectoryData {
        entries: vec![DirectoryEntryRecord {
            role: role.clone(),
            pid: pid_b,
        }],
    })
    .expect("import Subnet Directory");

    let resolved = SubnetDirectoryQuery::get(role.clone()).expect("Directory role missing");
    assert_eq!(resolved, pid_b);

    let duplicates = SubnetRegistryOps::data()
        .entries
        .into_iter()
        .filter(|entry| entry.record.role == role)
        .count();

    assert_eq!(duplicates, 2);
}

#[test]
fn directory_addressing_does_not_fallback_to_registry() {
    let _guard = lock();

    for entry in SubnetRegistryOps::data().entries {
        let _ = SubnetRegistryOps::unregister(&entry.pid);
    }
    SubnetDirectoryOps::import_trusted_partial(SubnetDirectoryData {
        entries: Vec::new(),
    })
    .expect("clear Subnet Directory");

    let role = CanisterRole::new("seam_directory_no_fallback");
    let root_pid = p(13);
    let pid = p(14);
    let created_at = 1;

    SubnetRegistryOps::register_root(root_pid, created_at);
    SubnetRegistryOps::register_unchecked(pid, &role, root_pid, vec![], created_at)
        .expect("register canister");

    let resolved = SubnetDirectoryQuery::get(role.clone());
    assert!(resolved.is_none());

    let registry_count = SubnetRegistryOps::data()
        .entries
        .into_iter()
        .filter(|entry| entry.record.role == role)
        .count();
    assert_eq!(registry_count, 1);
}

fn install_index_service_test_config(service_role: &CanisterRole, singleton_role: &CanisterRole) {
    let _config = ConfigTestBuilder::new()
        .with_default_canister_kind(service_role.clone(), CanisterKind::Service)
        .with_default_canister_kind(singleton_role.clone(), CanisterKind::Singleton)
        .with_fleet_service(service_role.clone())
        .install();
    import_test_env(
        service_role.clone(),
        crate::ids::SubnetSlotId::DEFAULT,
        p(20),
    );
}

fn clear_app_and_subnet_directoryes() {
    FleetDirectoryOps::import_trusted_partial(FleetDirectoryData {
        entries: Vec::new(),
    })
    .expect("clear Fleet Directory");
    SubnetDirectoryOps::import_trusted_partial(SubnetDirectoryData {
        entries: Vec::new(),
    })
    .expect("clear Subnet Directory");
}

#[test]
fn incomplete_index_imports_reject_roles_outside_configured_service_sets() {
    let _guard = lock();

    let service_role = CanisterRole::new("project_hub");
    let singleton_role = CanisterRole::new("project_ledger");
    let service_pid = p(21);
    let singleton_pid = p(22);

    install_index_service_test_config(&service_role, &singleton_role);
    clear_app_and_subnet_directoryes();

    FleetDirectoryOps::import_args_allow_incomplete(fleet_input(vec![DirectoryEntryInput {
        role: service_role.clone(),
        pid: service_pid,
    }]))
    .expect("configured app service role should import");
    SubnetDirectoryOps::import_args_allow_incomplete(subnet_input(vec![DirectoryEntryInput {
        role: service_role.clone(),
        pid: service_pid,
    }]))
    .expect("configured subnet service role should import");

    FleetDirectoryOps::import_args_allow_incomplete(fleet_input(vec![DirectoryEntryInput {
        role: singleton_role.clone(),
        pid: singleton_pid,
    }]))
    .expect_err("Fleet Directory should reject roles outside explicit fleet_directory");

    SubnetDirectoryOps::import_args_allow_incomplete(subnet_input(vec![DirectoryEntryInput {
        role: singleton_role.clone(),
        pid: singleton_pid,
    }]))
    .expect_err("Subnet Directory should reject non-service roles");

    FleetDirectoryOps::import(FleetDirectoryData {
        entries: vec![DirectoryEntryRecord {
            role: service_role.clone(),
            pid: service_pid,
        }],
    })
    .expect("full Fleet Directory import should accept exact configured role set");
    SubnetDirectoryOps::import(SubnetDirectoryData {
        entries: vec![DirectoryEntryRecord {
            role: service_role.clone(),
            pid: service_pid,
        }],
    })
    .expect("full Subnet Directory import should accept exact configured role set");

    FleetDirectoryOps::import(FleetDirectoryData {
        entries: vec![
            DirectoryEntryRecord {
                role: service_role.clone(),
                pid: service_pid,
            },
            DirectoryEntryRecord {
                role: singleton_role.clone(),
                pid: singleton_pid,
            },
        ],
    })
    .expect_err("full Fleet Directory import should reject roles outside explicit fleet_directory");

    SubnetDirectoryOps::import(SubnetDirectoryData {
        entries: vec![
            DirectoryEntryRecord {
                role: service_role.clone(),
                pid: service_pid,
            },
            DirectoryEntryRecord {
                role: singleton_role,
                pid: singleton_pid,
            },
        ],
    })
    .expect_err("full Subnet Directory import should reject non-service roles");

    assert_eq!(FleetDirectoryOps::get(&service_role), Some(service_pid));
    assert_eq!(SubnetDirectoryOps::get(&service_role), Some(service_pid));
}

#[test]
fn local_index_filters_drop_roles_outside_configured_service_sets() {
    let _guard = lock();

    let service_role = CanisterRole::new("project_hub");
    let singleton_role = CanisterRole::new("project_ledger");
    let service_pid = p(21);
    let singleton_pid = p(22);

    install_index_service_test_config(&service_role, &singleton_role);

    let filtered_app = FleetDirectoryOps::filter_args_for_local_config(fleet_input(vec![
        DirectoryEntryInput {
            role: service_role.clone(),
            pid: service_pid,
        },
        DirectoryEntryInput {
            role: singleton_role.clone(),
            pid: singleton_pid,
        },
    ]))
    .expect("filter Fleet Directory for local config");
    assert_eq!(filtered_app.entries.len(), 1);
    assert_eq!(&filtered_app.entries[0].role, &service_role);

    let filtered_subnet = SubnetDirectoryOps::filter_args_for_local_config(subnet_input(vec![
        DirectoryEntryInput {
            role: service_role.clone(),
            pid: service_pid,
        },
        DirectoryEntryInput {
            role: singleton_role,
            pid: singleton_pid,
        },
    ]))
    .expect("filter Subnet Directory for local config");
    assert_eq!(filtered_subnet.entries.len(), 1);
    assert_eq!(&filtered_subnet.entries[0].role, &service_role);
}
