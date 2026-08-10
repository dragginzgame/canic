//! Module: ops::fleet_service_peer
//!
//! Responsibility: derive and validate remote Fleet-service peer requester authority.
//! Does not own: endpoint authentication, lifecycle orchestration, grant policy, or persistence.
//! Boundary: converts one validated root Mirror into an exact read-only requester projection.

use crate::view::{
    fleet_registry_mirror::ValidatedRootFleetRegistryMirrorView,
    fleet_service_peer::FleetServicePeerRequesterView,
};
use canic_core::{
    control_plane_support::{
        config::{ComponentProvisioningGrant, ComponentTopology},
        error::{InternalError, InternalErrorOrigin},
    },
    dto::{
        component_registry::FleetServiceComponentRequester,
        error::Error,
        fleet_registry::{
            FleetRegistryVersion, FleetServiceBinding, FleetServiceComponentBinding,
            FleetSubnetRootEntry, FleetSubnetRootStatus,
        },
    },
    ids::{ComponentBinding, ComponentSpecId, FleetServiceId, FleetSubnetRootBinding},
};

///
/// FleetServicePeerOps
///
/// Stateless conversion and validation operations for one cross-root requester proof.
///

pub struct FleetServicePeerOps;

impl FleetServicePeerOps {
    /// Derive one exact remote requester from a fully validated current root Mirror.
    pub(crate) fn resolve(
        target_root: &FleetSubnetRootBinding,
        topology: &ComponentTopology,
        mirror: &ValidatedRootFleetRegistryMirrorView,
        caller: candid::Principal,
        expected_service: &FleetServiceId,
    ) -> Result<FleetServicePeerRequesterView, InternalError> {
        if mirror.root_entry.status != FleetSubnetRootStatus::Active {
            return Err(InternalError::public(Error::forbidden(
                "cross-root peer provisioning requires an Active target root",
            )));
        }
        let registry = &mirror.active.snapshot.registry;
        if registry.authority != target_root.authority {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "cross-root peer Registry authority differs from the target root",
            ));
        }
        if mirror.active.directory.provenance.registry != mirror.active.snapshot.version {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "cross-root peer Fleet Directory authority differs from the current Registry",
            ));
        }
        let (service, member) = exact_registry_service_caller(&registry.services, caller)?;
        if &service.service != expected_service {
            return Err(InternalError::public(Error::forbidden(
                "cross-root peer caller belongs to a different Fleet service",
            )));
        }
        let owner =
            exact_service_member_root(&registry.fleet_subnet_roots, member.fleet_subnet_root)?;
        let owner_is_remote_and_active = [
            owner.status == FleetSubnetRootStatus::Active,
            owner.fleet_subnet_root != target_root.fleet_subnet_root,
        ]
        .into_iter()
        .all(|valid| valid);
        if !owner_is_remote_and_active {
            return Err(InternalError::public(Error::forbidden(
                "cross-root peer caller requires a distinct Active owning root",
            )));
        }
        let admission = owner
            .component_admissions
            .iter()
            .find(|admission| admission.component_spec == service.component_spec)
            .ok_or_else(|| {
                InternalError::invariant(
                    InternalErrorOrigin::Storage,
                    "cross-root peer requester Spec is absent from its owning root admission",
                )
            })?;
        let requester_root = root_binding(&registry.authority, owner);
        let component = component_binding(&registry.authority, service, member, owner, admission);
        topology
            .validate_component_binding(&requester_root, &component)
            .map_err(|error| {
                InternalError::invariant(
                    InternalErrorOrigin::Storage,
                    format!("cross-root peer requester binding is invalid: {error}"),
                )
            })?;
        Ok(FleetServicePeerRequesterView {
            requester: FleetServiceComponentRequester {
                service: service.service.clone(),
                member_purpose: member.member_purpose,
                group_placement: member.group_placement.clone(),
                member_path: member.member_path.clone(),
                component,
            },
            root: requester_root,
        })
    }

    /// Validate the durable remote origin against the protected topology and target root.
    pub(crate) fn validate_origin(
        target_root: &FleetSubnetRootBinding,
        topology: &ComponentTopology,
        target_component_spec: &ComponentSpecId,
        requester: &FleetServiceComponentRequester,
        registry: &FleetRegistryVersion,
        grant: &ComponentProvisioningGrant,
    ) -> Result<(), InternalError> {
        let component = &requester.component;
        let registry_is_bound = [
            registry.authority == target_root.authority,
            registry.authority == component.authority,
            registry.revision > 0,
            registry.content_hash != [0; 32],
        ]
        .into_iter()
        .all(|valid| valid);
        let requester_is_remote = component.fleet_subnet_root != target_root.fleet_subnet_root;
        let Some(spec) = topology.get(&component.component_spec) else {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "stored cross-root requester Spec is absent from the protected topology",
            ));
        };
        let component_is_exact = [
            component.spec_hash == spec.spec_hash,
            component.role == spec.component_role,
            component.component.as_bytes() != &[0; 32],
            component.canister_id != candid::Principal::anonymous(),
        ]
        .into_iter()
        .all(|valid| valid);
        let expected_grant =
            topology.provisioning_grant(&component.component_spec, &grant.target_component_spec);
        let origin_is_exact = [
            registry_is_bound,
            requester_is_remote,
            component_is_exact,
            &grant.target_component_spec == target_component_spec,
            expected_grant == Some(grant),
        ]
        .into_iter()
        .all(|exact| exact);
        if !origin_is_exact {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "stored cross-root peer provisioning origin differs from protected authority",
            ));
        }
        Ok(())
    }
}

fn exact_registry_service_caller(
    services: &[FleetServiceBinding],
    caller: candid::Principal,
) -> Result<(&FleetServiceBinding, &FleetServiceComponentBinding), InternalError> {
    let mut candidates = services.iter().flat_map(|service| {
        service
            .members
            .iter()
            .filter(move |member| member.canister_id == caller)
            .map(move |member| (service, member))
    });
    let member = candidates.next().ok_or_else(|| {
        InternalError::public(Error::forbidden(
            "cross-root peer caller is not a current Fleet service member",
        ))
    })?;
    if candidates.next().is_some() {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "cross-root peer caller has ambiguous Fleet Registry service membership",
        ));
    }
    Ok(member)
}

fn exact_service_member_root(
    roots: &[FleetSubnetRootEntry],
    expected_root: candid::Principal,
) -> Result<&FleetSubnetRootEntry, InternalError> {
    let mut candidates = roots
        .iter()
        .filter(|root| root.fleet_subnet_root == expected_root);
    let root = candidates.next().ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "cross-root peer requester owning root is absent from the Fleet Registry",
        )
    })?;
    if candidates.next().is_some() {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "cross-root peer requester owning root is ambiguous in the Fleet Registry",
        ));
    }
    Ok(root)
}

fn root_binding(
    authority: &canic_core::ids::FleetRegistryAuthority,
    root: &FleetSubnetRootEntry,
) -> FleetSubnetRootBinding {
    FleetSubnetRootBinding {
        authority: authority.clone(),
        placement_subnet: root.placement_subnet,
        fleet_subnet_root: root.fleet_subnet_root,
        component_admissions: root.component_admissions.clone(),
        component_topology_digest: root.component_topology_digest,
        limits: root.limits.clone(),
    }
}

fn component_binding(
    authority: &canic_core::ids::FleetRegistryAuthority,
    service: &FleetServiceBinding,
    member: &FleetServiceComponentBinding,
    root: &FleetSubnetRootEntry,
    admission: &canic_core::ids::ComponentSpecAdmission,
) -> ComponentBinding {
    ComponentBinding {
        authority: authority.clone(),
        component: member.component,
        component_spec: service.component_spec.clone(),
        spec_hash: admission.spec_hash,
        role: service.role.clone(),
        placement_subnet: root.placement_subnet,
        fleet_subnet_root: root.fleet_subnet_root,
        canister_id: member.canister_id,
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::fleet_registry_mirror::RootFleetRegistryActiveView;
    use canic_core::{
        cdk::types::Cycles,
        control_plane_support::config::{
            ConfigModel, FleetServiceMemberPurpose, FleetServicePlacementPolicy,
        },
        dto::fleet_registry::{
            FleetDirectoryProvenance, FleetDirectoryService, FleetDirectoryServiceComponent,
            FleetDirectorySnapshot, FleetRegistry, FleetRegistryManifest,
            FleetRegistrySnapshotResponse, FleetServiceMode, FleetSubnetRootDirectoryEntry,
        },
        ids::{
            AppId, CanisterRole, CanonicalNetworkId, ComponentInstanceId, ComponentSpecAdmission,
            CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey,
            FleetRegistryAuthority, FleetSubnetCanisterPoolConfig, FleetSubnetRootLimits,
            FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest,
            SubnetId,
        },
    };

    const CONFIG: &str = r#"
        [app]
        name = "toko"

        [roles.root]
        package = "root"
        kind = "root"

        [roles.project_hub]
        package = "project_hub"
        kind = "canister"

        [roles.user_hub]
        package = "user_hub"
        kind = "canister"

        [component_specs.projects]
        component_role = "project_hub"
        maximum_instances = 4

        [component_specs.projects.provisions.users]
        maximum_instances_per_requester_per_root = 2

        [component_specs.users]
        component_role = "user_hub"
        maximum_instances = 4
    "#;

    #[test]
    fn exact_current_registry_and_directory_derive_the_remote_requester() {
        let fixture = peer_fixture();
        let resolved = FleetServicePeerOps::resolve(
            &fixture.target_root,
            &fixture.topology,
            &fixture.mirror,
            fixture.caller,
            &fixture.service,
        )
        .expect("derive cross-root requester");

        assert_eq!(resolved.requester, fixture.requester);
        assert_eq!(resolved.root, fixture.requester_root);
    }

    #[test]
    fn stale_directory_or_inactive_root_rejects() {
        let mut fixture = peer_fixture();
        fixture
            .mirror
            .active
            .directory
            .provenance
            .registry
            .content_hash = [99; 32];
        assert!(resolve(&fixture, fixture.caller, &fixture.service).is_err());

        let mut fixture = peer_fixture();
        fixture.mirror.active.snapshot.registry.fleet_subnet_roots[1].status =
            FleetSubnetRootStatus::Draining;
        assert!(resolve(&fixture, fixture.caller, &fixture.service).is_err());

        let mut fixture = peer_fixture();
        fixture.mirror.root_entry.status = FleetSubnetRootStatus::Draining;
        assert!(resolve(&fixture, fixture.caller, &fixture.service).is_err());
    }

    #[test]
    fn forwarded_caller_wrong_service_and_local_owner_reject() {
        let fixture = peer_fixture();
        let forwarded = candid::Principal::from_slice(&[40; 29]);
        assert!(resolve(&fixture, forwarded, &fixture.service).is_err());

        let wrong_service = "users".parse().expect("Fleet service ID");
        assert!(resolve(&fixture, fixture.caller, &wrong_service).is_err());

        let mut fixture = peer_fixture();
        fixture.mirror.active.snapshot.registry.services[0].members[0].fleet_subnet_root =
            fixture.target_root.fleet_subnet_root;
        assert!(resolve(&fixture, fixture.caller, &fixture.service).is_err());
    }

    #[test]
    fn duplicate_raw_caller_membership_rejects() {
        let mut fixture = peer_fixture();
        let mut duplicate = fixture.mirror.active.snapshot.registry.services[0].clone();
        duplicate.service = "users".parse().expect("Fleet service ID");
        duplicate.component_spec = "users".parse().expect("Component Spec");
        duplicate.role = CanisterRole::new("user_hub");
        fixture
            .mirror
            .active
            .snapshot
            .registry
            .services
            .push(duplicate);

        assert!(resolve(&fixture, fixture.caller, &fixture.service).is_err());
    }

    fn resolve(
        fixture: &Fixture,
        caller: candid::Principal,
        service: &FleetServiceId,
    ) -> Result<FleetServicePeerRequesterView, InternalError> {
        FleetServicePeerOps::resolve(
            &fixture.target_root,
            &fixture.topology,
            &fixture.mirror,
            caller,
            service,
        )
    }

    struct Fixture {
        topology: ComponentTopology,
        target_root: FleetSubnetRootBinding,
        requester_root: FleetSubnetRootBinding,
        mirror: ValidatedRootFleetRegistryMirrorView,
        service: FleetServiceId,
        caller: candid::Principal,
        requester: FleetServiceComponentRequester,
    }

    fn peer_fixture() -> Fixture {
        let roots = root_fixture();
        let service = service_fixture(&roots.topology, &roots.authority, &roots.requester_root);
        let target_entry = root_entry(&roots.target_root, roots.release_set);
        let requester_entry = root_entry(&roots.requester_root, roots.release_set);
        let mirror = mirror(
            &roots.authority,
            &roots.target_root,
            target_entry,
            requester_entry,
            service.registry,
            service.directory,
        );
        Fixture {
            topology: roots.topology,
            target_root: roots.target_root,
            requester_root: roots.requester_root,
            mirror,
            service: service.service,
            caller: service.caller,
            requester: service.requester,
        }
    }

    struct RootFixture {
        topology: ComponentTopology,
        authority: FleetRegistryAuthority,
        target_root: FleetSubnetRootBinding,
        requester_root: FleetSubnetRootBinding,
        release_set: FleetSubnetRootReleaseSet,
    }

    fn root_fixture() -> RootFixture {
        let config: ConfigModel = toml::from_str(CONFIG).expect("peer config");
        let topology = ComponentTopology::compile(&config).expect("peer topology");
        let authority = authority();
        let admissions = topology
            .component_specs
            .iter()
            .map(|spec| ComponentSpecAdmission {
                component_spec: spec.component_spec.clone(),
                spec_hash: spec.spec_hash,
                maximum_root_instances: 4,
            })
            .collect::<Vec<_>>();
        let topology_digest = topology
            .project_for_admissions(&admissions)
            .expect("root projection")
            .digest()
            .expect("root topology digest");
        let limits = root_limits();
        let target_root = FleetSubnetRootBinding {
            authority: authority.clone(),
            placement_subnet: SubnetId::from_principal(candid::Principal::from_slice(&[20; 29])),
            fleet_subnet_root: candid::Principal::from_slice(&[21; 29]),
            component_admissions: admissions.clone(),
            component_topology_digest: topology_digest,
            limits: limits.clone(),
        };
        let requester_root = FleetSubnetRootBinding {
            authority: authority.clone(),
            placement_subnet: SubnetId::from_principal(candid::Principal::from_slice(&[22; 29])),
            fleet_subnet_root: candid::Principal::from_slice(&[23; 29]),
            component_admissions: admissions,
            component_topology_digest: topology_digest,
            limits,
        };
        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [24; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([25; 32]),
        };
        RootFixture {
            topology,
            authority,
            target_root,
            requester_root,
            release_set,
        }
    }

    struct ServiceFixture {
        service: FleetServiceId,
        caller: candid::Principal,
        registry: FleetServiceBinding,
        directory: FleetDirectoryService,
        requester: FleetServiceComponentRequester,
    }

    fn service_fixture(
        topology: &ComponentTopology,
        authority: &FleetRegistryAuthority,
        requester_root: &FleetSubnetRootBinding,
    ) -> ServiceFixture {
        let service: FleetServiceId = "projects".parse().expect("Fleet service ID");
        let caller = candid::Principal::from_slice(&[26; 29]);
        let component = ComponentInstanceId::from_root_allocation(
            authority.binding.fleet.fleet,
            authority.epoch,
            requester_root.fleet_subnet_root,
            1,
        );
        let group_placement = canic_core::ids::ComponentGroupPlacementId {
            deployment: "project_hubs".parse().expect("deployment ID"),
            ordinal: 0,
        };
        let member_path = canic_core::ids::ComponentGroupMemberPath::try_from(vec![
            "hub".parse().expect("member ID"),
        ])
        .expect("member path");
        let registry_member = FleetServiceComponentBinding {
            member_purpose: FleetServiceMemberPurpose::PoolMember,
            component,
            fleet_subnet_root: requester_root.fleet_subnet_root,
            canister_id: caller,
            group_placement: group_placement.clone(),
            member_path: member_path.clone(),
        };
        let placement = FleetServicePlacementPolicy {
            maximum_members_per_root: 1,
            minimum_distinct_roots: 1,
        };
        let registry = FleetServiceBinding {
            service: service.clone(),
            role: CanisterRole::new("project_hub"),
            component_spec: "projects".parse().expect("Component Spec"),
            mode: FleetServiceMode::ActivePool,
            placement,
            members: vec![registry_member.clone()],
        };
        let directory = FleetDirectoryService {
            service: service.clone(),
            role: registry.role.clone(),
            component_spec: registry.component_spec.clone(),
            mode: registry.mode,
            placement,
            members: vec![FleetDirectoryServiceComponent {
                member_purpose: registry_member.member_purpose,
                component: registry_member.component,
                fleet_subnet_root: registry_member.fleet_subnet_root,
                canister_id: registry_member.canister_id,
                group_placement: group_placement.clone(),
                member_path: member_path.clone(),
            }],
        };
        let requester_spec = topology
            .get(&"projects".parse().expect("Component Spec"))
            .expect("requester Spec");
        let requester = FleetServiceComponentRequester {
            service: service.clone(),
            member_purpose: registry_member.member_purpose,
            group_placement,
            member_path,
            component: ComponentBinding {
                authority: authority.clone(),
                component,
                component_spec: requester_spec.component_spec.clone(),
                spec_hash: requester_spec.spec_hash,
                role: requester_spec.component_role.clone(),
                placement_subnet: requester_root.placement_subnet,
                fleet_subnet_root: requester_root.fleet_subnet_root,
                canister_id: caller,
            },
        };
        ServiceFixture {
            service,
            caller,
            registry,
            directory,
            requester,
        }
    }

    fn mirror(
        authority: &FleetRegistryAuthority,
        target_root: &FleetSubnetRootBinding,
        target_entry: FleetSubnetRootEntry,
        requester_entry: FleetSubnetRootEntry,
        service: FleetServiceBinding,
        directory_service: FleetDirectoryService,
    ) -> ValidatedRootFleetRegistryMirrorView {
        let version = FleetRegistryVersion {
            authority: authority.clone(),
            revision: 2,
            content_hash: [27; 32],
        };
        let registry = FleetRegistry {
            authority: authority.clone(),
            revision: version.revision,
            component_specs: Vec::new(),
            fleet_subnet_roots: vec![target_entry.clone(), requester_entry.clone()],
            services: vec![service],
        };
        let directory = FleetDirectorySnapshot {
            provenance: FleetDirectoryProvenance {
                registry: version.clone(),
                source_fleet_subnet_root: target_root.fleet_subnet_root,
            },
            fleet_subnet_roots: vec![
                directory_root_entry(&target_entry),
                directory_root_entry(&requester_entry),
            ],
            services: vec![directory_service],
        };
        ValidatedRootFleetRegistryMirrorView {
            active: RootFleetRegistryActiveView {
                previous_registry: FleetRegistryVersion {
                    authority: authority.clone(),
                    revision: 1,
                    content_hash: [28; 32],
                },
                snapshot: FleetRegistrySnapshotResponse {
                    registry,
                    manifest: FleetRegistryManifest {
                        authority: authority.clone(),
                        revision: version.revision,
                        byte_length: 1,
                        content_hash: version.content_hash,
                    },
                    version,
                },
                directory,
            },
            root_entry: target_entry,
        }
    }

    fn authority() -> FleetRegistryAuthority {
        FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                        fleet_id: FleetId::from_generated_bytes([3; 32]),
                    },
                    app: AppId::from("toko"),
                },
                coordinator_subnet: SubnetId::from_principal(candid::Principal::from_slice(
                    &[4; 29],
                )),
                coordinator: candid::Principal::from_slice(&[5; 29]),
            },
            epoch: 1,
        }
    }

    fn root_limits() -> FleetSubnetRootLimits {
        FleetSubnetRootLimits {
            maximum_component_instances: 8,
            maximum_registry_bytes: 1_000_000,
            maximum_wasm_store_bytes: 1_000_000,
            canister_pool: FleetSubnetCanisterPoolConfig {
                minimum_size: 1,
                maximum_size: 10,
                canister_cycles: Cycles::new(5_000_000_000_000),
            },
            cycles_funding: CyclesFundingBudget {
                window_secs: 3_600,
                maximum_cycles: Cycles::new(1_000_000_000_000),
            },
            maximum_group_placements: 16,
        }
    }

    fn root_entry(
        root: &FleetSubnetRootBinding,
        release_set: FleetSubnetRootReleaseSet,
    ) -> FleetSubnetRootEntry {
        FleetSubnetRootEntry {
            placement_subnet: root.placement_subnet,
            fleet_subnet_root: root.fleet_subnet_root,
            component_admissions: root.component_admissions.clone(),
            component_topology_digest: root.component_topology_digest,
            active_release_set: release_set,
            limits: root.limits.clone(),
            status: FleetSubnetRootStatus::Active,
        }
    }

    const fn directory_root_entry(root: &FleetSubnetRootEntry) -> FleetSubnetRootDirectoryEntry {
        FleetSubnetRootDirectoryEntry {
            placement_subnet: root.placement_subnet,
            fleet_subnet_root: root.fleet_subnet_root,
            status: root.status,
        }
    }
}
