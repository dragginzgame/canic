//! Qualify the retained authority graph and strict frame boundaries.
use super::*;
use crate::{
    dto::{
        coordinator_acknowledgements::RootAcknowledgement,
        error::Error,
        fleet_registry::{FleetRegistryVersion, FleetSubnetRootSnapshotAcknowledgement},
    },
    ids::*,
};
use ic_platform::principal::ExternalPrincipal as Principal;

#[test]
fn acknowledgement_inventory_preserves_exact_authority_and_rejects_wrong_frames() {
    let original = FleetSubnetRootSnapshotAcknowledgement {
        fleet_subnet_root: Principal::from_slice(&[9; 29]),
        version: FleetRegistryVersion {
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
        },
    };
    let request = crate::dto::coordinator_acknowledgements::AcknowledgeRequest {
        version: original.version.clone().into(),
    };
    let request_bytes = ic_wire::native::encode_request(
        acknowledge::METHOD_ID,
        &request,
        acknowledge::REQUEST_LIMITS,
    )
    .unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<
            crate::dto::coordinator_acknowledgements::AcknowledgeRequest,
        >(
            acknowledge::METHOD_ID,
            &request_bytes,
            acknowledge::REQUEST_LIMITS
        )
        .unwrap(),
        request
    );
    let reply: Result<Vec<RootAcknowledgement>, Error> = Ok(vec![original.clone().into()]);
    let bytes = ic_wire::native::encode_response(METHOD_ID, &reply, RESPONSE_LIMITS).unwrap();
    let decoded: Result<Vec<RootAcknowledgement>, Error> =
        ic_wire::native::decode_response(METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    let restored: FleetSubnetRootSnapshotAcknowledgement = decoded.unwrap().remove(0).into();
    assert_eq!(restored, original);
    assert_eq!(
        ic_wire::native::decode_response::<Result<Vec<RootAcknowledgement>, Error>>(
            METHOD_ID + 1,
            &bytes,
            RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    );
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(
        ic_wire::native::decode_response::<Result<Vec<RootAcknowledgement>, Error>>(
            METHOD_ID,
            &trailing,
            RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Trailing)
    );
    let request = ic_wire::native::encode_request(METHOD_ID, &(), REQUEST_LIMITS).unwrap();
    ic_wire::native::decode_request::<()>(METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<()>(
            METHOD_ID,
            &[0; REQUEST_LIMITS.bytes + 1],
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Limit)
    );
}

#[cfg(feature = "wire-schema")]
#[test]
fn acknowledgement_inventory_has_a_native_schema() {
    ic_wire::typescript::native_schema::<Result<Vec<RootAcknowledgement>, Error>>().unwrap();
}
