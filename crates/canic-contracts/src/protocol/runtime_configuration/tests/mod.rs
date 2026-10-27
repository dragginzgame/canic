//! Runtime configuration schema and validated deployment labels keep canonical constraints.
use super::*;
use crate::deployment::{ComponentDeploymentLabelKey, ComponentDeploymentLabelValue};

#[test]
fn native_labels_preserve_utf8_bounds_and_reject_invalid_text() {
    for value in ["a", "é", "room one"] {
        let label = ComponentDeploymentLabelValue::try_from(value.to_owned()).unwrap();
        let bytes = ic_wire::native::encode_request(METHOD_ID, &label, REQUEST_LIMITS).unwrap();
        let decoded: ComponentDeploymentLabelValue =
            ic_wire::native::decode_request(METHOD_ID, &bytes, REQUEST_LIMITS).unwrap();
        assert_eq!(decoded, label);
    }
    for value in ["", "line\nbreak", "\u{85}"] {
        let bytes = ic_wire::native::encode_request(METHOD_ID, &value, REQUEST_LIMITS).unwrap();
        assert!(
            ic_wire::native::decode_request::<ComponentDeploymentLabelValue>(
                METHOD_ID,
                &bytes,
                REQUEST_LIMITS
            )
            .is_err()
        );
    }
    let value = "é".repeat(65);
    let bytes = ic_wire::native::encode_request(METHOD_ID, &value, REQUEST_LIMITS).unwrap();
    assert!(
        ic_wire::native::decode_request::<ComponentDeploymentLabelValue>(
            METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        )
        .is_err()
    );
    for key in ["", "contains space", "é"] {
        let bytes = ic_wire::native::encode_request(METHOD_ID, &key, REQUEST_LIMITS).unwrap();
        assert!(
            ic_wire::native::decode_request::<ComponentDeploymentLabelKey>(
                METHOD_ID,
                &bytes,
                REQUEST_LIMITS
            )
            .is_err()
        );
    }
}

#[cfg(feature = "wire-schema")]
#[test]
fn runtime_schema_contains_complete_directory_and_group_graph() {
    ic_wire::typescript::native_schema::<
        crate::dto::runtime_configuration::ComponentRuntimeDirectoryPreparationRequest,
    >()
    .unwrap();
    ic_wire::typescript::native_schema::<
        Result<crate::dto::role::OperationReceipt, crate::dto::error::Error>,
    >()
    .unwrap();
}

#[test]
fn runtime_request_preserves_canonical_authority_bytes_and_operation_identity() {
    use crate::{
        dto::{component_registry as stored, fleet_registry},
        ids::{
            AppId, CanisterRole, CanonicalNetworkId, ComponentBinding, ComponentInstanceId,
            ComponentSpecId, FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey,
            FleetRegistryAuthority, SubnetId,
        },
    };
    use ic_platform::principal::ExternalPrincipal;
    let principal = ExternalPrincipal::from_slice(&[7; 29]);
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([3; 32]),
                },
                app: AppId::from("fixture"),
            },
            coordinator_subnet: SubnetId::from_principal(principal),
            coordinator: principal,
            recovery_controllers: vec![principal],
        },
        epoch: u64::MAX,
    };
    let binding = ComponentBinding {
        authority: authority.clone(),
        component: ComponentInstanceId::from_generated_bytes([4; 32]),
        component_spec: "fixture".parse::<ComponentSpecId>().unwrap(),
        spec_hash: [5; 32],
        role: CanisterRole::owned("fixture".to_owned()),
        placement_subnet: SubnetId::from_principal(principal),
        fleet_subnet_root: principal,
        canister_id: principal,
    };
    let original = stored::ComponentRuntimeDirectoryPreparationRequest {
        operation_id: [9; 32],
        authority: stored::ComponentRuntimeDirectoryAuthority {
            fleet: fleet_registry::FleetDirectorySnapshot {
                provenance: fleet_registry::FleetDirectoryProvenance {
                    registry: fleet_registry::FleetRegistryVersion {
                        authority,
                        revision: u64::MAX,
                        content_hash: [6; 32],
                    },
                    source_fleet_subnet_root: principal,
                },
                fleet_subnet_roots: vec![],
                services: vec![],
            },
            component: stored::ComponentDirectoryHead {
                provenance: stored::ComponentDirectoryProvenance {
                    component: binding,
                    source_fleet_subnet_root: principal,
                    component_registry_revision: u64::MAX,
                    component_registry_content_hash: [8; 32],
                    synchronized_at_ns: u64::MAX,
                },
                descendant_count: 0,
            },
            component_group: None,
        },
        direct_children: vec![],
    };
    let canonical = serde_json::to_vec(&original).unwrap();
    let native =
        crate::dto::runtime_configuration::ComponentRuntimeDirectoryPreparationRequest::from(
            original.clone(),
        );
    let bytes = ic_wire::native::encode_request(METHOD_ID, &native, REQUEST_LIMITS).unwrap();
    let decoded: crate::dto::runtime_configuration::ComponentRuntimeDirectoryPreparationRequest =
        ic_wire::native::decode_request(METHOD_ID, &bytes, REQUEST_LIMITS).unwrap();
    let restored = stored::ComponentRuntimeDirectoryPreparationRequest::from(decoded);
    assert_eq!(restored, original);
    assert_eq!(serde_json::to_vec(&restored).unwrap(), canonical);
    assert!(matches!(
        ic_wire::native::decode_request::<
            crate::dto::runtime_configuration::ComponentRuntimeDirectoryPreparationRequest,
        >(
            crate::protocol::cascade::state::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}
