//! Target-bound initializer persistence, refusal and interrupted install evidence.

use super::*;
use canic_contracts::dto::component_registry::{
    ComponentApplicationInitialization, MAX_COMPONENT_APPLICATION_INIT_BYTES,
    RootComponentInitializationRequest,
};

fn request(target: Principal, arguments: Vec<u8>) -> RootComponentInitializationRequest {
    RootComponentInitializationRequest {
        operation_id: [12; 32],
        initialization: ComponentApplicationInitialization {
            target_canister: target,
            arguments,
        },
    }
}

#[test]
fn initialization_refuses_target_drift_and_bounds_without_state_changes() {
    let (_, _, target) = prepared_created_allocation();
    let before = RootComponentRegistryStore::export();
    for input in [
        request(Principal::anonymous(), vec![1]),
        request(target, vec![]),
        request(target, vec![1; MAX_COMPONENT_APPLICATION_INIT_BYTES + 1]),
    ] {
        let error = ComponentRegistryOps::bind_application_initialization(input).unwrap_err();
        assert!(
            [
                InternalError::conflict().public_code(),
                InternalError::invalid_input().public_code()
            ]
            .contains(&error.public_code())
        );
        assert_eq!(RootComponentRegistryStore::export(), before);
    }
}

#[test]
fn initialization_freezes_exact_bytes_and_recovers_the_same_install_intent() {
    let (root, created, target) = prepared_created_allocation();
    let input = request(target, vec![0, 255, 3]);
    let bound = ComponentRegistryOps::bind_application_initialization(input.clone()).unwrap();
    assert_eq!(
        bound.application_initialization,
        Some(input.initialization.clone())
    );
    let before = RootComponentRegistryStore::export();
    assert_eq!(
        ComponentRegistryOps::bind_application_initialization(input.clone()).unwrap(),
        bound
    );
    assert_eq!(RootComponentRegistryStore::export(), before);
    let mut changed = input.clone();
    changed.initialization.arguments.push(4);
    assert_eq!(
        ComponentRegistryOps::bind_application_initialization(changed)
            .unwrap_err()
            .public_code(),
        InternalError::conflict().public_code()
    );
    assert_eq!(RootComponentRegistryStore::export(), before);
    let plan = RootComponentInstallPlan {
        application_init_hash: ComponentRegistryOps::application_init_hash(Some(
            &input.initialization,
        ))
        .unwrap(),
        fixture_grant_revision: None,
        raw_module_hash: [20; 32],
        protocol_profile_digest: ProtocolProfileDigest::from_bytes([23; 32]),
        chunk_hashes: vec![vec![21; 32]],
        binding: ComponentBinding {
            authority: root.authority,
            component: created.component,
            component_spec: created.component_spec,
            spec_hash: created.spec_hash,
            role: created.role,
            placement_subnet: root.placement_subnet,
            fleet_subnet_root: root.fleet_subnet_root,
            canister_id: target,
        },
        maximum_registry_bytes: 16_777_216,
    };
    let mut stale = plan.clone();
    stale.application_init_hash = None;
    assert_eq!(
        ComponentRegistryOps::validate_install_capacity([12; 32], &stale)
            .unwrap_err()
            .public_code(),
        InternalError::conflict().public_code()
    );
    let created_bytes = ComponentRegistryOps::current().unwrap().encoded_bytes;
    ComponentRegistryOps::begin_install(
        [12; 32],
        plan.clone(),
        ReplayCostGuardSettlement {
            quota_intent_id: IntentId(23),
            reservation_intent_id: IntentId(24),
        },
    )
    .unwrap();
    let retained = RootComponentRegistryStore::export();
    RootComponentRegistryStore::import(retained);
    ComponentRegistryOps::renew_install_intent(
        [12; 32],
        &plan,
        ReplayCostGuardSettlement {
            quota_intent_id: IntentId(25),
            reservation_intent_id: IntentId(26),
        },
    )
    .unwrap();
    ComponentRegistryOps::mark_installed([12; 32]).unwrap();
    let installed = ComponentRegistryOps::bind_application_initialization(input.clone()).unwrap();
    assert_eq!(
        installed.application_initialization,
        Some(input.initialization.clone())
    );
    assert!(ComponentRegistryOps::current().unwrap().encoded_bytes > created_bytes);
    let after = RootComponentRegistryStore::export();
    assert_eq!(
        ComponentRegistryOps::bind_application_initialization(input).unwrap(),
        installed
    );
    assert_eq!(RootComponentRegistryStore::export(), after);
}

#[test]
fn initialization_capacity_is_accounted_before_publication() {
    let (_, _, target) = prepared_created_allocation();
    let mut current = RootComponentRegistryStore::export();
    let metadata = current.current.as_mut().unwrap();
    metadata.root.limits.maximum_registry_bytes = metadata.encoded_bytes;
    RootComponentRegistryStore::import(current);
    let before = RootComponentRegistryStore::export();
    assert!(
        ComponentRegistryOps::bind_application_initialization(request(
            target,
            vec![1; MAX_COMPONENT_APPLICATION_INIT_BYTES]
        ))
        .unwrap_err()
        .is_public_resource_exhausted()
    );
    assert_eq!(RootComponentRegistryStore::export(), before);
}

#[test]
fn initialization_accepts_the_full_bounded_payload() {
    let (_, _, target) = prepared_created_allocation();
    let input = request(target, vec![255; MAX_COMPONENT_APPLICATION_INIT_BYTES]);
    let bound = ComponentRegistryOps::bind_application_initialization(input.clone()).unwrap();
    assert_eq!(bound.application_initialization, Some(input.initialization));
    let record = RootComponentRegistryStore::allocation([12; 32]).unwrap();
    assert!(
        RootComponentRegistryStore::allocation_entry_bytes(&record)
            <= RootComponentRegistryStore::allocation_record_max_bytes()
    );
}
