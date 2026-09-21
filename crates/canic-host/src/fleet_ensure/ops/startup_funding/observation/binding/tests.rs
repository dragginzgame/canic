use super::*;
use canic_control_plane::dto::root::{
    RootComponentChildOperationStatus, RootComponentOperationStatus,
};
use canic_core::{dto::component_registry::*, ids::*};

fn p(value: u8) -> Principal {
    Principal::from_slice(&[value])
}

fn component() -> ComponentBinding {
    ComponentBinding {
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                        fleet_id: FleetId::from_generated_bytes([1; 32]),
                    },
                    app: AppId::from("funding"),
                },
                coordinator_subnet: SubnetId::from_principal(p(1)),
                coordinator: p(2),
            },
            epoch: 1,
        },
        component: ComponentInstanceId::from_generated_bytes([3; 32]),
        component_spec: "hubs".parse().unwrap(),
        spec_hash: [4; 32],
        role: "hub".into(),
        placement_subnet: SubnetId::from_principal(p(1)),
        fleet_subnet_root: p(5),
        canister_id: p(6),
    }
}

fn release() -> FleetSubnetRootReleaseSet {
    FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([7; 32])),
        manifest_digest: ReleaseSetDigest::from_bytes([8; 32]),
    }
}

fn top() -> RootComponentOperationStatus {
    let binding = component();
    RootComponentOperationStatus {
        complete: true,
        allocation: RootComponentAllocationResponse {
            operation_id: [9; 32],
            allocation_sequence: 1,
            component: binding.component,
            component_spec: binding.component_spec.clone(),
            spec_hash: binding.spec_hash,
            role: binding.role.clone(),
            provisioning_origin: ComponentProvisioningOrigin::FleetAdministrator { caller: p(2) },
            release_set: release(),
            phase: RootComponentAllocationPhase::Committed,
            creation: None,
            installation: Some(RootComponentInstallEvidence {
                raw_module_hash: [10; 32],
                chunk_hashes: vec![],
                binding,
            }),
        },
    }
}

fn descendant() -> RootComponentChildOperationStatus {
    let component = component();
    RootComponentChildOperationStatus {
        allocation: RootComponentChildAllocationResponse {
            last_failure: None,
            operation_id: [11; 32],
            component: component.component,
            parent_canister_id: p(6),
            parent_role: "hub".into(),
            child_role: "shard".into(),
            child_kind:
                canic_core::control_plane_support::config::schema::ComponentChildKind::Shard,
            maximum_instances_per_parent: 3,
            maximum_descendants: 3,
            maximum_registry_bytes: 4096,
            reserved_against_registry: ComponentRegistryHead {
                component: component.component,
                revision: 1,
                content_hash: [12; 32],
            },
            release_set: release(),
            phase: RootComponentAllocationPhase::Committed,
            creation: None,
            installation: Some(RootComponentChildInstallEvidence {
                raw_module_hash: [13; 32],
                chunk_hashes: vec![],
                binding: ComponentChildBinding {
                    component,
                    parent_canister_id: p(6),
                    role: "shard".into(),
                    canister_id: p(14),
                },
            }),
        },
    }
}

#[test]
fn funding_parent_comes_from_exact_allocation_instead_of_pool_custody() {
    let top = top();
    let claim = CanisterPoolClaim {
        component: top.allocation.component,
        operation_id: top.allocation.operation_id,
    };
    let result = project(
        p(5),
        p(6),
        &claim,
        RootOperationStatusResponse::ProvisionComponent(top),
    )
    .unwrap();
    assert_eq!(result.parent, p(5));
    assert_eq!(result.role, CanisterRole::from("hub"));
    assert_eq!(result.component, component());
    assert_eq!(result.canister_id, p(6));
    assert_eq!(result.parent_role, None);
    assert_eq!(result.release_set, release());
    let child = descendant();
    let claim = CanisterPoolClaim {
        component: child.allocation.component,
        operation_id: child.allocation.operation_id,
    };
    let result = project(
        p(5),
        p(14),
        &claim,
        RootOperationStatusResponse::ProvisionChild(child.clone()),
    )
    .unwrap();
    assert_eq!(result.parent, p(6));
    assert_eq!(result.role, CanisterRole::from("shard"));
    assert_eq!(result.component, component());
    assert_eq!(result.canister_id, p(14));
    assert_eq!(result.parent_role, Some(CanisterRole::from("hub")));
    assert_eq!(result.release_set, release());
    // A deeper child's exact immediate parent is retained without a guessed Root edge.
    let mut deeper = child;
    deeper.allocation.parent_canister_id = p(15);
    deeper
        .allocation
        .installation
        .as_mut()
        .unwrap()
        .binding
        .parent_canister_id = p(15);
    assert_eq!(
        project(
            p(5),
            p(14),
            &claim,
            RootOperationStatusResponse::ProvisionChild(deeper)
        )
        .unwrap()
        .parent,
        p(15)
    );
}

#[test]
fn allocation_mismatch_and_unsettled_claims_never_become_zero_usage() {
    let child = descendant();
    let claim = CanisterPoolClaim {
        component: child.allocation.component,
        operation_id: child.allocation.operation_id,
    };
    for (root, target) in [(p(7), p(14)), (p(5), p(7))] {
        assert_eq!(
            project(
                root,
                target,
                &claim,
                RootOperationStatusResponse::ProvisionChild(child.clone())
            ),
            Err(StartupUsageUnavailable::AuthorityMismatch)
        );
    }
    let mut wrong = child.clone();
    wrong.allocation.operation_id = [99; 32];
    assert_eq!(
        project(
            p(5),
            p(14),
            &claim,
            RootOperationStatusResponse::ProvisionChild(wrong)
        ),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
    let mut wrong = child.clone();
    wrong.allocation.parent_canister_id = p(7);
    assert_eq!(
        project(
            p(5),
            p(14),
            &claim,
            RootOperationStatusResponse::ProvisionChild(wrong)
        ),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
    let mut unfinished = child;
    unfinished.allocation.phase = RootComponentAllocationPhase::Reserved;
    assert_eq!(
        project(
            p(5),
            p(14),
            &claim,
            RootOperationStatusResponse::ProvisionChild(unfinished)
        ),
        Err(StartupUsageUnavailable::PolicyTransition)
    );
}

#[cfg(unix)]
#[test]
fn allocation_observation_is_query_only_and_skips_non_workloads() {
    use std::os::unix::fs::PermissionsExt as _;
    let directory = crate::test_support::temp_dir("funding-parent-query");
    std::fs::create_dir_all(&directory).unwrap();
    let executable = directory.join("icp");
    let response_file = directory.join("response.json");
    let icp = IcpCli::new(executable.to_str().unwrap(), None);
    let candid = directory.join("root.did");
    // No executable exists: a Ready asset must not perform even identity/version discovery.
    assert_eq!(
        observe(&icp, &candid, p(5), p(6), &CanisterPoolAssetStatus::Ready),
        Err(StartupUsageUnavailable::NotWorkload)
    );
    let script = crate::test_support::tool_script(&format!(
        "#!/bin/sh\ncase \"$*\" in\n  --version) printf 'icp @ICP_VERSION@\\n' ;;\n  *canic_root_operation_status*--query*) cat '{}' ;;\n  *) exit 1 ;;\nesac\n",
        response_file.display()
    ));
    std::fs::write(&executable, script).unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    // Transport qualification uses Rust Candid encoding; this is not a sidecar equality proof.
    std::fs::write(&candid, "service : {}").unwrap();
    let value = descendant();
    let status = CanisterPoolAssetStatus::Workload {
        claim: CanisterPoolClaim {
            component: value.allocation.component,
            operation_id: value.allocation.operation_id,
        },
    };
    let bytes = candid::encode_one(Ok::<_, canic_core::dto::error::Error>(Response::Operation(
        Box::new(RootOperationStatusResponse::ProvisionChild(value)),
    )))
    .unwrap();
    std::fs::write(
        &response_file,
        serde_json::json!({"response_bytes": canic_core::cdk::utils::hash::hex_bytes(&bytes)})
            .to_string(),
    )
    .unwrap();
    assert_eq!(
        observe(&icp, &candid, p(5), p(14), &status).unwrap().parent,
        p(6)
    );
    std::fs::write(&response_file, "invalid response").unwrap();
    assert_eq!(
        observe(&icp, &candid, p(5), p(14), &status),
        Err(StartupUsageUnavailable::ObservationFailed)
    );
    std::fs::remove_dir_all(directory).unwrap();
}
