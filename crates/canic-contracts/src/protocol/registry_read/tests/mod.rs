//! Exact query identities and complete native graph registration.
use super::*;
use crate::dto::component_registry::ComponentRegistryPartitionRequest;

#[test]
fn partition_request_retains_component_and_rejects_other_read_identities() {
    let request = ComponentRegistryPartitionRequest {
        component: crate::ids::ComponentInstanceId::from_generated_bytes([7; 32]),
    };
    let bytes =
        ic_wire::native::encode_request(partition::METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<ComponentRegistryPartitionRequest>(
            partition::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        )
        .unwrap(),
        request
    );
    for other in [status::METHOD_ID, active_partition::METHOD_ID] {
        assert!(matches!(
            ic_wire::native::decode_request::<ComponentRegistryPartitionRequest>(
                other,
                &bytes,
                REQUEST_LIMITS
            ),
            Err(ic_wire::Error::Identity)
        ));
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(
        ic_wire::native::decode_request::<ComponentRegistryPartitionRequest>(
            partition::METHOD_ID,
            &trailing,
            REQUEST_LIMITS
        )
        .is_err()
    );
}

#[cfg(feature = "wire-schema")]
#[test]
fn registry_reads_register_complete_native_graphs() {
    use crate::dto::{component_control, component_registry, error::Error, registry_read};
    ic_wire::typescript::native_schema::<component_control::PreparationRequest>().unwrap();
    ic_wire::typescript::native_schema::<ComponentRegistryPartitionRequest>().unwrap();
    ic_wire::typescript::native_schema::<component_registry::ComponentRegistryActivePartitionRequest>().unwrap();
    ic_wire::typescript::native_schema::<Result<component_control::RegistryStatus, Error>>()
        .unwrap();
    ic_wire::typescript::native_schema::<Result<registry_read::Partition, Error>>().unwrap();
    ic_wire::typescript::native_schema::<Result<registry_read::ActivePartition, Error>>().unwrap();
}
