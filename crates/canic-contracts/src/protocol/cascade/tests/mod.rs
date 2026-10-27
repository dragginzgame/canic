//! Lossless topology projection and strict native framing for both cascade operations.
use super::*;
use crate::dto::{cascade as stored, cascade_transport as native};
use ic_platform::principal::ExternalPrincipal;

#[test]
fn topology_preserves_parent_links_roles_and_canonical_internal_bytes() {
    let parent = ExternalPrincipal::from_slice(&[1; 29]);
    let child = ExternalPrincipal::from_slice(&[2; 29]);
    let original = stored::TopologySnapshotInput {
        parents: vec![stored::TopologyPathNode {
            pid: child,
            role: crate::ids::CanisterRole::WASM_STORE,
            parent_pid: Some(parent),
        }],
        children_map: vec![stored::TopologyChildren {
            parent_pid: child,
            children: vec![],
        }],
    };
    let canonical = crate::protocol::authority::encode(&original).unwrap();
    let frame = ic_wire::native::encode_request(
        topology::METHOD_ID,
        &native::TopologySnapshot::from(original),
        REQUEST_LIMITS,
    )
    .unwrap();
    let decoded: native::TopologySnapshot =
        ic_wire::native::decode_request(topology::METHOD_ID, &frame, REQUEST_LIMITS).unwrap();
    let restored = stored::TopologySnapshotInput::from(decoded);
    assert_eq!(
        crate::protocol::authority::encode(&restored).unwrap(),
        canonical
    );
    assert_eq!(restored.parents[0].parent_pid, Some(parent));
    assert_eq!(restored.parents[0].pid, child);
    assert!(matches!(
        ic_wire::native::decode_request::<native::TopologySnapshot>(
            state::METHOD_ID,
            &frame,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
    let mut trailing = frame;
    trailing.push(0);
    assert!(matches!(
        ic_wire::native::decode_request::<native::TopologySnapshot>(
            topology::METHOD_ID,
            &trailing,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Trailing)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn both_cascade_methods_have_complete_metadata() {
    ic_wire::typescript::native_schema::<stored::StateSnapshotInput>().unwrap();
    ic_wire::typescript::native_schema::<native::TopologySnapshot>().unwrap();
    ic_wire::typescript::native_schema::<
        Result<crate::dto::fleet_state_control::CascadeReport, crate::dto::error::Error>,
    >()
    .unwrap();
}
