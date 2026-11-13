//! Controller-led maintenance preserves effect custody across two executables.

use canic_m0_probe::{
    CREATION_CYCLES, EXECUTABLE, FUND, FUNDING_STATUS, Fault, FundingOutcome, FundingRequest,
    FundingStatus, LIMITS, MAX_FUNDING_CYCLES, Phase, START, STATUS, Status, UPGRADE,
};
use ic_testkit::pocket_ic::{CanisterId, nonblocking::PocketIc};

mod support;

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
            ic_wire::native::encode_request(STATUS, &(), LIMITS).unwrap(),
        )
        .await
        .expect("controller can read retained evidence");
    let funding = pic
        .query_call(
            executor,
            operator,
            "funding_status",
            ic_wire::native::encode_request(FUNDING_STATUS, &(), LIMITS).unwrap(),
        )
        .await
        .unwrap();
    Checkpoint {
        creation: ic_wire::native::decode_response(STATUS, &creation, LIMITS).unwrap(),
        funding: ic_wire::native::decode_response(FUNDING_STATUS, &funding, LIMITS).unwrap(),
    }
}

async fn executable(pic: &PocketIc, executor: CanisterId, operator: CanisterId) -> bool {
    let reply = pic
        .query_call(
            executor,
            operator,
            "executable",
            ic_wire::native::encode_request(EXECUTABLE, &(), LIMITS).unwrap(),
        )
        .await
        .unwrap();
    ic_wire::native::decode_response(EXECUTABLE, &reply, LIMITS).unwrap()
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
        std::env::var_os("CANIC_M0_CANDIDATE_WASM")
            .expect("run make m0-maintenance-test to build both executables"),
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
        let owner = support::instance(&binary).await;
        let pic = owner.instance();
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
        let creation = ic_wire::native::encode_request(START, &fault, LIMITS).unwrap();
        let funding = ic_wire::native::encode_request(
            FUND,
            &FundingRequest {
                target: recipient.as_slice().to_vec(),
                cycles: MAX_FUNDING_CYCLES,
                fault,
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

        let upgrade = ic_wire::native::encode_request(UPGRADE, &false, LIMITS).unwrap();
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
                    ic_wire::native::encode_request(UPGRADE, &true, LIMITS).unwrap(),
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
            let creation: Status =
                ic_wire::native::decode_response(START, &created.unwrap(), LIMITS).unwrap();
            let funding: FundingStatus =
                ic_wire::native::decode_response(FUND, &funded.unwrap(), LIMITS).unwrap();
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
        owner.close().await;
    }
}
