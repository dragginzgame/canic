use super::*;
use ic_platform_types::Principal;

#[test]
fn assigned_target_preserves_generation_and_refuses_invalid_principals() {
    let original = Allocation {
        slot: 2,
        generation: [7; 32],
    };
    let target = Principal::from_slice(&[4, 1]);
    let placement = |text| Placement::Assigned(original.clone(), text);
    assert_eq!(
        placement(target.to_text()).assigned_target().unwrap(),
        Some((&original, target))
    );
    for text in [
        target.to_text().to_uppercase(),
        Principal::anonymous().to_text(),
        Principal::management_canister().to_text(),
        Principal::from_slice(&[7, 2]).to_text(),
        "invalid".into(),
    ] {
        assert!(placement(text).assigned_target().is_err());
    }
    assert_eq!(
        Placement::Pending(original).assigned_target().unwrap(),
        None
    );
}

#[test]
fn keyed_methods_preserve_the_published_placement_frames() {
    use crate::method::Contract;
    use ic_wire::{Direction, Encoding};

    fn check<M: Contract<Input = Key, Output = Placement>>(name: &str, id: u64) {
        assert_eq!(M::NAME, name);
        for budget in [M::REQUEST, M::REPLY] {
            assert_eq!(
                (budget.bytes, budget.allocation, budget.nodes, budget.depth),
                (1024, 32768, 1024, 12)
            );
        }
        let key = [0xa5; 32];
        let previously_encoded = |direction| Encoding::Frame {
            method: id,
            direction,
        };
        let request =
            ic_wire::encode(&key, previously_encoded(Direction::Request), LIMITS).unwrap();
        assert_eq!(M::encode_request(&key).unwrap(), request);
        assert_eq!(M::decode_request(&request).unwrap(), key);

        let placement = Placement::Assigned(
            Allocation {
                slot: 3,
                generation: [0xf0; 32],
            },
            Principal::from_slice(&[4, 1]).to_text(),
        );
        let reply =
            ic_wire::encode(&placement, previously_encoded(Direction::Response), LIMITS).unwrap();
        assert_eq!(M::encode_reply(&placement).unwrap(), reply);
        assert_eq!(M::decode_reply(&reply).unwrap(), placement);
    }

    check::<AssignShard>("assign_shard", 91);
    check::<LookupShard>("lookup_shard", 92);
    check::<AssignInstance>(
        "assign_instance",
        ic_wire::method_id("canic.assign_instance.v1"),
    );
    check::<LookupInstance>(
        "lookup_instance",
        ic_wire::method_id("canic.lookup_instance.v1"),
    );
}

#[test]
fn standby_inspection_can_return_eight_distinct_claimed_assets() {
    use super::standby::{Asset, InspectStandby, Refill};
    use crate::method::Contract;
    use ic_platform_types::Principal;
    let root = Principal::from_slice(&[255; 29]);
    let history = (0..8u8)
        .map(|sequence| {
            (
                Refill {
                    root,
                    sequence,
                    cycles: 1_000_000_000_000,
                    expires_at_ns: u64::MAX,
                },
                Asset::Claimed(
                    Principal::from_slice(&[sequence; 29]),
                    super::Allocation {
                        slot: u32::from(sequence),
                        generation: [255; 32],
                    },
                ),
            )
        })
        .collect::<Vec<_>>();
    let reply = InspectStandby::encode_reply(&history).unwrap();
    assert_eq!(InspectStandby::decode_reply(&reply).unwrap(), history);
}
