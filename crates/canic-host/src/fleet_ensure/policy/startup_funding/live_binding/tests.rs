use super::*;
use crate::fleet_ensure::policy::startup_funding::tests::{funding_binding, hub_config};
use canic_core::{
    control_plane_support::config::ComponentDeploymentConfiguration,
    ids::{ComponentInstanceId, ReleaseSetDigest},
};

fn fixture() -> (
    ComponentTopology,
    StartupChildFundingBinding,
    StartupChildFundingBinding,
) {
    let mut topology = ComponentDeploymentConfiguration::compile(&hub_config())
        .unwrap()
        .component_topology;
    let spec = &mut topology.component_specs[0];
    // Dynamic descendants can reuse a role; topology validation forbids initial cycles only.
    let mut nested = spec.spawn_grants[0].clone();
    nested.parent_role = "shard".into();
    nested.initial_instances_per_parent = 0;
    spec.spawn_grants.push(nested);
    let top = funding_binding(spec);
    let mut child = top.clone();
    child.canister_id = Principal::from_slice(&[7]);
    child.parent = top.canister_id;
    child.parent_role = Some(top.role.clone());
    child.role = "shard".into();
    (topology, top, child)
}

#[test]
fn siblings_and_deep_children_resolve_without_inventory_order_assumptions() {
    let (topology, top, first) = fixture();
    let mut sibling = first.clone();
    sibling.canister_id = Principal::from_slice(&[8]);
    let mut nested = first.clone();
    nested.canister_id = Principal::from_slice(&[9]);
    nested.parent = first.canister_id;
    nested.parent_role = Some(first.role.clone());
    for bindings in [
        [&nested, &sibling, &first, &top],
        [&top, &first, &sibling, &nested],
    ] {
        let result = graph(
            &topology,
            top.release_set.release_build_id,
            bindings.into_iter(),
        );
        for binding in bindings {
            assert_eq!(result[&binding.canister_id], Ok(()));
        }
    }
}

#[test]
fn missing_parents_cycles_and_duplicate_principals_never_qualify_a_relay() {
    let (topology, top, first) = fixture();
    let release = top.release_set.release_build_id;
    let result = graph(&topology, release, [&first].into_iter());
    assert_eq!(
        result[&first.canister_id],
        Err(StartupUsageUnavailable::ParentNotObserved)
    );
    let result = graph(&topology, release, [&top, &top, &first].into_iter());
    assert_eq!(
        result[&first.canister_id],
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
    assert_eq!(
        result[&top.canister_id],
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
    let mut a = first.clone();
    let mut b = first;
    b.canister_id = Principal::from_slice(&[8]);
    a.parent = b.canister_id;
    a.parent_role = Some(b.role.clone());
    b.parent = a.canister_id;
    b.parent_role = Some(a.role.clone());
    let result = graph(&topology, release, [&top, &a, &b].into_iter());
    assert_eq!(result[&top.canister_id], Ok(()));
    for child in [&a, &b] {
        assert_eq!(
            result[&child.canister_id],
            Err(StartupUsageUnavailable::AuthorityMismatch)
        );
    }
}

#[test]
fn parent_join_binds_component_epoch_release_and_role_without_poisoning_valid_parent() {
    let (topology, top, child) = fixture();
    let mut variants = vec![child.clone(); 5];
    variants[0].component.component = ComponentInstanceId::from_generated_bytes([99; 32]);
    variants[1].component.authority.epoch += 1;
    variants[2].release_set.manifest_digest = ReleaseSetDigest::from_bytes([99; 32]);
    variants[3].component.placement_subnet = SubnetId::from_principal(Principal::from_slice(&[99]));
    variants[4].parent_role = Some("shard".into());
    for variant in variants {
        let result = graph(
            &topology,
            top.release_set.release_build_id,
            [&top, &variant].into_iter(),
        );
        assert_eq!(result[&top.canister_id], Ok(()));
        assert_eq!(
            result[&child.canister_id],
            Err(StartupUsageUnavailable::AuthorityMismatch)
        );
    }
}

#[test]
fn pool_custody_cannot_turn_a_descendant_into_a_root_funded_component() {
    let (topology, top, mut child) = fixture();
    child.parent = top.parent;
    assert_eq!(
        declared(&topology, top.release_set.release_build_id, &child),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
    child.parent_role = None;
    assert_eq!(
        declared(&topology, top.release_set.release_build_id, &child),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
}

/// Exercise selected placement against the existing generated-estate fixture's real desired shape.
pub(in crate::fleet_ensure) fn qualify_selected(desired: &DesiredFleet) {
    let bootstrap = desired.bootstrap.as_ref().unwrap();
    let root = &bootstrap.roots[0];
    let principal = |name: &str| {
        desired
            .canisters
            .iter()
            .find(|entry| entry.name == name)
            .unwrap()
            .principal
            .as_ref()
            .unwrap()
            .parse::<Principal>()
            .unwrap()
    };
    let mut binding = funding_binding(
        &bootstrap
            .component_deployment_configuration
            .component_topology
            .component_specs[0],
    );
    binding.release_set.release_build_id = bootstrap.release_build_id;
    binding.parent = principal(&root.root);
    binding.component.fleet_subnet_root = binding.parent;
    binding.component.placement_subnet = root.placement_subnet;
    binding.component.authority.binding = FleetCoordinatorBinding {
        fleet: FleetBinding {
            fleet: FleetKey {
                canonical_network_id: bootstrap.canonical_network_id,
                fleet_id: bootstrap.fleet_id,
            },
            app: bootstrap.app.clone(),
        },
        coordinator_subnet: bootstrap.coordinator_subnet,
        coordinator: principal(&bootstrap.coordinator),
    };
    assert_eq!(selected(desired, &root.root, &binding), Ok(()));
    qualify_current(desired, &root.root, &binding);
    crate::fleet_ensure::policy::startup_funding::relay_quote::qualify(
        desired, &root.root, &binding,
    );
    let mut withdrawn = desired.clone();
    withdrawn.bootstrap.as_mut().unwrap().roots[0]
        .component_admissions
        .clear();
    assert_eq!(
        selected(&withdrawn, &root.root, &binding),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
    let mut variants = vec![binding; 5];
    variants[0].component.authority.binding.fleet.app = "different-app".into();
    variants[1].component.authority.binding.coordinator = Principal::from_slice(&[97]);
    variants[2].component.placement_subnet = SubnetId::from_principal(Principal::from_slice(&[98]));
    variants[3].component.authority.epoch = 0;
    variants[4].component.fleet_subnet_root = Principal::from_slice(&[99]);
    for variant in variants {
        assert_eq!(
            selected(desired, &root.root, &variant),
            Err(StartupUsageUnavailable::AuthorityMismatch)
        );
    }
}

fn qualify_current(desired: &DesiredFleet, root_name: &str, binding: &StartupChildFundingBinding) {
    let root = &desired.bootstrap.as_ref().unwrap().roots[0];
    let registry = StartupFundingRegistry {
        authority: binding.component.authority.clone(),
        revision: 4,
        content_hash: [55; 32],
        roots: BTreeMap::from([(
            binding.component.fleet_subnet_root,
            StartupFundingPlacement {
                active: true,
                placement_subnet: root.placement_subnet,
                release_set: binding.release_set,
                component_admissions: root.component_admissions.clone(),
                component_topology_digest: root.component_topology_digest,
                limits: root.limits.clone(),
                funding: root.funding.clone(),
            },
        )]),
    };
    assert_eq!(current(desired, root_name, binding, &registry), Ok(()));
    let mut newer = registry.clone();
    newer.authority.epoch += 1;
    assert_eq!(
        current(desired, root_name, binding, &newer),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
    let mut removed = registry.clone();
    removed.roots.clear();
    assert_eq!(
        current(desired, root_name, binding, &removed),
        Err(StartupUsageUnavailable::AuthorityMismatch)
    );
    let mut variants = vec![registry; 6];
    for (index, registry) in variants.iter_mut().enumerate() {
        let placement = registry
            .roots
            .get_mut(&binding.component.fleet_subnet_root)
            .unwrap();
        match index {
            0 => placement.active = false,
            1 => placement.component_admissions.clear(),
            2 => placement.placement_subnet = Principal::from_slice(&[99]).into(),
            3 => placement.release_set.manifest_digest = ReleaseSetDigest::from_bytes([99; 32]),
            4 => placement.limits.maximum_component_instances += 1,
            5 => placement.funding.root_funding.maximum_automatic_grants += 1,
            _ => unreachable!(),
        }
        assert_eq!(
            current(desired, root_name, binding, registry),
            Err(StartupUsageUnavailable::PolicyTransition)
        );
    }
}
