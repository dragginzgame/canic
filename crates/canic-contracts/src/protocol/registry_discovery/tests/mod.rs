//! Exact discovery identity, lossless projection and strict frame separation.
use super::*;
use crate::{
    dto::{
        error::Error,
        fleet_registry::{FleetRegistryManifest, FleetRegistryVersion},
        registry_discovery::{RegistryManifest, RegistryVersion},
    },
    ids::*,
};
use ic_platform::principal::ExternalPrincipal as Principal;

fn version() -> FleetRegistryVersion {
    FleetRegistryVersion {
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                        fleet_id: FleetId::from_generated_bytes([7; 32]),
                    },
                    app: AppId::from("demo"),
                },
                coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[2; 29])),
                coordinator: Principal::from_slice(&[3; 29]),
                recovery_controllers: vec![Principal::from_slice(&[4; 29])],
            },
            epoch: u64::MAX,
        },
        revision: u64::MAX,
        content_hash: [255; 32],
    }
}

#[test]
fn discovery_frames_retain_original_registry_authority_and_manifest() {
    let original = version();
    let reply: Result<RegistryVersion, Error> = Ok(original.clone().into());
    let bytes =
        ic_wire::native::encode_response(super::version::METHOD_ID, &reply, RESPONSE_LIMITS)
            .unwrap();
    let decoded: Result<RegistryVersion, Error> =
        ic_wire::native::decode_response(super::version::METHOD_ID, &bytes, RESPONSE_LIMITS)
            .unwrap();
    assert_eq!(FleetRegistryVersion::from(decoded.unwrap()), original);
    let original = FleetRegistryManifest {
        authority: original.authority,
        revision: original.revision,
        byte_length: u64::MAX,
        content_hash: original.content_hash,
    };
    let reply: Result<RegistryManifest, Error> = Ok(original.clone().into());
    let bytes =
        ic_wire::native::encode_response(manifest::METHOD_ID, &reply, RESPONSE_LIMITS).unwrap();
    let decoded: Result<RegistryManifest, Error> =
        ic_wire::native::decode_response(manifest::METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    assert_eq!(FleetRegistryManifest::from(decoded.unwrap()), original);
    assert_eq!(
        ic_wire::native::decode_response::<Result<RegistryManifest, Error>>(
            super::version::METHOD_ID,
            &bytes,
            RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    );
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(
        ic_wire::native::decode_response::<Result<RegistryManifest, Error>>(
            manifest::METHOD_ID,
            &trailing,
            RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Trailing)
    );
}

#[test]
fn discovery_queries_require_exact_unit_frames() {
    for id in [
        manifest::METHOD_ID,
        super::version::METHOD_ID,
        snapshot::METHOD_ID,
        root_inventory::METHOD_ID,
    ] {
        let bytes = ic_wire::native::encode_request(id, &(), REQUEST_LIMITS).unwrap();
        ic_wire::native::decode_request::<()>(id, &bytes, REQUEST_LIMITS).unwrap();
        assert_eq!(
            ic_wire::native::decode_request::<()>(id + 2, &bytes, REQUEST_LIMITS),
            Err(ic_wire::Error::Identity)
        );
        assert_eq!(
            ic_wire::native::decode_response::<()>(id, &bytes, RESPONSE_LIMITS),
            Err(ic_wire::Error::Identity)
        );
    }
}

#[test]
fn full_registry_round_trip_preserves_canonical_storage_hash_input() {
    use crate::dto::{fleet_registry, fleet_snapshot};
    let authority = version().authority;
    let selectors = vec![
        FleetAdmissionSelector::Fleet,
        FleetAdmissionSelector::ComponentSpec("worker".parse().unwrap()),
        FleetAdmissionSelector::ComponentInstance(ComponentInstanceId::from_generated_bytes(
            [8; 32],
        )),
        FleetAdmissionSelector::FleetSubnetRoot(SubnetId::from_principal(Principal::from_slice(
            &[9; 29],
        ))),
    ];
    let original = fleet_registry::FleetRegistry {
        admission: FleetAdmissionPolicy {
            schema_version: 1,
            fleet: authority.binding.fleet.clone(),
            generation: u64::MAX,
            fleet_principals: vec![Principal::anonymous(), Principal::from_slice(&[255; 29])],
            rules: selectors
                .into_iter()
                .map(|selector| FleetAdmissionRule {
                    selector,
                    principals: vec![Principal::from_slice(&[7; 29])],
                })
                .collect(),
            policy_digest: [5; 32],
        },
        authority,
        revision: u64::MAX,
        component_specs: vec![],
        fleet_subnet_roots: vec![],
        services: vec![],
    };
    let canonical = serde_json::to_vec(&original).unwrap();
    let reply: Result<fleet_snapshot::FleetRegistry, Error> = Ok(original.clone().into());
    let frame =
        ic_wire::native::encode_response(snapshot::METHOD_ID, &reply, RESPONSE_LIMITS).unwrap();
    let decoded: Result<fleet_snapshot::FleetRegistry, Error> =
        ic_wire::native::decode_response(snapshot::METHOD_ID, &frame, RESPONSE_LIMITS).unwrap();
    let restored: fleet_registry::FleetRegistry = decoded.unwrap().into();
    assert_eq!(restored, original);
    assert_eq!(serde_json::to_vec(&restored).unwrap(), canonical);
}

#[test]
fn root_inventory_frame_preserves_exact_registry_and_physical_counts() {
    use crate::dto::{
        fleet_registry::FleetSubnetRootStatus, fleet_snapshot::RootInventory,
        fleet_subnet_root::FleetSubnetRootCanisterSummary,
    };
    let original = FleetSubnetRootCanisterSummary {
        fleet_registry: version(),
        placement_subnet: SubnetId::from_principal(Principal::from_slice(&[8; 29])),
        fleet_subnet_root: Principal::from_slice(&[9; 29]),
        status: FleetSubnetRootStatus::Draining,
        infrastructure_canisters: u32::MAX,
        component_canisters: u32::MAX,
        pooled_canisters: u32::MAX,
        total_canisters: u32::MAX,
    };
    let frame = ic_wire::native::encode_response(
        root_inventory::METHOD_ID,
        &Ok::<RootInventory, Error>(original.clone().into()),
        RESPONSE_LIMITS,
    )
    .unwrap();
    let decoded: Result<RootInventory, Error> =
        ic_wire::native::decode_response(root_inventory::METHOD_ID, &frame, RESPONSE_LIMITS)
            .unwrap();
    assert_eq!(
        FleetSubnetRootCanisterSummary::from(decoded.unwrap()),
        original
    );
}

#[cfg(feature = "wire-schema")]
#[test]
fn complete_registry_graph_has_native_schema_metadata() {
    ic_wire::typescript::native_schema::<Result<crate::dto::fleet_snapshot::FleetRegistry, Error>>(
    )
    .unwrap();
    ic_wire::typescript::native_schema::<Result<crate::dto::fleet_snapshot::RootInventory, Error>>(
    )
    .unwrap();
}
