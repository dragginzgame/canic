//! Numeric identities retain the deployed enum's exact encoding and rejection boundary.
use super::{ChildId, Request};
use serde::{Deserialize, Serialize};

// Frozen public representation before numeric identities; includes its original type name.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
enum Slot {
    First,
    Second,
}

#[derive(Deserialize, Serialize)]
struct OldRequest {
    slot: Slot,
    value: u64,
}

#[test]
fn numeric_identities_preserve_deployed_wire_and_request_bytes() {
    for (index, old) in [Slot::First, Slot::Second].into_iter().enumerate() {
        let current = ChildId::new(index).unwrap();
        assert_eq!(current, ChildId::HISTORICAL_SERVICES[index]);
        assert_eq!(current.index(), index);
        let limits = crate::LIMITS;
        let payload = ic_wire::encode(&old, ic_wire::Encoding::Payload, limits).unwrap();
        assert_eq!(
            ic_wire::encode(&current, ic_wire::Encoding::Payload, limits).unwrap(),
            payload
        );
        assert_eq!(
            ic_wire::decode::<ChildId>(&payload, ic_wire::Encoding::Payload, limits).unwrap(),
            current
        );
        let encoding = ic_wire::Encoding::Frame {
            method: crate::application::STAGE_CHILD_CREATION,
            direction: ic_wire::Direction::Request,
        };
        let old_request = OldRequest {
            slot: old,
            value: 123,
        };
        let retained = ic_wire::encode(&old_request, encoding, limits).unwrap();
        let current_bytes = ic_wire::encode(
            &Request {
                slot: current,
                value: 123_u64,
            },
            encoding,
            limits,
        )
        .unwrap();
        assert_eq!(current_bytes, retained);
        let old_read: OldRequest = ic_wire::decode(&current_bytes, encoding, limits).unwrap();
        assert_eq!(old_read.slot, old_request.slot);
        let new_read: Request<u64> = ic_wire::decode(&retained, encoding, limits).unwrap();
        assert_eq!(new_read.slot, current);
        assert_eq!(new_read.value, old_request.value);
        let stage = crate::stored::Stage {
            approval: 123_u64,
            reference: crate::stored::Reference {
                store: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
                manifest: ic_wasm_store_protocol::Manifest {
                    release: ic_wasm_store_protocol::Release {
                        name: "app".into(),
                        version: "1".into(),
                    },
                    hash: [7; 32],
                    bytes: 1,
                    chunks: vec![[7; 32]],
                },
            },
        };
        let old_pin = ic_wire::encode(
            &(crate::stored::STAGE, &old_request.slot, &stage),
            ic_wire::Encoding::Payload,
            crate::stored::LIMITS,
        )
        .unwrap();
        assert_eq!(
            stage.pin(crate::stored::STAGE, current).operation,
            crate::digest(&old_pin)
        );
    }
    assert!(ChildId::FIRST < ChildId::SECOND);
}

#[test]
fn unsupported_identities_cannot_enter_runtime_or_retained_authority() {
    for value in [4, 10, 22, 23] {
        assert_eq!(ChildId::new(value), None);
        let bytes = ic_wire::encode(&value, ic_wire::Encoding::Payload, crate::LIMITS).unwrap();
        assert!(
            ic_wire::decode::<ChildId>(&bytes, ic_wire::Encoding::Payload, crate::LIMITS).is_err()
        );
    }
    #[derive(Serialize)]
    enum Candidate {
        First,
        Second,
        FirstProject,
        SecondProject,
        Unsupported,
    }
    // Wire identifies enum variants by discriminant, not their Rust names.
    for (candidate, accepted) in [
        (Candidate::First, true),
        (Candidate::Second, true),
        (Candidate::FirstProject, true),
        (Candidate::SecondProject, true),
        (Candidate::Unsupported, false),
    ] {
        let bytes = ic_wire::encode(&candidate, ic_wire::Encoding::Payload, crate::LIMITS).unwrap();
        assert_eq!(
            ic_wire::decode::<ChildId>(&bytes, ic_wire::Encoding::Payload, crate::LIMITS).is_ok(),
            accepted
        );
    }
}

#[test]
fn placement_slots_round_trip_and_cannot_enter_ordinary_requests() {
    for slot in ChildId::INSTANCES.into_iter().chain(ChildId::SHARDS) {
        assert!(ChildId::new(slot.index()).is_none());
        let frame = ic_wire::Encoding::Payload;
        let id = ic_wire::encode(&slot, frame, crate::LIMITS).unwrap();
        assert_eq!(
            ic_wire::decode::<ChildId>(&id, frame, crate::LIMITS).unwrap(),
            slot
        );
        let request =
            ic_wire::encode(&Request { slot, value: 1_u64 }, frame, crate::LIMITS).unwrap();
        assert!(ic_wire::decode::<Request<u64>>(&request, frame, crate::LIMITS).is_err());
    }
}
