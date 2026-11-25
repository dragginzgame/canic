use super::*;
use ic_platform_types::Principal;

#[test]
fn placement_clients_preserve_local_refusal_and_dispatch_failure() {
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
        pin!(client::acquire_instance(root, key)).poll(&mut context),
        Poll::Ready(Err(payment::AcquireError::Call(CallError::Dispatch(_))))
    ));
    assert!(matches!(
        pin!(client::observe_instance(root, key)).poll(&mut context),
        Poll::Ready(Err(CallError::Dispatch(_)))
    ));
    let fund = |demand, attached| payment::acquire_placement(root, demand, 1, attached);
    assert!(matches!(
        pin!(fund(payment::Demand::Instance(key), 2)).poll(&mut context),
        Poll::Ready(Err(payment::AcquireError::AttachmentExceedsMaximum))
    ));
    assert!(matches!(
        pin!(fund(payment::Demand::Instance(key), 0)).poll(&mut context),
        Poll::Ready(Err(payment::AcquireError::Call(CallError::Dispatch(_))))
    ));
    assert!(matches!(
        pin!(fund(payment::Demand::Singleton("x".repeat(2048)), 0)).poll(&mut context),
        Poll::Ready(Err(payment::AcquireError::Call(CallError::Request(_))))
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
fn lookup_placement_round_trips_a_registry_claim() {
    use crate::method::Contract;
    let key = Lookup::Instance([0xa5; 32]);
    assert_eq!(
        LookupPlacement::decode_request(&LookupPlacement::encode_request(&key).unwrap()).unwrap(),
        key
    );
}

#[test]
fn standby_inspection_covers_claimed_assets_and_retained_transfer_history() {
    use super::standby::{Asset, ObserveStandby, Refill};
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
        let reply = ObserveStandby::encode_reply(&rows).unwrap();
        assert_eq!(ObserveStandby::decode_reply(&reply).unwrap(), rows);
    }
}

#[test]
fn previous_payment_frame_cannot_settle_an_allocation_after_the_cut() {
    use crate::method::Contract;
    use payment::{AcquirePlacement, Demand};

    let old = ic_wire::encode(
        &(Demand::Instance([1; 32]), 10_u128),
        ic_wire::Encoding::Frame {
            method: ic_wire::method_id("canic.placement.fund_placement.v1"),
            direction: ic_wire::Direction::Request,
        },
        AcquirePlacement::REQUEST,
    )
    .unwrap();
    assert!(AcquirePlacement::decode_request(&old).is_err());
}
