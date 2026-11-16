//! Direct-deposit evidence across successful and rejecting callbacks.

use canic_effect_probe::{
    FUND, FUNDING_STATUS, Fault, FundingOutcome, FundingRequest, FundingStatus, LIMITS,
    MAX_FUNDING_CYCLES, Phase,
};
use ic_testkit::pocket_ic::{CanisterId, nonblocking::PocketIc};

mod support;

async fn status(pic: &PocketIc, executor: CanisterId) -> FundingStatus {
    let bytes = ic_wire::encode(
        &(),
        ic_wire::Encoding::Frame {
            method: FUNDING_STATUS,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
    .unwrap();
    let reply = pic
        .query_call(executor, CanisterId::anonymous(), "funding_status", bytes)
        .await
        .expect("funding evidence remains readable");
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: FUNDING_STATUS,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn funding_commit_boundaries() {
    let (wasm, binary) = support::inputs();
    for deleted_target in [false, true] {
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
            let target = pic.create_canister().await;
            // Keep a real routed principal for rejection; the living recipient
            // has no Wasm and performs no application work during this experiment.
            let target_before = pic.cycle_balance(target).await;
            if deleted_target {
                pic.stop_canister(target, None).await.unwrap();
                pic.delete_canister(target, None).await.unwrap();
            }
            let intent = FundingRequest {
                target: target.as_slice().to_vec(),
                cycles: MAX_FUNDING_CYCLES,
                fault,
            };
            let command = ic_wire::encode(
                &intent,
                ic_wire::Encoding::Frame {
                    method: FUND,
                    direction: ic_wire::Direction::Request,
                },
                LIMITS,
            )
            .unwrap();
            let executor_before = pic.cycle_balance(executor).await;
            let result = pic
                .update_call(executor, CanisterId::anonymous(), "fund", command.clone())
                .await;
            if fault == Fault::None {
                let reply = result.expect("completed deposit or retained rejection");
                let response: FundingStatus = ic_wire::decode(
                    &reply,
                    ic_wire::Encoding::Frame {
                        method: FUND,
                        direction: ic_wire::Direction::Response,
                    },
                    LIMITS,
                )
                .unwrap();
                assert_eq!(response, status(pic, executor).await);
            } else {
                let marker = match fault {
                    Fault::Dispatch => "injected dispatch trap after enqueue",
                    Fault::BeforeResultWrite => "injected trap before result write",
                    Fault::AfterResultWrite => "injected trap after result write",
                    Fault::LaterWork => "later work failed after result commit",
                    Fault::None => unreachable!(),
                };
                let error = result.expect_err("injected trap must reach caller");
                assert!(
                    error.reject_message.contains(marker),
                    "unexpected failure: {error:?}"
                );
            }
            for _ in 0..5 {
                pic.tick().await;
            }
            let state = status(pic, executor).await;
            let spent = executor_before - pic.cycle_balance(executor).await;
            let expected_credit = if deleted_target || fault == Fault::Dispatch {
                0
            } else {
                intent.cycles
            };
            if !deleted_target {
                assert_eq!(
                    pic.cycle_balance(target).await - target_before,
                    expected_credit
                );
            } else {
                assert!(!pic.canister_exists(target).await);
            }
            // Physical balance observations are an independent oracle, never
            // supplied to the executor as proof of this operation's outcome.
            if expected_credit == 0 {
                assert!(
                    spent < intent.cycles,
                    "attachment must not be lost: {spent}"
                );
            } else {
                assert!(spent >= intent.cycles, "executor pays the attachment");
            }
            if fault == Fault::Dispatch {
                assert_eq!(state, FundingStatus::default());
                let retry = pic
                    .update_call(executor, CanisterId::anonymous(), "fund", command)
                    .await
                    .unwrap_err();
                assert!(
                    retry
                        .reject_message
                        .contains("injected dispatch trap after enqueue")
                );
                for _ in 0..5 {
                    pic.tick().await;
                }
            } else {
                assert_eq!(state.intent.as_ref(), Some(&intent));
                assert_eq!(state.attempts, 1);
                assert!(!state.request.is_empty());
                assert!(!state.later_work_entered);
                let replay = pic
                    .update_call(executor, CanisterId::anonymous(), "fund", command)
                    .await;
                if matches!(fault, Fault::BeforeResultWrite | Fault::AfterResultWrite) {
                    assert_eq!(state.phase, Phase::Issued);
                    assert_eq!(state.reserved_cycles, intent.cycles);
                    assert_eq!(state.outcome, None);
                    assert_eq!(state.refunded_cycles, None);
                    assert!(
                        replay
                            .unwrap_err()
                            .reject_message
                            .contains("blocked: original funding")
                    );
                } else {
                    assert_eq!(state.phase, Phase::Recorded);
                    assert_eq!(state.reserved_cycles, 0);
                    if deleted_target {
                        assert!(
                            matches!(state.outcome, Some(FundingOutcome::Rejected { code }) if code != 0)
                        );
                        assert_eq!(state.refunded_cycles, Some(intent.cycles));
                    } else {
                        assert_eq!(state.outcome, Some(FundingOutcome::Deposited));
                        assert_eq!(state.refunded_cycles, Some(0));
                    }
                    let replay: FundingStatus = ic_wire::decode(
                        &replay.unwrap(),
                        ic_wire::Encoding::Frame {
                            method: FUND,
                            direction: ic_wire::Direction::Response,
                        },
                        LIMITS,
                    )
                    .unwrap();
                    assert_eq!(replay, state);
                }
                // Neither a new amount nor a new destination can overwrite the
                // occupied slot, including after a fully refunded rejection.
                for changed in [
                    FundingRequest {
                        cycles: intent.cycles - 1,
                        ..intent.clone()
                    },
                    FundingRequest {
                        target: executor.as_slice().to_vec(),
                        ..intent.clone()
                    },
                ] {
                    let bytes = ic_wire::encode(
                        &changed,
                        ic_wire::Encoding::Frame {
                            method: FUND,
                            direction: ic_wire::Direction::Request,
                        },
                        LIMITS,
                    )
                    .unwrap();
                    let error = pic
                        .update_call(executor, CanisterId::anonymous(), "fund", bytes)
                        .await
                        .unwrap_err();
                    assert!(
                        error
                            .reject_message
                            .contains("different funding intent refused"),
                        "changed intent: {error:?}"
                    );
                }
            }
            assert_eq!(status(pic, executor).await, state);
            if !deleted_target {
                assert_eq!(
                    pic.cycle_balance(target).await - target_before,
                    expected_credit
                );
            }
            println!(
                "deleted_target={deleted_target}, {fault:?}: credit={expected_credit}, phase={:?}, reserved_cycles={}, outcome={:?}, refund={:?}, executor_cycle_decrease={spent}",
                state.phase, state.reserved_cycles, state.outcome, state.refunded_cycles
            );
            assert_eq!(
                owner.close(std::time::Duration::from_secs(10)).await,
                ic_testkit::server::Deletion::Attempted
            );
            let stopped = server.stop().unwrap();
            assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn funding_admission_bounds() {
    let (wasm, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let executor = pic.create_canister().await;
    pic.install_canister(executor, wasm, vec![], None).await;
    let target = pic.create_canister().await;
    let target_before = pic.cycle_balance(target).await;
    let intent = FundingRequest {
        target: target.as_slice().to_vec(),
        cycles: MAX_FUNDING_CYCLES,
        fault: Fault::None,
    };
    for invalid in [
        FundingRequest {
            cycles: 0,
            ..intent.clone()
        },
        FundingRequest {
            cycles: MAX_FUNDING_CYCLES + 1,
            ..intent.clone()
        },
        FundingRequest {
            target: vec![0; 30],
            ..intent.clone()
        },
    ] {
        let bytes = ic_wire::encode(
            &invalid,
            ic_wire::Encoding::Frame {
                method: FUND,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .unwrap();
        assert!(
            pic.update_call(executor, CanisterId::anonymous(), "fund", bytes)
                .await
                .is_err()
        );
        assert_eq!(status(pic, executor).await, FundingStatus::default());
    }
    let bytes = ic_wire::encode(
        &intent,
        ic_wire::Encoding::Frame {
            method: FUND,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
    .unwrap();
    let error = pic
        .update_call(executor, target, "fund", bytes.clone())
        .await
        .unwrap_err();
    assert!(error.reject_message.contains("controller required"));
    assert_eq!(status(pic, executor).await, FundingStatus::default());
    assert_eq!(pic.cycle_balance(target).await, target_before);
    // Invalid attempts do not consume the slot: one valid call still succeeds.
    pic.update_call(executor, CanisterId::anonymous(), "fund", bytes)
        .await
        .unwrap();
    assert_eq!(
        pic.cycle_balance(target).await - target_before,
        intent.cycles
    );
    assert_eq!(
        status(pic, executor).await.outcome,
        Some(FundingOutcome::Deposited)
    );
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
