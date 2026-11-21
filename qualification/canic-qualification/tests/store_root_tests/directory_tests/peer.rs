//! One real sender and receiver under the same installing Root.
use super::*;
use ic_testkit::held::{Catalog, Fixture, Limits, Reply};

pub(super) async fn exercise(root: &Client<'_>, accounts: CanisterId, project: CanisterId) {
    let pic = root.pic;
    let sender = Client {
        pic,
        id: project,
        operator: root.id,
    };
    verify_wait_modes(&sender).await;
    let receiver = Client {
        pic,
        id: accounts,
        operator: root.id,
    };
    let sync = async || {
        sender
            .update::<_, Result<u64, String>>(
                "send_sync",
                ic_wire::method_id("send_sync"),
                &None::<CanisterId>,
                canic::method::LIMITS,
            )
            .await
    };
    // Project is ready, but the accounts hub missed its directory publication.
    assert_eq!(sync().await, Err("sync unconfirmed".into()));
    assert_eq!(publish(root, "accounts_hub", 2).await, Ok(2));
    assert_eq!(sync().await, Ok(1));

    let request = |name, input: &()| {
        ic_wire::encode(
            input,
            ic_wire::Encoding::Frame {
                method: ic_wire::method_id(name),
                direction: ic_wire::Direction::Request,
            },
            canic::method::LIMITS,
        )
        .unwrap()
    };
    assert!(query_service::<bool>(pic, accounts, "certificate_available", ()).await);
    assert!(
        !sender
            .update::<_, bool>(
                "probe_certificate",
                ic_wire::method_id("probe_certificate"),
                &(),
                canic::method::LIMITS
            )
            .await
    );
    for caller in [root.id, accounts, root.operator] {
        // Even malformed input is refused by the guard before Wire decoding.
        let reject = pic
            .update_call(accounts, caller, "project_sync", vec![])
            .await
            .unwrap_err();
        assert!(
            reject.reject_message.contains("unexpected topology sender"),
            "{reject:?}"
        );
    }
    for caller in [project, root.id] {
        assert!(
            pic.query_call(accounts, caller, "sync_count", request("sync_count", &()))
                .await
                .is_ok()
        );
    }
    assert!(
        pic.query_call(
            accounts,
            root.operator,
            "sync_count",
            request("sync_count", &())
        )
        .await
        .is_err()
    );

    canic_qualification::upgrade_stopped(
        pic,
        accounts,
        std::fs::read(artifacts().join("directory.wasm")).unwrap(),
        vec![],
        Some(root.id),
    )
    .await
    .unwrap();
    assert_eq!(sync().await, Ok(1));
    let pause = async |value: bool| {
        receiver
            .update::<_, ()>(
                "pause_writes",
                ic_wire::method_id("pause_writes"),
                &value,
                canic::method::LIMITS,
            )
            .await;
    };
    let gate = pic.create_canister().await;
    let catalog = Catalog::new(
        &[Reply {
            method: "hold",
            bytes: &[],
        }],
        Limits {
            slots: 1,
            name_bytes: 8,
            reply_bytes: 0,
        },
    )
    .unwrap();
    let mut held = Fixture::new(
        pic,
        gate.to_text().parse().unwrap(),
        &catalog,
        100_000_000_000,
        8192,
    )
    .await
    .unwrap();
    pic.add_cycles(gate, 1_000_000_000_000).await;
    pic.install_canister(gate, held.wasm().to_vec(), vec![], None)
        .await;
    let frame = |direction| ic_wire::Encoding::Frame {
        method: ic_wire::method_id("send_sync"),
        direction,
    };
    let msg = pic
        .submit_call(
            project,
            root.id,
            "send_sync",
            ic_wire::encode(
                &Some(gate),
                frame(ic_wire::Direction::Request),
                canic::method::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let mut token = None;
    for _ in 0..40 {
        pic.tick().await;
        token = held.pending(pic).await.unwrap().pop();
        if token.is_some() {
            break;
        }
    }
    let token = token.expect("admitted call reached barrier");
    pause(true).await;
    assert_eq!(sync().await, Err("writes paused".into()));
    pause(false).await;
    held.release(pic, &token).await.unwrap();
    let reply = pic.await_call(msg).await.unwrap();
    assert_eq!(
        ic_wire::decode::<Result<u64, String>>(
            &reply,
            frame(ic_wire::Direction::Response),
            canic::method::LIMITS
        )
        .unwrap(),
        Err("write permission changed".into())
    );
    assert_eq!(sync().await, Ok(2));
}

// Observe the replica-provided deadline on the receiving side of an actual call.
// Platform Runtime 0.23.23 has no msg_deadline accessor. This test fixture imports
// the IC primitive and returns one of two ordinary Wire responses.
async fn verify_wait_modes(sender: &Client<'_>) {
    let pic = sender.pic;
    let target = pic.create_canister().await;
    pic.add_cycles(target, 1_000_000_000_000).await;
    let frame = ic_wire::Encoding::Frame {
        method: 0x4341_4e49_4300_0067,
        direction: ic_wire::Direction::Response,
    };
    let reply = |value| ic_wire::encode(&value, frame, canic::method::LIMITS).unwrap();
    let bounded = reply(true);
    let unbounded = reply(false);
    let bytes = |frame: &[u8]| {
        frame
            .iter()
            .map(|byte| format!(r"\{byte:02x}"))
            .collect::<String>()
    };
    let wasm = wat::parse_str(format!(
        r#"(module
        (import "ic0" "msg_deadline" (func $deadline (result i64)))
        (import "ic0" "msg_reply_data_append" (func $append (param i32 i32)))
        (import "ic0" "msg_reply" (func $reply))
        (memory (export "memory") 1)
        (data (i32.const 0) "{bounded_frame}{unbounded_frame}")
        (func (export "canister_update has_deadline")
            (if (i64.eqz (call $deadline))
                (then (call $append (i32.const {offset}) (i32.const {unbounded_len})))
                (else (call $append (i32.const 0) (i32.const {offset}))))
            (call $reply)))"#,
        bounded_frame = bytes(&bounded),
        unbounded_frame = bytes(&unbounded),
        offset = bounded.len(),
        unbounded_len = unbounded.len(),
    ))
    .unwrap();
    pic.install_canister(target, wasm, vec![], None).await;
    for seconds in [Some(5u32), None] {
        let has_deadline: bool = sender
            .update(
                "probe_wait",
                ic_wire::method_id("probe_wait"),
                &(target, seconds),
                canic::method::LIMITS,
            )
            .await;
        assert_eq!(has_deadline, seconds.is_some());
    }
}
