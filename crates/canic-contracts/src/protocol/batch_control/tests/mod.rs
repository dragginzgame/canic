//! Native framing does not become canonical plan or receipt hashing authority.
use super::*;
use crate::{
    dto::{
        batch_control as wire, component_provisioning as stored, error::Error,
        fleet_registry::FleetRegistryVersion,
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
fn reviewed_plan_preserves_canonical_bytes_and_original_operation() {
    let version = version();
    let original = stored::FleetComponentProvisioningPrepareRequest {
        operation_id: [10; 32],
        plan: stored::FleetComponentProvisioningPlan {
            fleet: version.authority.binding.fleet.clone(),
            fleet_registry: version,
            configuration_digest: ComponentDeploymentConfigurationDigest::from_bytes([11; 32]),
            operation: stored::FleetComponentProvisioningOperation::FreshInstall,
            directory_confirmation_roots: vec![Principal::from_slice(&[12; 29])],
            batches: Vec::new(),
        },
    };
    let bytes = ic_wire::native::encode_request(
        prepare::METHOD_ID,
        &wire::PrepareRequest::from(original.clone()),
        PLAN_LIMITS,
    )
    .unwrap();
    let decoded: wire::PrepareRequest =
        ic_wire::native::decode_request(prepare::METHOD_ID, &bytes, PLAN_LIMITS).unwrap();
    let restored = stored::FleetComponentProvisioningPrepareRequest::from(decoded);
    assert_eq!(restored, original);
    assert_eq!(
        crate::protocol::authority::encode(restored).unwrap(),
        crate::protocol::authority::encode(original).unwrap()
    );
    assert!(matches!(
        ic_wire::native::decode_request::<wire::PrepareRequest>(
            accept::METHOD_ID,
            &bytes,
            PLAN_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[test]
fn synchronization_retains_exact_progress_and_canonical_receipt() {
    let original = stored::RootComponentDirectorySynchronizationResponse {
        operation_id: [1; 32],
        plan_hash: [2; 32],
        source_fleet_registry: version(),
        published_fleet_registry: version(),
        fleet_subnet_root: Principal::from_slice(&[3; 29]),
        affected_component_count: u32::MAX,
        synchronized_component_count: 7,
        fleet_directory_content_hash: [4; 32],
        complete: false,
        synchronized_at_ns: Some(u64::MAX),
        receipt_content_hash: [5; 32],
    };
    let response: Result<wire::Synchronization, Error> = Ok(original.clone().into());
    let bytes =
        ic_wire::native::encode_response(synchronize::METHOD_ID, &response, RESPONSE_LIMITS)
            .unwrap();
    let decoded: Result<wire::Synchronization, Error> =
        ic_wire::native::decode_response(synchronize::METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    let restored = stored::RootComponentDirectorySynchronizationResponse::from(decoded.unwrap());
    assert_eq!(restored, original);
    assert_eq!(
        crate::protocol::authority::encode(restored).unwrap(),
        crate::protocol::authority::encode(original).unwrap()
    );
}

#[cfg(feature = "wire-schema")]
#[test]
fn provisioning_has_complete_native_metadata() {
    ic_wire::typescript::native_schema::<wire::PrepareRequest>().unwrap();
    ic_wire::typescript::native_schema::<wire::RootRequest>().unwrap();
    ic_wire::typescript::native_schema::<wire::SynchronizeRequest>().unwrap();
    ic_wire::typescript::native_schema::<Result<crate::dto::role::OperationReceipt, Error>>()
        .unwrap();
    ic_wire::typescript::native_schema::<Result<wire::Synchronization, Error>>().unwrap();
}
