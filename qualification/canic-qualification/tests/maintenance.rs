//! Controller-led maintenance preserves effect custody across two executables.

mod support;

use canic_effect_probe::{
    CREATION_CYCLES, EXECUTABLE, FUND, FUNDING_STATUS, Fault, FundingOutcome, FundingRequest,
    FundingStatus, LIMITS, MAX_FUNDING_CYCLES, Phase, START, STATUS, Status, UPGRADE,
};
use ic_testkit::pocket_ic::{CanisterId, nonblocking::PocketIc};

#[derive(Debug, Eq, PartialEq)]
struct Checkpoint {
    creation: Status,
    funding: FundingStatus,
}

async fn checkpoint(pic: &PocketIc, executor: CanisterId, operator: CanisterId) -> Checkpoint {
    let creation = pic
        .query_call(
            executor,
            operator,
            "status",
            ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method: STATUS,
                    direction: ic_wire::Direction::Request,
                },
                LIMITS,
            )
            .unwrap(),
        )
        .await
        .expect("controller can read retained evidence");
    let funding = pic
        .query_call(
            executor,
            operator,
            "funding_status",
            ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method: FUNDING_STATUS,
                    direction: ic_wire::Direction::Request,
                },
                LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    Checkpoint {
        creation: ic_wire::decode(
            &creation,
            ic_wire::Encoding::Frame {
                method: STATUS,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap(),
        funding: ic_wire::decode(
            &funding,
            ic_wire::Encoding::Frame {
                method: FUNDING_STATUS,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap(),
    }
}

async fn executable(pic: &PocketIc, executor: CanisterId, operator: CanisterId) -> bool {
    let reply = pic
        .query_call(
            executor,
            operator,
            "executable",
            ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method: EXECUTABLE,
                    direction: ic_wire::Direction::Request,
                },
                LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: EXECUTABLE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
}

async fn count(pic: &PocketIc, executor: CanisterId) -> u64 {
    pic.get_subnet_metrics(pic.get_subnet(executor).await.unwrap())
        .await
        .unwrap()
        .num_canisters
}

#[tokio::test(flavor = "current_thread")]
async fn maintenance_preserves_effect_custody() {
    let (original, binary) = support::inputs();
    let candidate = std::fs::read(
        std::env::var_os("CANIC_EFFECT_CANDIDATE_WASM")
            .expect("run make effect-maintenance-test to build both executables"),
    )
    .unwrap();
    assert_ne!(
        original, candidate,
        "maintenance must change the executable"
    );
    let operator = CanisterId::self_authenticating(b"M0 maintenance operator");

    for (fault, deleted) in [
        (Fault::Dispatch, false),
        (Fault::None, false),
        (Fault::AfterResultWrite, false),
        (Fault::LaterWork, false),
        (Fault::None, true),
        (Fault::AfterResultWrite, true),
    ] {
        let (mut server, owner) = support::instance(&binary).await;
        let pic = owner.client();
        let executor = pic
            .create_canister_with_settings(Some(operator), None)
            .await;
        pic.install_canister(executor, original.clone(), vec![], Some(operator))
            .await;
        let recipient = pic.create_canister().await;
        if deleted {
            pic.stop_canister(recipient, None).await.unwrap();
            pic.delete_canister(recipient, None).await.unwrap();
        }
        let creation = ic_wire::encode(
            &fault,
            ic_wire::Encoding::Frame {
                method: START,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .unwrap();
        let funding = ic_wire::encode(
            &FundingRequest {
                target: recipient.as_slice().to_vec(),
                cycles: MAX_FUNDING_CYCLES,
                fault,
            },
            ic_wire::Encoding::Frame {
                method: FUND,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .unwrap();
        let created = pic
            .update_call(executor, operator, "start", creation.clone())
            .await;
        let funded = pic
            .update_call(executor, operator, "fund", funding.clone())
            .await;
        assert_eq!(created.is_ok(), fault == Fault::None);
        assert_eq!(funded.is_ok(), fault == Fault::None);

        // R's observations stay outside E throughout stop, failed install and
        // restart. This qualifies E maintenance, not durable host crash recovery.
        let retained = checkpoint(pic, executor, operator).await;
        let old_module = pic
            .canister_status(executor, Some(operator))
            .await
            .unwrap()
            .module_hash;
        let before_count = count(pic, executor).await;
        let recipient_balance = if deleted {
            None
        } else {
            Some(pic.cycle_balance(recipient).await)
        };
        match fault {
            Fault::Dispatch => {
                assert_eq!(retained.creation, Status::default());
                assert_eq!(retained.funding, FundingStatus::default());
            }
            Fault::AfterResultWrite => {
                assert_eq!(retained.creation.phase, Phase::Issued);
                assert_eq!(retained.creation.reserved_cycles, CREATION_CYCLES);
                assert_eq!(retained.funding.phase, Phase::Issued);
                assert_eq!(retained.funding.reserved_cycles, MAX_FUNDING_CYCLES);
                assert_eq!(retained.creation.child, None);
                assert_eq!(retained.funding.outcome, None);
            }
            Fault::None | Fault::LaterWork => {
                assert_eq!(retained.creation.phase, Phase::Recorded);
                assert_eq!(retained.funding.phase, Phase::Recorded);
                assert_eq!(retained.creation.reserved_cycles, 0);
                assert_eq!(retained.funding.reserved_cycles, 0);
                assert!(retained.creation.child.is_some());
                if deleted {
                    assert!(matches!(
                        retained.funding.outcome,
                        Some(FundingOutcome::Rejected { .. })
                    ));
                    assert_eq!(retained.funding.refunded_cycles, Some(MAX_FUNDING_CYCLES));
                } else {
                    assert_eq!(retained.funding.outcome, Some(FundingOutcome::Deposited));
                }
            }
            Fault::BeforeResultWrite => unreachable!(),
        }

        let upgrade = ic_wire::encode(
            &false,
            ic_wire::Encoding::Frame {
                method: UPGRADE,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .unwrap();
        assert!(
            pic.upgrade_canister(executor, candidate.clone(), upgrade.clone(), None)
                .await
                .is_err()
        );
        assert!(!executable(pic, executor, operator).await);
        assert_eq!(checkpoint(pic, executor, operator).await, retained);

        // Qualify trapping post-upgrade with both empty and populated prior
        // stable memory. The failed candidate also writes stable memory first.
        for candidate_active in [false, true] {
            let before_stable = pic.get_stable_memory(executor).await;
            let before_module = pic
                .canister_status(executor, Some(operator))
                .await
                .unwrap()
                .module_hash;
            pic.stop_canister(executor, Some(operator)).await.unwrap();
            let failed = pic
                .upgrade_canister(
                    executor,
                    if candidate_active {
                        original.clone()
                    } else {
                        candidate.clone()
                    },
                    ic_wire::encode(
                        &true,
                        ic_wire::Encoding::Frame {
                            method: UPGRADE,
                            direction: ic_wire::Direction::Request,
                        },
                        LIMITS,
                    )
                    .unwrap(),
                    Some(operator),
                )
                .await
                .expect_err("injected post-upgrade trap");
            assert!(
                failed
                    .reject_message
                    .contains("injected maintenance trap after restore")
            );
            assert_eq!(pic.get_stable_memory(executor).await, before_stable);
            assert_eq!(
                pic.canister_status(executor, Some(operator))
                    .await
                    .unwrap()
                    .module_hash,
                before_module
            );
            pic.start_canister(executor, Some(operator)).await.unwrap();
            assert_eq!(executable(pic, executor, operator).await, candidate_active);
            assert_eq!(checkpoint(pic, executor, operator).await, retained);

            if !candidate_active {
                pic.stop_canister(executor, Some(operator)).await.unwrap();
                pic.upgrade_canister(executor, candidate.clone(), upgrade.clone(), Some(operator))
                    .await
                    .unwrap();
                pic.start_canister(executor, Some(operator)).await.unwrap();
                assert!(executable(pic, executor, operator).await);
                assert_ne!(
                    pic.canister_status(executor, Some(operator))
                        .await
                        .unwrap()
                        .module_hash,
                    old_module
                );
                assert_eq!(checkpoint(pic, executor, operator).await, retained);
            }
        }
        assert_eq!(pic.get_controllers(executor).await, vec![operator]);
        if let Some(child) = &retained.creation.child {
            assert_eq!(
                pic.get_controllers(CanisterId::from_slice(child)).await,
                vec![executor]
            );
        }

        let created = pic.update_call(executor, operator, "start", creation).await;
        let funded = pic.update_call(executor, operator, "fund", funding).await;
        if fault == Fault::AfterResultWrite {
            assert!(
                created
                    .unwrap_err()
                    .reject_message
                    .contains("blocked: original creation")
            );
            assert!(
                funded
                    .unwrap_err()
                    .reject_message
                    .contains("blocked: original funding")
            );
        } else if fault == Fault::Dispatch {
            assert!(created.is_err());
            assert!(funded.is_err());
        } else {
            let creation: Status = ic_wire::decode(
                &created.unwrap(),
                ic_wire::Encoding::Frame {
                    method: START,
                    direction: ic_wire::Direction::Response,
                },
                LIMITS,
            )
            .unwrap();
            let funding: FundingStatus = ic_wire::decode(
                &funded.unwrap(),
                ic_wire::Encoding::Frame {
                    method: FUND,
                    direction: ic_wire::Direction::Response,
                },
                LIMITS,
            )
            .unwrap();
            assert_eq!(creation, retained.creation);
            assert_eq!(funding, retained.funding);
        }
        assert_eq!(checkpoint(pic, executor, operator).await, retained);
        assert_eq!(count(pic, executor).await, before_count);
        if let Some(balance) = recipient_balance {
            assert_eq!(pic.cycle_balance(recipient).await, balance);
        }
        println!(
            "{fault:?}, deleted={deleted}: failed upgrade rolled back, changed executable preserved both journals, no duplicate effects"
        );
        assert_eq!(
            owner.close(std::time::Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        let stopped = server.stop().unwrap();
        assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
    }
}
