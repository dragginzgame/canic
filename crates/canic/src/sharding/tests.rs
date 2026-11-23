use super::*;
use ic_platform_types::Principal;

#[test]
fn instance_clients_preserve_local_dispatch_failure() {
    use crate::method::CallError;
    use std::{
        future::Future,
        pin::pin,
        task::{Context, Poll, Waker},
    };

    let root = Principal::from_slice(&[4, 1]);
    let key = [42; 32];
    let mut context = Context::from_waker(Waker::noop());
    assert!(matches!(
        pin!(assign_instance(root, key)).poll(&mut context),
        Poll::Ready(Err(CallError::Dispatch(_)))
    ));
    assert!(matches!(
        pin!(lookup_instance(root, key)).poll(&mut context),
        Poll::Ready(Err(CallError::Dispatch(_)))
    ));
}

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
    assert_eq!(
        placement(target.to_text()).into_outcome(),
        Ok(PlacementOutcome::Assigned(original.clone(), target))
    );
    for text in [
        target.to_text().to_uppercase(),
        Principal::anonymous().to_text(),
        Principal::management_canister().to_text(),
        Principal::from_slice(&[7, 2]).to_text(),
        "invalid".into(),
    ] {
        assert!(placement(text.clone()).assigned_target().is_err());
        assert!(placement(text).into_outcome().is_err());
    }
    assert_eq!(
        Placement::Pending(original).assigned_target().unwrap(),
        None
    );
}

#[test]
fn owned_outcomes_preserve_each_unassigned_state() {
    let allocation = Allocation {
        slot: 23,
        generation: [9; 32],
    };
    for (reply, expected) in [
        (Placement::Unknown, PlacementOutcome::Unknown),
        (Placement::Full, PlacementOutcome::Full),
        (Placement::Busy, PlacementOutcome::Busy),
        (Placement::GrowthRefused, PlacementOutcome::GrowthRefused),
        (
            Placement::Pending(allocation.clone()),
            PlacementOutcome::Pending(allocation.clone()),
        ),
        (
            Placement::Unavailable(allocation.clone(), Some("retired".into())),
            PlacementOutcome::Unavailable(allocation, Some("retired".into())),
        ),
    ] {
        assert_eq!(reply.into_outcome(), Ok(expected));
    }
}

#[test]
fn keyed_methods_scope_only_shared_shard_requests_by_pool() {
    use crate::method::Contract;
    use ic_wire::{Direction, Encoding};

    fn check<M: Contract<Output = Placement>>(name: &str, id: u64, key: &M::Input)
    where
        M::Input: PartialEq + std::fmt::Debug,
    {
        assert_eq!(M::NAME, name);
        for budget in [M::REQUEST, M::REPLY] {
            assert_eq!(
                (budget.bytes, budget.allocation, budget.nodes, budget.depth),
                (1024, 32768, 1024, 12)
            );
        }
        let previously_encoded = |direction| Encoding::Frame {
            method: id,
            direction,
        };
        let request = ic_wire::encode(key, previously_encoded(Direction::Request), LIMITS).unwrap();
        assert_eq!(M::encode_request(key).unwrap(), request);
        assert_eq!(M::decode_request(&request).unwrap(), *key);

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

    check::<AssignShard>("assign_shard", 91, &("users".into(), [0xa5; 32]));
    check::<LookupShard>("lookup_shard", 92, &("users".into(), [0xa5; 32]));
    check::<AssignInstance>(
        "assign_instance",
        ic_wire::method_id("canic.assign_instance.v1"),
        &[0xa5; 32],
    );
    check::<LookupInstance>(
        "lookup_instance",
        ic_wire::method_id("canic.lookup_instance.v1"),
        &[0xa5; 32],
    );
}

#[test]
fn standby_inspection_covers_claimed_assets_and_retained_transfer_history() {
    use super::standby::{Asset, InspectStandby, Refill};
    use crate::method::Contract;
    use ic_platform_types::Principal;
    let root = Principal::from_slice(&[255; 29]);
    let history = (0..8u8)
        .map(|sequence| {
            (
                super::standby::Source::Refill(Refill {
                    root,
                    sequence,
                    cycles: 1_000_000_000_000,
                    expires_at_ns: u64::MAX,
                }),
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
    let retired = history
        .iter()
        .map(|(_, asset)| {
            let Asset::Claimed(shard, allocation) = asset else {
                unreachable!()
            };
            (
                super::standby::Source::Shard(super::ShardTarget {
                    shard: *shard,
                    allocation: allocation.clone(),
                }),
                Asset::Reset(
                    *shard,
                    allocation.clone(),
                    u64::MAX,
                    super::standby::ResetPhase::StartIssued,
                ),
            )
        })
        .collect();
    let exported = (0..40u8)
        .map(|index| {
            let spare = Principal::from_slice(&[index; 29]);
            (
                super::standby::Source::Worker(super::ShardTarget {
                    shard: spare,
                    allocation: super::Allocation {
                        slot: u32::from(index),
                        generation: [255; 32],
                    },
                }),
                Asset::Transfer(
                    spare,
                    root,
                    u64::MAX,
                    super::standby::TransferPhase::Confirmed,
                ),
            )
        })
        .collect();
    for rows in [history, retired, exported] {
        let reply = InspectStandby::encode_reply(&rows).unwrap();
        assert_eq!(InspectStandby::decode_reply(&reply).unwrap(), rows);
    }
}
