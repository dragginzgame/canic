//! Observe actual management creation across deliberately interrupted executions.

mod support;

use canic_effect_probe::{CREATION_CYCLES, Fault, LIMITS, Phase, START, STATUS, Status};
use ic_testkit::pocket_ic::nonblocking::PocketIc;

async fn count(pic: &PocketIc, executor: ic_testkit::pocket_ic::CanisterId) -> u64 {
    let subnet = pic.get_subnet(executor).await.expect("executor subnet");
    pic.get_subnet_metrics(subnet)
        .await
        .expect("subnet metrics")
        .num_canisters
}

async fn status(pic: &PocketIc, executor: ic_testkit::pocket_ic::CanisterId) -> Status {
    let bytes = ic_wire::encode(
        &(),
        ic_wire::Encoding::Frame {
            method: STATUS,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
    .unwrap();
    let reply = pic
        .query_call(
            executor,
            ic_testkit::pocket_ic::CanisterId::anonymous(),
            "status",
            bytes,
        )
        .await
        .expect("status remains readable");
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: STATUS,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn creation_commit_boundaries() {
    let (wasm, binary) = support::inputs();

    for fault in [
        Fault::Dispatch,
        Fault::BeforeResultWrite,
        Fault::AfterResultWrite,
        Fault::LaterWork,
        Fault::None,
    ] {
        let (mut server, owner) = support::instance(&binary).await;
        let pic = owner.client();
        let executor = pic.create_canister().await;
        pic.install_canister(executor, wasm.clone(), vec![], None)
            .await;
        let before_count = count(pic, executor).await;
        let before_balance = pic.cycle_balance(executor).await;
        let command = ic_wire::encode(
            &fault,
            ic_wire::Encoding::Frame {
                method: START,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .unwrap();
        let result = pic
            .update_call(
                executor,
                ic_testkit::pocket_ic::CanisterId::anonymous(),
                "start",
                command.clone(),
            )
            .await;
        if fault == Fault::None {
            assert!(result.is_ok(), "{result:?}");
        } else {
            let error = result.expect_err("the injected trap must reach the caller");
            let marker = match fault {
                Fault::Dispatch => "injected dispatch trap after enqueue",
                Fault::BeforeResultWrite => "injected trap before result write",
                Fault::AfterResultWrite => "injected trap after result write",
                Fault::LaterWork => "later work failed after result commit",
                Fault::None => unreachable!(),
            };
            assert!(
                error.reject_message.contains(marker),
                "unexpected failure: {error:?}"
            );
        }
        // Advance well past the response: an enqueued call must not appear later.
        for _ in 0..5 {
            pic.tick().await;
        }
        let state = status(pic, executor).await;
        let after_count = count(pic, executor).await;
        let spent = before_balance - pic.cycle_balance(executor).await;

        match fault {
            Fault::Dispatch => {
                assert_eq!(after_count, before_count);
                assert_eq!(state, Status::default());
            }
            Fault::BeforeResultWrite | Fault::AfterResultWrite => {
                assert_eq!(after_count, before_count + 1);
                assert_eq!(state.phase, Phase::Issued);
                assert_eq!(state.attempts, 1);
                assert!(!state.request.is_empty());
                assert_eq!(state.reserved_cycles, CREATION_CYCLES);
                assert_eq!(state.child, None);
                assert_eq!(state.refunded_cycles, None);
                let replay = pic
                    .update_call(
                        executor,
                        ic_testkit::pocket_ic::CanisterId::anonymous(),
                        "start",
                        command,
                    )
                    .await;
                assert!(
                    replay
                        .unwrap_err()
                        .reject_message
                        .contains("blocked: original creation")
                );
                assert_eq!(status(pic, executor).await, state);
                assert_eq!(count(pic, executor).await, after_count);
            }
            Fault::LaterWork | Fault::None => {
                assert_eq!(after_count, before_count + 1);
                assert_eq!(state.phase, Phase::Recorded);
                assert_eq!(state.attempts, 1);
                assert_eq!(state.reserved_cycles, 0);
                assert!(state.refunded_cycles.is_some());
                assert!(!state.later_work_entered);
                let child =
                    ic_testkit::pocket_ic::CanisterId::from_slice(state.child.as_ref().unwrap());
                assert!(pic.canister_exists(child).await);
                assert_eq!(pic.get_controllers(child).await, vec![executor]);
                let replay = pic
                    .update_call(
                        executor,
                        ic_testkit::pocket_ic::CanisterId::anonymous(),
                        "start",
                        command,
                    )
                    .await
                    .unwrap();
                let replay: Status = ic_wire::decode(
                    &replay,
                    ic_wire::Encoding::Frame {
                        method: START,
                        direction: ic_wire::Direction::Response,
                    },
                    LIMITS,
                )
                .unwrap();
                assert_eq!(replay, state);
                assert_eq!(count(pic, executor).await, after_count);
            }
        }
        println!(
            "{fault:?}: canister_delta={}, phase={:?}, attempts={}, reserved_cycles={}, executor_cycle_decrease={spent}",
            after_count - before_count,
            state.phase,
            state.attempts,
            state.reserved_cycles
        );
        assert_eq!(
            owner.close(std::time::Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        let stopped = server.stop().unwrap();
        assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
    }
}
