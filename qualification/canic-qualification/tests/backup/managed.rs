//! The operator controls Root. Only Root controls the admitted counter.
use super::*;
use canic_management_protocol::{
    Approval, CREATION_CYCLES, LIMITS, Phase, STAGE, STAGE_LIMITS, STATUS, Stage, Status, digest,
};
use ic_testkit::pocket_ic::nonblocking::PocketIc;

pub(super) async fn install(
    pic: &PocketIc,
    caller: CanisterId,
) -> (CanisterId, CanisterId, Vec<u8>, Vec<u8>) {
    let assets = PathBuf::from(
        std::env::var_os("CANIC_BACKUP_FIXTURE").expect("use make backup-signer-test"),
    );
    let root_wasm = fs::read(assets.join("root.wasm")).unwrap();
    let child_wasm = fs::read(assets.join("counter.wasm")).unwrap();
    let root = pic.create_canister().await;
    pic.add_cycles(root, 10_000_000_000_000).await;
    pic.install_canister(root, root_wasm.clone(), vec![], None)
        .await;
    pic.set_controllers(root, None, vec![caller]).await.unwrap();
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let stage = Stage {
        approval: Approval {
            manager: root.to_text(),
            module_hash: digest(&child_wasm),
            creation_cycles: CREATION_CYCLES,
            execute_at_ns: now,
            expires_at_ns: now + 120_000_000_000,
        },
        wasm: child_wasm.clone(),
    };
    let request = |method| ic_wire::Encoding::Frame {
        method,
        direction: ic_wire::Direction::Request,
    };
    pic.update_call(
        root,
        caller,
        "stage",
        ic_wire::encode(&stage, request(STAGE), STAGE_LIMITS).unwrap(),
    )
    .await
    .unwrap();
    for _ in 0..30 {
        pic.advance_time(Duration::from_millis(100)).await;
        pic.tick().await;
    }
    let reply = pic
        .query_call(
            root,
            caller,
            "status",
            ic_wire::encode(&(), request(STATUS), LIMITS).unwrap(),
        )
        .await
        .unwrap();
    let status: Status = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: STATUS,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap();
    assert_eq!(status.phase, Phase::Ready);
    let child = CanisterId::from_text(status.worker.unwrap()).unwrap();
    assert_eq!(
        pic.canister_status(child, Some(root))
            .await
            .unwrap()
            .settings
            .controllers,
        [root]
    );
    (root, child, root_wasm, child_wasm)
}

/// Retain the original request before submission. After restart only ask Root for
/// its original outcome; absence of that outcome cannot authorize replacement.
pub(super) async fn act(
    pic: &PocketIc,
    caller: CanisterId,
    plan: OperationPlan<'_>,
    step: Step,
    guard: &std::cell::RefCell<CounterRecoveryGuard>,
) {
    use canic_management_protocol::backup as snapshot;
    use std::io::Write;
    let directory = PathBuf::from(std::env::var_os("CANIC_BACKUP_FIXTURE").unwrap());
    let path = directory.join(format!("{}-{step:?}.wire", plan.operation_id()));
    let frame = |direction| ic_wire::Encoding::Frame {
        method: snapshot::EXECUTE,
        direction,
    };
    let first = !path.exists();
    if first {
        let expiry = pic.get_time().await.as_nanos_since_unix_epoch() + 120_000_000_000;
        let request = canic_host::backup::recovery_request(plan, step, expiry).unwrap();
        let mut file = fs::File::create_new(&path).unwrap();
        file.write_all(
            &ic_wire::encode(
                &request,
                frame(ic_wire::Direction::Request),
                snapshot::LIMITS,
            )
            .unwrap(),
        )
        .unwrap();
        file.sync_all().unwrap();
        fs::File::open(&directory).unwrap().sync_all().unwrap();
    }
    let bytes = fs::read(path).unwrap();
    let original: snapshot::Request =
        ic_wire::decode(&bytes, frame(ic_wire::Direction::Request), snapshot::LIMITS).unwrap();
    let binding = original.prepare().unwrap().0;
    let root = CanisterId::from_text(&binding.operation.manager).unwrap();
    // Simulate a disconnected operator for the post-load transitions. Submit once
    // and discard the ingress handle. Only Root's retained lookup feeds the guard.
    if first
        && matches!(plan, OperationPlan::Restore(_))
        && matches!(step, Step::Start | Step::Accept | Step::Open)
    {
        pic.submit_call(root, caller, "snapshot_execute", bytes)
            .await
            .unwrap();
        for _ in 0..10 {
            pic.tick().await;
        }
    } else if first {
        let reply = pic
            .update_call(root, caller, "snapshot_execute", bytes)
            .await
            .unwrap();
        assert!(
            snapshot::Exchange::Execute
                .decode_response(&binding, &reply)
                .unwrap()
                .is_ok(),
            "Root refused {step:?}"
        );
    }
    let lookup = ic_wire::encode(
        &binding,
        ic_wire::Encoding::Frame {
            method: snapshot::LOOKUP,
            direction: ic_wire::Direction::Request,
        },
        snapshot::LIMITS,
    )
    .unwrap();
    if first && step == Step::Accept {
        check_controller_change(pic, caller, root, plan, original.expires_at_ns, &lookup).await;
    }
    let bytes = pic
        .update_call(root, caller, "snapshot_lookup", lookup)
        .await
        .unwrap();
    let record = snapshot::Exchange::Lookup
        .decode_response(&binding, &bytes)
        .unwrap()
        .unwrap();
    guard
        .borrow_mut()
        .observe(plan, &original, &record)
        .unwrap();
}

/// Replacing Root's controller revokes the old caller. Its successor cannot
/// issue the remaining Open under the original operator's approval.
async fn check_controller_change(
    pic: &PocketIc,
    caller: CanisterId,
    root: CanisterId,
    plan: OperationPlan<'_>,
    expiry: u64,
    lookup: &[u8],
) {
    use canic_management_protocol::backup as snapshot;
    let successor = CanisterId::self_authenticating(b"backup recovery successor");
    pic.set_controllers(root, Some(caller), vec![successor])
        .await
        .unwrap();
    assert!(
        pic.update_call(root, caller, "snapshot_lookup", lookup.to_vec())
            .await
            .is_err()
    );
    let open = canic_host::backup::recovery_request(plan, Step::Open, expiry).unwrap();
    let open_bytes = ic_wire::encode(
        &open,
        ic_wire::Encoding::Frame {
            method: snapshot::EXECUTE,
            direction: ic_wire::Direction::Request,
        },
        snapshot::LIMITS,
    )
    .unwrap();
    assert!(
        pic.update_call(root, caller, "snapshot_execute", open_bytes.clone())
            .await
            .is_err()
    );
    let reply = pic
        .update_call(root, successor, "snapshot_execute", open_bytes)
        .await
        .unwrap();
    assert_eq!(
        snapshot::Exchange::Execute
            .decode_response(&open.prepare().unwrap().0, &reply)
            .unwrap(),
        Err(snapshot::Error::Invalid)
    );
    pic.set_controllers(root, Some(successor), vec![caller])
        .await
        .unwrap();
}

/// Resubmit the original capture opening to Root after rollback. A retained
/// successful receipt must have no new effect on the running child.
pub(super) async fn replay_capture_open(
    pic: &PocketIc,
    caller: CanisterId,
    root: CanisterId,
    plan: &ic_backup::store::CapturePlan,
) {
    use canic_management_protocol::backup as s;
    let directory = PathBuf::from(std::env::var_os("CANIC_BACKUP_FIXTURE").unwrap());
    let bytes =
        fs::read(directory.join(format!("{}-Open.wire", plan.request().operation_id))).unwrap();
    let original: s::Request = ic_wire::decode(
        &bytes,
        ic_wire::Encoding::Frame {
            method: s::EXECUTE,
            direction: ic_wire::Direction::Request,
        },
        s::LIMITS,
    )
    .unwrap();
    let binding = original.prepare().unwrap().0;
    let result = pic
        .update_call(root, caller, "snapshot_execute", bytes)
        .await
        .unwrap();
    assert!(matches!(
        s::Exchange::Execute
            .decode_response(&binding, &result)
            .unwrap()
            .unwrap()
            .outcome,
        s::Outcome::Reply(_)
    ));
}

/// A new opening cannot be issued before acceptance, even by the correct operator.
pub(super) async fn premature_open(
    pic: &PocketIc,
    caller: CanisterId,
    root: CanisterId,
    restore: OperationPlan<'_>,
) {
    use canic_management_protocol::backup as s;
    let req = canic_host::backup::recovery_request(
        restore,
        Step::Open,
        pic.get_time().await.as_nanos_since_unix_epoch() + 60_000_000_000,
    )
    .unwrap();
    let frame = ic_wire::encode(
        &req,
        ic_wire::Encoding::Frame {
            method: s::EXECUTE,
            direction: ic_wire::Direction::Request,
        },
        s::LIMITS,
    )
    .unwrap();
    assert!(
        pic.update_call(
            root,
            CanisterId::anonymous(),
            "snapshot_execute",
            frame.clone()
        )
        .await
        .is_err()
    );
    let result = pic
        .update_call(root, caller, "snapshot_execute", frame)
        .await
        .unwrap();
    assert_eq!(
        s::Exchange::Execute
            .decode_response(&req.prepare().unwrap().0, &result)
            .unwrap(),
        Err(s::Error::Busy)
    );
}
