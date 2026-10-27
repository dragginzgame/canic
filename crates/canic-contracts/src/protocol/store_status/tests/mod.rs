//! Native Store status preserves installation authority and operation evidence.
use super::*;
use crate::{
    dto::{
        component_directory::{FleetCoordinatorBinding, FleetRegistryAuthority},
        error::Error,
        role::OperationStatusRequest,
        store_status::Authority,
    },
    ids::{AppId, FleetBinding, ReleaseBuildId, ReleaseBuildNonce},
};

fn authority() -> Authority {
    Authority {
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: FleetBinding {
                    fleet: crate::test::support::fleet_key(1),
                    app: AppId::from("fixture"),
                },
                coordinator_subnet: ic_platform::Principal::from_slice(&[2]),
                coordinator: ic_platform::Principal::from_slice(&[3]),
                recovery_controllers: vec![ic_platform::Principal::from_slice(&[4])],
            },
            epoch: u64::MAX,
        },
        placement_subnet: ic_platform::Principal::from_slice(&[5]),
        fleet_subnet_root: ic_platform::Principal::from_slice(&[6]),
        wasm_store: ic_platform::Principal::from_slice(&[7]),
        installation_controller: ic_platform::Principal::from_slice(&[8]),
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([9; 32])),
        wasm_module_hash: [10; 32],
    }
}

#[test]
fn authority_round_trip_retains_all_principals_and_release_identity() {
    let expected = authority();
    let canonical = crate::ids::FleetSubnetWasmStoreAuthority::from(expected.clone());
    assert_eq!(Authority::from(canonical), expected);
    let reply: Result<_, Error> = Ok(expected.clone());
    let frame =
        ic_wire::native::encode_response(authority::METHOD_ID, &reply, RESPONSE_LIMITS).unwrap();
    let decoded: Result<Authority, Error> =
        ic_wire::native::decode_response(authority::METHOD_ID, &frame, RESPONSE_LIMITS).unwrap();
    assert_eq!(decoded.unwrap(), expected);
    assert!(matches!(
        ic_wire::native::decode_response::<Result<Authority, Error>>(
            activation::METHOD_ID,
            &frame,
            RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[test]
fn operation_queries_cannot_be_reinterpreted_across_status_lanes() {
    let request = OperationStatusRequest {
        operation_id: [17; 32],
    };
    for id in [activation::METHOD_ID, gc::METHOD_ID] {
        let mut frame = ic_wire::native::encode_request(id, &request, REQUEST_LIMITS).unwrap();
        let decoded: OperationStatusRequest =
            ic_wire::native::decode_request(id, &frame, REQUEST_LIMITS).unwrap();
        assert_eq!(decoded, request);
        let other = if id == activation::METHOD_ID {
            gc::METHOD_ID
        } else {
            activation::METHOD_ID
        };
        assert!(matches!(
            ic_wire::native::decode_request::<OperationStatusRequest>(
                other,
                &frame,
                REQUEST_LIMITS
            ),
            Err(ic_wire::Error::Identity)
        ));
        frame.push(0);
        assert!(matches!(
            ic_wire::native::decode_request::<OperationStatusRequest>(id, &frame, REQUEST_LIMITS),
            Err(ic_wire::Error::Trailing)
        ));
    }
}

#[cfg(feature = "wire-schema")]
#[test]
fn all_status_graphs_have_native_metadata() {
    ic_wire::typescript::native_schema::<Result<Authority, Error>>().unwrap();
    ic_wire::typescript::native_schema::<OperationStatusRequest>().unwrap();
    ic_wire::typescript::native_schema::<
        Result<crate::dto::runtime_reporting::FleetActivationStatusResponse, Error>,
    >()
    .unwrap();
    ic_wire::typescript::native_schema::<
        Result<crate::dto::template::WasmStoreGcOperationStatus, Error>,
    >()
    .unwrap();
}
