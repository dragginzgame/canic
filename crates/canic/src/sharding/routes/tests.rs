#[test]
fn hub_routes_reopen_ten_thousand_subjects_and_isolate_named_pools() {
    use crate::sharding::{Allocation, Placement, ShardTarget, routes};
    use ic_platform_types::Principal;
    use std::{
        future::Future,
        task::{Context, Poll, Waker},
    };
    let memory = ic_storage::DefaultMemoryImpl::default();
    let keys = ["hub.roster.v1", "hub.owners.v1"];
    let open = |fresh| {
        let runtime = crate::storage::bootstrap(memory.clone(), fresh, "hub", &keys, 256);
        routes::open(
            &runtime,
            keys[0],
            keys[1],
            fresh.then_some(Principal::from_slice(&[1, 1])),
        );
    };
    let target = |id: u8| ShardTarget {
        allocation: Allocation {
            slot: u32::from(id),
            generation: [id; 32],
        },
        shard: Principal::from_slice(&[id, 1]),
    };
    let assign = |pool: &str, subject| {
        let mut call = std::pin::pin!(routes::assign_shard(pool, subject, || {}));
        let Poll::Ready(Ok(Placement::Assigned(_, id))) =
            call.as_mut().poll(&mut Context::from_waker(Waker::noop()))
        else {
            panic!("expected a local assignment")
        };
        id
    };
    open(true);
    routes::record_shard("users", 0, target(2), 10_000);
    for n in 0..10_000_u32 {
        assert_eq!(
            assign("users", Principal::from_slice(&n.to_le_bytes())),
            target(2).shard.to_text()
        );
    }
    routes::record_shard("users", 1, target(3), 10_000);
    routes::record_shard("other", 0, target(4), 1);
    let alice = Principal::from_slice(&0_u32.to_le_bytes());
    assert_eq!(assign("other", alice), target(4).shard.to_text());
    assert_eq!(
        assign("users", Principal::from_slice(&10_000_u32.to_le_bytes())),
        target(3).shard.to_text()
    );
    super::INDEX.set(None); // A canister upgrade drops all heap leases.
    open(false);
    assert_eq!(routes::get_shard("users", alice), Some(target(2)));
    assert_eq!(routes::get_shard("other", alice), Some(target(4)));
    assert_eq!(routes::get_shard("missing", alice), None);
}

#[test]
fn drain_preserves_both_reservations_through_reopen_and_exact_confirmation() {
    use super::*;
    use std::{
        future::Future,
        task::{Context, Poll, Waker},
    };
    fn local<F: Future>(f: F) -> F::Output {
        match std::pin::pin!(f).poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(v) => v,
            Poll::Pending => panic!("unexpected remote growth"),
        }
    }
    let principal = |id| Principal::from_slice(&[id, 1]);
    let root = principal(1);
    let alice = principal(2);
    let memory = DefaultMemoryImpl::default();
    let open = |fresh| {
        let keys = ["hub.routes.v1", "hub.owners.v1"];
        let runtime = crate::storage::bootstrap(memory.clone(), fresh, "hub", &keys, 8);
        super::open(&runtime, keys[0], keys[1], fresh.then_some(root));
    };
    let target = |slot| ShardTarget {
        shard: principal(slot),
        allocation: super::super::Allocation {
            slot: slot.into(),
            generation: [slot; 32],
        },
    };
    let a = target(3);
    let b = target(4);
    let c = target(5);
    open(true);
    for (n, shard) in [a.clone(), b.clone(), c.clone()].into_iter().enumerate() {
        record_shard("users", n.try_into().unwrap(), shard, 2);
    }
    local(assign_shard("users", alice, || {})).unwrap();
    assert!(status(alice, "users", &a, true).is_err());
    assert_eq!(status(root, "users", &a, true).unwrap().live_placements, 1);
    local(prepare_move("users", alice, &a, || {})).unwrap();
    INDEX.set(None);
    open(false);
    assert_eq!(get_shard("users", alice), Some(a.clone()));
    assert_eq!(status(root, "users", &b, false).unwrap().live_placements, 1);
    assert!(confirm_move("users", alice, &a, &c).is_err());
    confirm_move("users", alice, &a, &b).unwrap();
    INDEX.set(None);
    open(false);
    confirm_move("users", alice, &a, &b).unwrap();
    assert_eq!(status(root, "users", &a, true).unwrap().live_placements, 0);
    assert_eq!(status(root, "users", &b, false).unwrap().live_placements, 1);
    status(root, "users", &c, true).unwrap();
    assert!(confirm_move("users", alice, &c, &b).is_err());
}
