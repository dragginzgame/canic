use super::*;
use crate::fleet_ensure::policy::startup_funding::{funding_binding, hub_config};
use canic_core::{
    control_plane_support::config::ComponentDeploymentConfiguration,
    dto::component_registry::{
        ComponentDirectoryChildEntry, ComponentDirectoryPageCursor, ComponentDirectoryPageResponse,
        ComponentDirectoryProvenance, ComponentProvisioningOrigin, ComponentRegistryHead,
    },
    ids::{
        ComponentChildBinding, ComponentSpecAdmission, CyclesFundingBudget,
        FleetSubnetCanisterPoolConfig, FleetSubnetRootLimits,
    },
    role_contract::ProtocolProfileDigest,
};

struct Fixture {
    topology: ComponentTopology,
    placement: StartupFundingPlacement,
    members: Vec<StartupChildFundingBinding>,
    partition: ComponentRegistryPartitionResponse,
    directory: ComponentDirectoryHead,
}

#[derive(CandidType)]
enum PageResponse {
    ComponentDirectoryPage(ComponentDirectoryPageResponse),
}

fn fixture() -> Fixture {
    let topology = ComponentDeploymentConfiguration::compile(&hub_config())
        .unwrap()
        .component_topology;
    let spec = &topology.component_specs[0];
    let top = funding_binding(spec);
    let mut child = top.clone();
    child.canister_id = Principal::from_slice(&[7; 29]);
    child.parent = top.canister_id;
    child.parent_role = Some(top.role.clone());
    child.role = "shard".into();
    let mut sibling = child.clone();
    sibling.canister_id = Principal::from_slice(&[8; 29]);
    let partition = ComponentRegistryPartitionResponse {
        head: ComponentRegistryHead {
            component: top.component.component,
            revision: 3,
            content_hash: [11; 32],
        },
        binding: top.component.clone(),
        protocol_profile_digest: ProtocolProfileDigest::from_bytes([12; 32]),
        provisioning_origin: ComponentProvisioningOrigin::FleetAdministrator {
            caller: Principal::from_slice(&[9; 29]),
        },
        release_set: top.release_set,
        status: ComponentLifecycleStatus::Active,
        reserved_descendants: 0,
        committed_descendants: 2,
        encoded_bytes: 1024,
    };
    let directory = ComponentDirectoryHead {
        provenance: ComponentDirectoryProvenance {
            component: top.component.clone(),
            source_fleet_subnet_root: top.parent,
            component_registry_revision: partition.head.revision,
            component_registry_content_hash: partition.head.content_hash,
            synchronized_at_ns: 0,
        },
        descendant_count: 2,
    };
    let placement = StartupFundingPlacement {
        active: true,
        placement_subnet: top.component.placement_subnet,
        release_set: top.release_set,
        component_admissions: vec![ComponentSpecAdmission {
            component_spec: spec.component_spec.clone(),
            spec_hash: spec.spec_hash,
            maximum_root_instances: 1,
        }],
        component_topology_digest: topology.digest().unwrap(),
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 1,
            maximum_registry_bytes: 2_097_152,
            maximum_wasm_store_bytes: 40_000_000,
            maximum_group_placements: 1,
            canister_pool: FleetSubnetCanisterPoolConfig {
                minimum_size: 1,
                maximum_size: 4,
                canister_cycles: 5_000_000_000_000.into(),
                creation_execution_margin: 1_000_000_000_000.into(),
            },
            cycles_funding: CyclesFundingBudget {
                window_secs: 3600,
                maximum_cycles: 10_000_000_000_000.into(),
            },
        },
        funding: crate::test_support::fleet_subnet_root_funding_authority(),
    };
    Fixture {
        topology,
        placement,
        members: vec![top, child, sibling],
        partition,
        directory,
    }
}

fn page(
    fixture: &Fixture,
    indices: &[usize],
    cursor: Option<u8>,
) -> ComponentDirectoryPageResponse {
    ComponentDirectoryPageResponse {
        directory: fixture.directory.clone(),
        entries: indices.iter().map(|index| {
            let binding = &fixture.members[*index];
            ComponentDirectoryChildEntry {
                binding: ComponentChildBinding { component: binding.component.clone(),
                    parent_canister_id: binding.parent, role: binding.role.clone(), canister_id: binding.canister_id },
                kind: canic_core::control_plane_support::config::schema::ComponentChildKind::Shard,
                installed_artifact_hash: [13; 32], protocol_profile_digest: ProtocolProfileDigest::from_bytes([14; 32]),
                status: ComponentLifecycleStatus::Active,
            }
        }).collect(),
        next_cursor: cursor.map(|value| ComponentDirectoryPageCursor(vec![value])),
    }
}

#[test]
fn inventory_requires_root_coverage_one_top_level_and_unique_identities() {
    let fixture = fixture();
    let members = fixture.members.iter().collect::<Vec<_>>();
    let summary = StartupRootInventory { workloads: 3 };
    assert!(group(&fixture.placement, summary, &members).is_ok());
    assert!(matches!(
        group(
            &fixture.placement,
            StartupRootInventory { workloads: 4 },
            &members
        ),
        Err(StartupUsageUnavailable::InventoryIncomplete)
    ));
    assert!(matches!(
        group(
            &fixture.placement,
            StartupRootInventory { workloads: 2 },
            &members[1..]
        ),
        Err(StartupUsageUnavailable::InventoryIncomplete)
    ));
    assert!(matches!(
        group(
            &fixture.placement,
            summary,
            &[members[0], members[1], members[1]]
        ),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    ));
    let mut withdrawn = fixture.placement;
    withdrawn.component_admissions.clear();
    assert!(matches!(
        group(&withdrawn, summary, &members),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    ));
}

#[test]
fn partition_requires_complete_active_and_unreserved_membership() {
    let fixture = fixture();
    let members = fixture.members.iter().collect::<Vec<_>>();
    assert_eq!(
        validate_partition(&fixture.partition, &fixture.topology, &members),
        Ok(())
    );
    let mut partial = fixture.partition.clone();
    partial.committed_descendants += 1;
    assert_eq!(
        validate_partition(&partial, &fixture.topology, &members),
        Err(StartupUsageUnavailable::InventoryIncomplete)
    );
    let mut pending = fixture.partition.clone();
    pending.reserved_descendants = 1;
    assert_eq!(
        validate_partition(&pending, &fixture.topology, &members),
        Err(StartupUsageUnavailable::PolicyTransition)
    );
    let mut removed = fixture.partition.clone();
    removed.status = ComponentLifecycleStatus::Removed;
    assert_eq!(
        validate_partition(&removed, &fixture.topology, &members),
        Err(StartupUsageUnavailable::PolicyTransition)
    );
    let mut oversized = fixture.partition;
    oversized.encoded_bytes = u64::MAX;
    assert_eq!(
        validate_partition(&oversized, &fixture.topology, &members),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
}

#[test]
fn directory_requires_current_partition_and_source_root() {
    let fixture = fixture();
    let root = fixture.members[0].parent;
    assert_eq!(
        validate_directory(root, &fixture.partition, &fixture.directory),
        Ok(())
    );
    let mut stale = fixture.directory.clone();
    stale.provenance.component_registry_revision -= 1;
    assert_eq!(
        validate_directory(root, &fixture.partition, &stale),
        Err(StartupUsageUnavailable::PolicyTransition)
    );
    assert_eq!(
        validate_directory(
            Principal::from_slice(&[99; 29]),
            &fixture.partition,
            &fixture.directory
        ),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
}

#[test]
fn pagination_covers_every_exact_edge_without_order_assumptions() {
    let fixture = fixture();
    let members = fixture.members.iter().collect::<Vec<_>>();
    let mut pages = [page(&fixture, &[2], Some(1)), page(&fixture, &[1], None)].into_iter();
    let mut calls = 0;
    assert_eq!(
        pages::collect(&fixture.directory, &members, |request| {
            assert_eq!(
                (request.parent_canister_id, request.role, request.status),
                (None, None, None)
            );
            assert_eq!(
                request.cursor,
                (calls == 1).then(|| ComponentDirectoryPageCursor(vec![1]))
            );
            calls += 1;
            Ok(pages.next().unwrap())
        }),
        Ok(())
    );
    assert_eq!(calls, 2);
}

#[test]
fn malformed_or_partial_pages_never_become_complete_and_stop_reading() {
    let fixture = fixture();
    let members = fixture.members.iter().collect::<Vec<_>>();
    let mut changed = page(&fixture, &[1, 2], None);
    changed.directory.provenance.component_registry_revision += 1;
    let mut wrong_parent = page(&fixture, &[1, 2], None);
    wrong_parent.entries[0].binding.parent_canister_id = fixture.members[0].parent;
    let mut draining = page(&fixture, &[1, 2], None);
    draining.entries[0].status = ComponentLifecycleStatus::Draining;
    for (response, reason) in [
        (
            page(&fixture, &[], Some(1)),
            StartupUsageUnavailable::InventoryIncomplete,
        ),
        (
            page(&fixture, &[1], None),
            StartupUsageUnavailable::InventoryIncomplete,
        ),
        (
            page(&fixture, &[1, 1], None),
            StartupUsageUnavailable::AuthorityMismatch,
        ),
        (
            page(&fixture, &[1, 2], Some(1)),
            StartupUsageUnavailable::InventoryIncomplete,
        ),
        (changed, StartupUsageUnavailable::PolicyTransition),
        (wrong_parent, StartupUsageUnavailable::AuthorityMismatch),
        (draining, StartupUsageUnavailable::PolicyTransition),
    ] {
        let mut calls = 0;
        assert_eq!(
            pages::collect(&fixture.directory, &members, |_| {
                calls += 1;
                Ok(response.clone())
            }),
            Err(reason)
        );
        assert_eq!(calls, 1);
    }
    assert_eq!(
        pages::collect(&fixture.directory, &members, |_| Err(
            StartupUsageUnavailable::ObservationFailed
        )),
        Err(StartupUsageUnavailable::ObservationFailed)
    );
}

#[cfg(unix)]
#[test]
fn complete_membership_is_query_only_and_rechecked_after_collection() {
    use std::os::unix::fs::PermissionsExt as _;
    let fixture = fixture();
    let path = crate::test_support::temp_dir("funding-inventory");
    std::fs::create_dir_all(&path).unwrap();
    let executable = path.join("icp");
    std::fs::write(
        &executable,
        crate::test_support::tool_script(
            r#"#!/bin/sh
case "$*" in
  --version) printf 'icp @ICP_VERSION@\n' ;;
  *canic_root_status*--query*)
    count=0
    if [ -f count ]; then count=$(cat count); fi
    count=$((count + 1))
    printf '%s' "$count" > count
    cat "$count.json" ;;
  *) exit 1 ;;
esac
"#,
        ),
    )
    .unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let candid = path.join("root.did");
    std::fs::write(&candid, "service : {}").unwrap();
    let write = |index, bytes: Vec<u8>| {
        std::fs::write(path.join(format!("{index}.json")), serde_json::json!({ "response_bytes": canic_core::cdk::utils::hash::hex_bytes(&bytes) }).to_string()).unwrap();
    };
    let partition_bytes = |partition| {
        candid::encode_one(Ok::<_, canic_core::dto::error::Error>(
            Response::ComponentRegistryPartition(Box::new(partition)),
        ))
        .unwrap()
    };
    let directory_bytes = candid::encode_one(Ok::<_, canic_core::dto::error::Error>(
        Response::ComponentDirectoryHead(Box::new(fixture.directory.clone())),
    ))
    .unwrap();
    write(1, partition_bytes(fixture.partition.clone()));
    write(2, directory_bytes.clone());
    write(
        3,
        candid::encode_one(Ok::<_, canic_core::dto::error::Error>(
            PageResponse::ComponentDirectoryPage(page(&fixture, &[1, 2], None)),
        ))
        .unwrap(),
    );
    write(4, partition_bytes(fixture.partition.clone()));
    write(5, directory_bytes);
    let mut changed = fixture.partition;
    changed.head.revision += 1;
    write(6, partition_bytes(changed));
    let icp = IcpCli::new(executable.to_str().unwrap(), None).with_cwd(path.clone());
    let root = fixture.members[0].parent;
    let evidence = observe(
        &icp,
        &candid,
        root,
        &fixture.placement,
        &fixture.topology,
        StartupRootInventory { workloads: 3 },
        &fixture.members.iter().collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(
        recheck(&icp, &candid, root, &evidence),
        Ok(StartupInventoryCoverage {
            components: 1,
            descendants: 2
        })
    );
    assert_eq!(
        recheck(&icp, &candid, root, &evidence),
        Err(StartupUsageUnavailable::PolicyTransition)
    );
    std::fs::remove_dir_all(path).unwrap();
}
