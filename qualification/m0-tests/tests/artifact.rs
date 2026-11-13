//! Retained bytes support one autonomous install and evidence-only reconciliation.

use canic_m0_artifact::{
    ARTIFACT_LIMITS, AdmissionError, Approval, Fault, LIMITS, Phase, RECONCILE, STAGE, STATUS,
    Status, UPGRADE, artifact_hash,
};
use ic_testkit::pocket_ic::{CanisterId, nonblocking::PocketIc};
use std::{path::PathBuf, time::Duration};

mod support;

async fn status(pic: &PocketIc, executor: CanisterId, operator: CanisterId) -> Status {
    let bytes = pic
        .query_call(
            executor,
            operator,
            "status",
            ic_wire::native::encode_request(STATUS, &(), LIMITS).unwrap(),
        )
        .await
        .unwrap();
    ic_wire::native::decode_response(STATUS, &bytes, LIMITS).unwrap()
}

async fn stage(
    pic: &PocketIc,
    executor: CanisterId,
    operator: CanisterId,
    approval: &Approval,
) -> Result<Status, AdmissionError> {
    let bytes = pic
        .update_call(
            executor,
            operator,
            "stage",
            ic_wire::native::encode_request(STAGE, approval, ARTIFACT_LIMITS).unwrap(),
        )
        .await
        .unwrap();
    ic_wire::native::decode_response(STAGE, &bytes, LIMITS).unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn retained_artifact_installs_without_operator_and_survives_maintenance() {
    let (original, binary) = support::inputs();
    let directory = PathBuf::from(
        std::env::var_os("CANIC_M0_ARTIFACT_DIRECTORY").expect("run make m0-artifact-test"),
    );
    let candidate = std::fs::read(directory.join("candidate.wasm")).unwrap();
    let worker_wasm = std::fs::read(directory.join("worker.wasm")).unwrap();
    assert_ne!(original, candidate);
    let operator = CanisterId::self_authenticating(b"artifact custody operator");
    for (fault, occupied) in [
        (Fault::None, false),
        (Fault::AfterUploadWrite, false),
        (Fault::AfterResultWrite, false),
        (Fault::None, true),
    ] {
        let owner = support::instance(&binary).await;
        let pic = owner.instance();
        let executor = pic
            .create_canister_with_settings(Some(operator), None)
            .await;
        pic.install_canister(executor, original.clone(), vec![], Some(operator))
            .await;
        let worker = pic.create_canister().await;
        if occupied {
            pic.install_canister(worker, worker_wasm.clone(), vec![], None)
                .await;
        }
        pic.set_controllers(worker, None, vec![executor])
            .await
            .unwrap();
        let initial_version = pic
            .canister_status(worker, Some(executor))
            .await
            .unwrap()
            .version;
        let approval = Approval {
            target: worker.as_slice().to_vec(),
            sha256: artifact_hash(&worker_wasm),
            wasm: worker_wasm.clone(),
            execute_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 60_000_000_000,
            fault,
        };
        assert!(
            pic.update_call(
                executor,
                CanisterId::anonymous(),
                "stage",
                ic_wire::native::encode_request(STAGE, &approval, ARTIFACT_LIMITS).unwrap()
            )
            .await
            .is_err()
        );
        assert!(
            pic.update_call(
                executor,
                operator,
                "stage",
                vec![0; ARTIFACT_LIMITS.bytes + 1]
            )
            .await
            .is_err()
        );
        let mut wrong = approval.clone();
        wrong.sha256[0] ^= 1;
        assert_eq!(
            stage(pic, executor, operator, &wrong).await,
            Err(AdmissionError::Hash)
        );
        assert_eq!(status(pic, executor, operator).await.phase, Phase::Empty);

        let staged = stage(pic, executor, operator, &approval).await.unwrap();
        assert_eq!(staged.phase, Phase::Scheduled);
        assert_eq!(staged.attempts, 0);
        assert_eq!(staged.sha256, Some(approval.sha256));
        assert_eq!(staged.wasm_bytes, worker_wasm.len());
        wrong = approval.clone();
        wrong.target = executor.as_slice().to_vec();
        assert_eq!(
            stage(pic, executor, operator, &wrong).await,
            Err(AdmissionError::Occupied)
        );

        // A failed upgrade cannot discard bytes or create another approval slot.
        pic.stop_canister(executor, Some(operator)).await.unwrap();
        let failure = pic
            .upgrade_canister(
                executor,
                candidate.clone(),
                ic_wire::native::encode_request(UPGRADE, &true, LIMITS).unwrap(),
                Some(operator),
            )
            .await
            .unwrap_err();
        assert!(
            failure
                .reject_message
                .contains("injected artifact maintenance trap")
        );
        pic.start_canister(executor, Some(operator)).await.unwrap();
        assert_eq!(status(pic, executor, operator).await, staged);

        // Upgrade before dispatch so only retained on-chain bytes are available
        // to the changed executable. No operator update follows until recovery.
        pic.stop_canister(executor, Some(operator)).await.unwrap();
        pic.upgrade_canister(
            executor,
            candidate.clone(),
            ic_wire::native::encode_request(UPGRADE, &false, LIMITS).unwrap(),
            Some(operator),
        )
        .await
        .unwrap();
        pic.start_canister(executor, Some(operator)).await.unwrap();
        let maintained = status(pic, executor, operator).await;
        assert!(maintained.maintenance_candidate);
        assert_eq!(maintained.phase, Phase::Scheduled);
        assert_eq!(maintained.sha256, staged.sha256);
        assert_eq!(maintained.target, staged.target);
        assert_eq!(maintained.wasm_bytes, staged.wasm_bytes);
        let executor_balance = pic.cycle_balance(executor).await;
        let worker_balance = pic.cycle_balance(worker).await;
        pic.advance_time(Duration::from_secs(61)).await;
        for _ in 0..20 {
            pic.tick().await;
        }
        let dispatched = status(pic, executor, operator).await;
        let attempts = u8::from(fault != Fault::AfterUploadWrite);
        assert_eq!(dispatched.attempts, attempts);
        assert_eq!(dispatched.upload_attempts, 1);
        assert!(dispatched.upload_request_sha256.is_some());
        assert_eq!(dispatched.request_sha256.is_some(), attempts == 1);
        if occupied {
            assert!(matches!(dispatched.phase, Phase::Rejected(_)));
            assert!(!dispatched.reply_retained);
        } else if fault == Fault::AfterUploadWrite {
            assert_eq!(dispatched.phase, Phase::UploadIssued);
            assert_eq!(dispatched.chunk_hash, None);
            assert!(!dispatched.reply_retained);
        } else if fault == Fault::AfterResultWrite {
            assert_eq!(dispatched.phase, Phase::Issued);
            assert!(!dispatched.reply_retained);
            assert_eq!(dispatched.observed_module, None);
        } else {
            assert_eq!(dispatched.phase, Phase::Verified);
            assert!(dispatched.reply_retained);
            assert_eq!(
                dispatched.observed_module.as_deref(),
                Some(approval.sha256.as_slice())
            );
        }
        let observed = pic.canister_status(worker, Some(executor)).await.unwrap();
        assert_eq!(
            observed.version,
            initial_version + u64::from(attempts == 1 && !occupied)
        );
        assert_eq!(
            observed.module_hash.as_deref(),
            if attempts == 1 {
                Some(approval.sha256.as_slice())
            } else {
                None
            }
        );
        assert_eq!(observed.settings.controllers, vec![executor]);
        assert_eq!(
            pic.stored_chunks(executor, Some(operator)).await.unwrap(),
            vec![approval.sha256.to_vec()]
        );
        if attempts == 1 {
            let reply = pic
                .query_call(
                    worker,
                    CanisterId::anonymous(),
                    "greet",
                    ic_wire::native::encode_request(canic::GREET, &"team", canic::LIMITS).unwrap(),
                )
                .await
                .unwrap();
            let greeting: canic::Greeting =
                ic_wire::native::decode_response(canic::GREET, &reply, canic::LIMITS).unwrap();
            assert_eq!(greeting.message, "Hello from Canic, team!");
        }
        println!(
            "{fault:?}, occupied={occupied}: artifact_bytes={}, stable_pages={}, upload_bytes={}, executor_cycle_decrease={}, worker_cycle_decrease={}, phase={:?}",
            worker_wasm.len(),
            maintained.stable_pages,
            ic_wire::native::encode_request(STAGE, &approval, ARTIFACT_LIMITS)
                .unwrap()
                .len(),
            executor_balance - pic.cycle_balance(executor).await,
            worker_balance - pic.cycle_balance(worker).await,
            dispatched.phase
        );
        let memory = pic
            .canister_status(executor, Some(operator))
            .await
            .unwrap()
            .memory_metrics;
        println!(
            "executor backing: heap_bytes={}, stable_bytes={}, chunk_store_bytes={}",
            memory.wasm_memory_size, memory.stable_memory_size, memory.wasm_chunk_store_size
        );

        // A later maintenance window must retain the uncertain request identity.
        pic.stop_canister(executor, Some(operator)).await.unwrap();
        pic.upgrade_canister(
            executor,
            original.clone(),
            ic_wire::native::encode_request(UPGRADE, &false, LIMITS).unwrap(),
            Some(operator),
        )
        .await
        .unwrap();
        pic.start_canister(executor, Some(operator)).await.unwrap();
        let restored = status(pic, executor, operator).await;
        assert_eq!(restored.phase, dispatched.phase);
        assert_eq!(restored.request_sha256, dispatched.request_sha256);
        assert_eq!(
            restored.upload_request_sha256,
            dispatched.upload_request_sha256
        );
        assert_eq!(restored.chunk_hash, dispatched.chunk_hash);
        assert_eq!(restored.reply_retained, dispatched.reply_retained);
        assert_eq!(restored.attempts, attempts);
        assert!(!restored.maintenance_candidate);
        let reconciliation = pic
            .update_call(
                executor,
                operator,
                "reconcile",
                ic_wire::native::encode_request(RECONCILE, &(), LIMITS).unwrap(),
            )
            .await;
        if occupied || fault == Fault::AfterUploadWrite {
            assert!(reconciliation.is_err());
        } else {
            let reconciled: Status =
                ic_wire::native::decode_response(RECONCILE, &reconciliation.unwrap(), LIMITS)
                    .unwrap();
            assert_eq!(reconciled.phase, Phase::Verified);
            assert_eq!(reconciled.reply_retained, fault == Fault::None);
            assert_eq!(
                reconciled.observed_module.as_deref(),
                Some(approval.sha256.as_slice())
            );
        }
        let replay = stage(pic, executor, operator, &approval).await.unwrap();
        assert_eq!(replay.attempts, attempts);
        assert_eq!(
            stage(pic, executor, operator, &wrong).await,
            Err(AdmissionError::Occupied)
        );
        for _ in 0..5 {
            pic.tick().await;
        }
        assert_eq!(
            pic.canister_status(worker, Some(executor))
                .await
                .unwrap()
                .version,
            observed.version
        );
        let final_status = status(pic, executor, operator).await;
        assert_eq!(final_status.attempts, attempts);
        assert_eq!(final_status.upload_attempts, 1);
        assert_eq!(
            pic.stored_chunks(executor, Some(operator)).await.unwrap(),
            vec![approval.sha256.to_vec()]
        );
        owner.close().await;
    }
}
