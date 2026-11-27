use super::{ChildId, Request};

#[test]
fn services_round_trip_and_prior_ids_refuse() {
    for ordinal in [0, 1, 256, 32768] {
        let id = ChildId::service(ordinal).unwrap();
        assert_eq!(id.service_index(), Some(ordinal));
        let bytes = ic_wire::encode(&id, ic_wire::Encoding::Payload, crate::LIMITS).unwrap();
        assert_eq!(
            ic_wire::decode::<ChildId>(&bytes, ic_wire::Encoding::Payload, crate::LIMITS).unwrap(),
            id
        );
    }
    for tag in 0..=5 {
        assert!(
            ic_wire::decode::<ChildId>(&[tag, 20], ic_wire::Encoding::Payload, crate::LIMITS)
                .is_err()
        );
    }
}

#[test]
fn placement_slots_round_trip_and_cannot_enter_ordinary_requests() {
    for slot in ChildId::INSTANCES.into_iter().chain(ChildId::SHARDS) {
        assert!(slot.service_index().is_none());
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
