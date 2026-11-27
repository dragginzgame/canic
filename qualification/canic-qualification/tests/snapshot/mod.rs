//! Real Root-originated snapshot calls. This does not substitute for Backup's consumer route.
use super::*;
use canic_management_protocol::backup::{
    self as protocol, Effect, Error, Kind, Operation, Outcome, Record, Request, Step,
    management as m,
};
use ic_platform_protocol::candid::Limits as CandidLimits;

const DECODE: CandidLimits = CandidLimits {
    bytes: protocol::MAX_INNER,
    header: 65_536,
    types: 1024,
    work: 8_000_000,
    allocation: 8_000_000,
    vector: protocol::MAX_INNER,
    skipping: 8_000_000,
};

fn request(op: &Operation, step: Step, effect: Effect, expiry: u64) -> Request {
    let bytes = match &effect {
        Effect::Recovery { child, .. } => Ok(step.recovery_payload(op.kind, child).unwrap().1),
        Effect::Canister(a) => a.encode(protocol::MAX_INNER),
        Effect::Take(a) => a.encode(protocol::MAX_INNER),
        Effect::Metadata(a) => a.encode(protocol::MAX_INNER),
        Effect::Read(a) => a.encode(protocol::MAX_INNER),
        Effect::Allocate(a) => a.encode(protocol::MAX_INNER),
        Effect::Upload(a) => a.encode(protocol::MAX_INNER),
        Effect::Load(a) => a.encode(protocol::MAX_INNER),
    }
    .unwrap();
    Request {
        operation: op.clone(),
        step,
        effect,
        arguments_hash: digest(&bytes),
        arguments_bytes: bytes.len() as u32,
        expires_at_ns: expiry,
        cycles: 0,
        cycle_limit: protocol::MAX_CYCLES,
        reply_bytes: if matches!(step, Step::Read(_)) {
            protocol::MAX_INNER as u32
        } else {
            16_384
        },
    }
}

async fn call<T: serde::Serialize>(
    pic: &PocketIc,
    root: CanisterId,
    method: &str,
    id: u64,
    value: &T,
) -> Vec<u8> {
    let bytes = ic_wire::encode(
        value,
        ic_wire::Encoding::Frame {
            method: id,
            direction: ic_wire::Direction::Request,
        },
        protocol::LIMITS,
    )
    .unwrap();
    pic.update_call(root, CanisterId::anonymous(), method, bytes)
        .await
        .unwrap()
}

async fn execute(pic: &PocketIc, root: CanisterId, r: &Request) -> Result<Record, Error> {
    let bytes = call(pic, root, "execute_snapshot_step", protocol::EXECUTE, r).await;
    match r.prepare() {
        Ok((binding, _)) => protocol::Exchange::Execute
            .decode_response(&binding, &bytes)
            .unwrap(),
        Err(_) => {
            // Deliberately malformed requests have no valid expected binding.
            let result: Result<Record, Error> = ic_wire::decode(
                &bytes,
                ic_wire::Encoding::Frame {
                    method: protocol::EXECUTE,
                    direction: ic_wire::Direction::Response,
                },
                protocol::LIMITS,
            )
            .unwrap();
            Err(result.expect_err("invalid request accepted"))
        }
    }
}

async fn lookup(pic: &PocketIc, root: CanisterId, r: &Request) -> Result<Record, Error> {
    let binding = r.prepare().unwrap().0;
    let bytes = call(
        pic,
        root,
        "lookup_snapshot_step",
        protocol::LOOKUP,
        &binding,
    )
    .await;
    protocol::Exchange::Lookup
        .decode_response(&binding, &bytes)
        .unwrap()
}

fn reply(record: Record) -> Vec<u8> {
    match record.outcome {
        Outcome::Reply(bytes) => bytes,
        other => panic!("inner outcome: {other:?}"),
    }
}

async fn fixture(pic: &PocketIc, wasm: Vec<u8>) -> (CanisterId, CanisterId, u64, ChildStore<'_>) {
    let root = pic.create_canister().await;
    let store = ChildStore::install(
        pic,
        root,
        std::fs::read(repo().join("target/backup/store.wasm")).unwrap(),
    )
    .await;
    pic.add_cycles(root, 5_000_000_000_000).await;
    pic.install_canister(root, wasm, vec![], None).await;
    let child = std::fs::read(repo().join("target/backup/app.wasm")).unwrap();
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let input = Stage {
        approval: Approval {
            manager: root.to_text(),
            module_hash: digest(&child),
            creation_cycles: CREATION_CYCLES,
            execute_at_ns: now,
            expires_at_ns: now + 120_000_000_000,
        },
        wasm: child,
    };
    stage(pic, root, &store, &input).await.unwrap();
    settle(pic).await;
    let s = status(pic, root).await;
    assert_eq!(s.phase, Phase::Ready, "{s:?}");
    (
        root,
        CanisterId::from_text(s.worker.unwrap()).unwrap(),
        now + 250_000_000_000,
        store,
    )
}

fn operation(root: CanisterId, child: CanisterId) -> Operation {
    Operation {
        id: [1; 16],
        kind: Kind::Capture,
        approval: [2; 32],
        network: [3; 32],
        manager: root.to_text(),
        caller: CanisterId::anonymous().to_text(),
        generation: 1,
        target: child.to_text(),
    }
}

#[tokio::test]
async fn original_snapshot_results_survive_duplicates_upgrade_and_callback_failure() {
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(240),
            stdout_bytes: 1024 * 1024,
            stderr_bytes: 1024 * 1024,
        },
    )
    .unwrap();
    let owner = server
        .instance(
            PocketIcBuilder::new().with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    let pic = owner.client();
    let wasm = std::fs::read(repo().join("target/backup/root.wasm")).unwrap();
    let (root, child, expiry, store) = fixture(pic, wasm.clone()).await;
    let value = canic_qualification::counter(
        pic,
        child,
        CanisterId::anonymous(),
        example_counter::Request::Increment,
    )
    .await;
    assert_eq!(value.unwrap().value, 1);
    let original_memory = pic.get_stable_memory(child).await;
    let op = operation(root, child);
    let args = m::CanisterArgs {
        canister_id: op.target.parse().unwrap(),
    };
    let stop = request(&op, Step::Stop, Effect::Canister(args.clone()), expiry);
    let mut unfunded = stop.clone();
    unfunded.cycle_limit = 0;
    assert_eq!(execute(pic, root, &unfunded).await, Err(Error::Capacity));
    let mut expired = stop.clone();
    expired.expires_at_ns = 1;
    assert_eq!(execute(pic, root, &expired).await, Err(Error::Expired));
    assert_eq!(lookup(pic, root, &stop).await, Err(Error::Missing));
    // Outsiders cannot even invoke observation; controller and application permission remain separate.
    let outsider = CanisterId::self_authenticating(b"snapshot outsider");
    let encoded = ic_wire::encode(
        &stop,
        ic_wire::Encoding::Frame {
            method: protocol::EXECUTE,
            direction: ic_wire::Direction::Request,
        },
        protocol::LIMITS,
    )
    .unwrap();
    assert!(
        pic.update_call(root, outsider, "execute_snapshot_step", encoded)
            .await
            .is_err()
    );
    reply(execute(pic, root, &stop).await.unwrap());
    // While the first child stays stopped, Root can create and start a sibling.
    let sibling = canic_management_protocol::children::ChildId::SECOND;
    let mut input = children::input(
        root,
        std::fs::read(repo().join("target/backup/app.wasm")).unwrap(),
        pic.get_time().await.as_nanos_since_unix_epoch(),
    );
    input.approval.execute_at_ns = pic.get_time().await.as_nanos_since_unix_epoch();
    children::admit(pic, root, &store, sibling, input)
        .await
        .unwrap();
    settle(pic).await;
    assert_eq!(
        creation_status(pic, root, sibling).await.phase,
        Phase::Ready
    );
    assert!(
        pic.update_call(
            root,
            CanisterId::anonymous(),
            "execute_snapshot_step",
            vec![0]
        )
        .await
        .is_err()
    );
    let initial = request(
        &op,
        Step::InitialStatus,
        Effect::Canister(args.clone()),
        expiry,
    );
    let s = m::decode_canister_status(&reply(execute(pic, root, &initial).await.unwrap()), DECODE)
        .unwrap();
    assert_eq!(s.status, m::Status::Stopped);
    assert_eq!(pic.get_stable_memory(child).await, original_memory);
    assert_eq!(s.settings.controllers[0].to_text(), root.to_text());
    let take = request(
        &op,
        Step::Take,
        Effect::Take(m::TakeSnapshotArgs {
            canister_id: args.canister_id,
            replace_snapshot: None,
            uninstall_code: Some(false),
            sender_canister_version: None,
        }),
        expiry,
    );
    let original = execute(pic, root, &take).await.unwrap();
    let snapshot = m::decode_take_snapshot(&reply(original.clone()), DECODE).unwrap();
    assert_eq!(execute(pic, root, &take).await.unwrap(), original);
    assert_eq!(lookup(pic, root, &take).await.unwrap(), original);
    pic.set_controllers(root, None, vec![outsider])
        .await
        .unwrap();
    let binding = take.prepare().unwrap().0;
    let bytes = ic_wire::encode(
        &binding,
        ic_wire::Encoding::Frame {
            method: protocol::LOOKUP,
            direction: ic_wire::Direction::Request,
        },
        protocol::LIMITS,
    )
    .unwrap();
    assert!(
        pic.update_call(
            root,
            CanisterId::anonymous(),
            "lookup_snapshot_step",
            bytes.clone()
        )
        .await
        .is_err()
    );
    let observed = pic
        .update_call(root, outsider, "lookup_snapshot_step", bytes)
        .await
        .unwrap();
    let observed: Result<Record, Error> = ic_wire::decode(
        &observed,
        ic_wire::Encoding::Frame {
            method: protocol::LOOKUP,
            direction: ic_wire::Direction::Response,
        },
        protocol::LIMITS,
    )
    .unwrap();
    assert_eq!(observed.unwrap(), original);
    pic.set_controllers(root, Some(outsider), vec![CanisterId::anonymous()])
        .await
        .unwrap();
    let mut stale = take.prepare().unwrap().0;
    stale.operation.generation += 1;
    let bytes = call(pic, root, "lookup_snapshot_step", protocol::LOOKUP, &stale).await;
    let refused = protocol::Exchange::Lookup
        .decode_response(&stale, &bytes)
        .unwrap();
    assert_eq!(refused, Err(Error::Conflict));
    let mut changed = take.clone();
    changed.operation.approval = [8; 32];
    assert_eq!(execute(pic, root, &changed).await, Err(Error::Conflict));
    let mut bad = take.clone();
    bad.arguments_hash = [9; 32];
    assert_eq!(execute(pic, root, &bad).await, Err(Error::Invalid));
    canic_qualification::upgrade_stopped(pic, root, wasm.clone(), vec![], None)
        .await
        .unwrap();
    assert_eq!(lookup(pic, root, &take).await.unwrap(), original);
    let meta = request(
        &op,
        Step::Metadata,
        Effect::Metadata(m::SnapshotArgs {
            canister_id: args.canister_id,
            snapshot_id: snapshot.id.clone(),
        }),
        expiry,
    );
    let metadata =
        m::decode_snapshot_metadata(&reply(execute(pic, root, &meta).await.unwrap()), DECODE)
            .unwrap();
    assert!(metadata.wasm_chunk_store.is_empty());
    // Complete acquisition and transfer through the same Root effect boundary.
    let mut extents = Vec::new();
    let mut ordinal = 0;
    for (region, size) in [
        (0, metadata.wasm_module_size),
        (1, metadata.wasm_memory_size),
        (2, metadata.stable_memory_size),
    ] {
        let mut offset = 0;
        while offset < size {
            let length = (size - offset).min(1_048_576);
            let e = m::Extent {
                offset,
                size: length,
            };
            let kind = match region {
                0 => m::ReadKind::WasmModule(e),
                1 => m::ReadKind::WasmMemory(e),
                _ => m::ReadKind::StableMemory(e),
            };
            let r = request(
                &op,
                Step::Read(ordinal),
                Effect::Read(m::ReadDataArgs {
                    kind,
                    canister_id: args.canister_id,
                    snapshot_id: snapshot.id.clone(),
                }),
                expiry,
            );
            let data =
                m::decode_snapshot_data(&reply(execute(pic, root, &r).await.unwrap()), DECODE)
                    .unwrap()
                    .chunk;
            assert_eq!(data.len() as u64, length);
            extents.push((region, offset, data));
            offset += length;
            ordinal += 1;
        }
    }
    reply(
        execute(
            pic,
            root,
            &request(
                &op,
                Step::FinalStatus,
                Effect::Canister(args.clone()),
                expiry,
            ),
        )
        .await
        .unwrap(),
    );
    let restore = Operation {
        kind: Kind::Restore,
        id: [4; 16],
        approval: [5; 32],
        ..op.clone()
    };
    let allocation = request(
        &restore,
        Step::Allocate,
        Effect::Allocate(m::UploadMetadataArgs {
            canister_id: args.canister_id,
            globals: metadata
                .globals
                .into_iter()
                .collect::<Option<Vec<_>>>()
                .unwrap(),
            replace_snapshot: None,
            certified_data: metadata.certified_data,
            global_timer: metadata.global_timer,
            on_low_wasm_memory_hook_status: metadata.on_low_wasm_memory_hook_status,
            wasm_module_size: metadata.wasm_module_size,
            stable_memory_size: metadata.stable_memory_size,
            wasm_memory_size: metadata.wasm_memory_size,
        }),
        expiry,
    );
    let destination = m::decode_snapshot_allocation(
        &reply(execute(pic, root, &allocation).await.unwrap()),
        DECODE,
    )
    .unwrap()
    .snapshot_id;
    for (n, (region, offset, data)) in extents.iter().enumerate() {
        let o = m::Offset { offset: *offset };
        let kind = match region {
            0 => m::WriteKind::WasmModule(o),
            1 => m::WriteKind::WasmMemory(o),
            _ => m::WriteKind::StableMemory(o),
        };
        let upload = request(
            &restore,
            Step::Upload(n as u32),
            Effect::Upload(m::UploadDataArgs {
                kind,
                chunk: data.clone(),
                canister_id: args.canister_id,
                snapshot_id: destination.clone(),
            }),
            expiry,
        );
        reply(execute(pic, root, &upload).await.unwrap());
        let e = m::Extent {
            offset: *offset,
            size: data.len() as u64,
        };
        let kind = match region {
            0 => m::ReadKind::WasmModule(e),
            1 => m::ReadKind::WasmMemory(e),
            _ => m::ReadKind::StableMemory(e),
        };
        let read = request(
            &restore,
            Step::Read(n as u32),
            Effect::Read(m::ReadDataArgs {
                kind,
                canister_id: args.canister_id,
                snapshot_id: destination.clone(),
            }),
            expiry,
        );
        assert_eq!(
            m::decode_snapshot_data(&reply(execute(pic, root, &read).await.unwrap()), DECODE)
                .unwrap()
                .chunk,
            *data
        );
    }
    let load = request(
        &restore,
        Step::Load,
        Effect::Load(m::LoadSnapshotArgs {
            canister_id: args.canister_id,
            snapshot_id: destination,
            sender_canister_version: None,
        }),
        expiry,
    );
    reply(execute(pic, root, &load).await.unwrap());
    let final_status = request(
        &restore,
        Step::FinalStatus,
        Effect::Canister(args.clone()),
        expiry,
    );
    let s = m::decode_canister_status(
        &reply(execute(pic, root, &final_status).await.unwrap()),
        DECODE,
    )
    .unwrap();
    assert_eq!(s.status, m::Status::Stopped);
    assert_eq!(
        pic.get_stable_memory(child).await,
        original_memory,
        "the uploaded snapshot restores the populated counter bytes"
    );
    // The capture and restore allocation already occupy two native snapshots.
    for index in 0..10_u8 {
        let operation = Operation {
            id: [20 + index; 16],
            approval: [20 + index; 32],
            ..op.clone()
        };
        let capture = request(
            &operation,
            Step::Take,
            Effect::Take(m::TakeSnapshotArgs {
                canister_id: args.canister_id,
                replace_snapshot: (index == 9).then(|| snapshot.id.clone()),
                uninstall_code: Some(false),
                sender_canister_version: None,
            }),
            expiry,
        );
        let recorded = execute(pic, root, &capture).await.unwrap();
        if index == 8 {
            assert!(
                matches!(recorded.outcome, Outcome::Rejected(..)),
                "{recorded:?}"
            );
        } else {
            m::decode_take_snapshot(&reply(recorded.clone()), DECODE).unwrap();
        }
        assert_eq!(lookup(pic, root, &capture).await.unwrap(), recorded);
        reply(
            execute(
                pic,
                root,
                &request(
                    &operation,
                    Step::FinalStatus,
                    Effect::Canister(args.clone()),
                    expiry,
                ),
            )
            .await
            .unwrap(),
        );
    }
    pic.advance_time(Duration::from_secs(300)).await;
    assert_eq!(
        execute(pic, root, &take).await.unwrap(),
        original,
        "expiry cannot replace the original result"
    );
    // A callback trap after the management effect leaves the original slot pending across upgrade.
    let fault_wasm = std::fs::read(repo().join("target/backup/root-fault.wasm")).unwrap();
    for (point, step) in [(1u8, Step::Take), (2, Step::Take), (1, Step::Fence)] {
        let (broken, worker, expiry, _) = fixture(pic, fault_wasm.clone()).await;
        let op = operation(broken, worker);
        let args = m::CanisterArgs {
            canister_id: op.target.parse().unwrap(),
        };
        if step == Step::Take {
            reply(
                execute(
                    pic,
                    broken,
                    &request(&op, Step::Stop, Effect::Canister(args.clone()), expiry),
                )
                .await
                .unwrap(),
            );
        }
        let ack = call(pic, broken, "inject_snapshot_fault", 82, &point).await;
        ic_wire::decode::<()>(
            &ack,
            ic_wire::Encoding::Frame {
                method: 82,
                direction: ic_wire::Direction::Response,
            },
            protocol::LIMITS,
        )
        .unwrap();
        let lost = if step == Step::Fence {
            request(
                &op,
                step,
                Effect::Recovery {
                    child: args,
                    source: op.approval,
                },
                expiry,
            )
        } else {
            request(
                &op,
                Step::Take,
                Effect::Take(m::TakeSnapshotArgs {
                    canister_id: op.target.parse().unwrap(),
                    replace_snapshot: None,
                    uninstall_code: None,
                    sender_canister_version: None,
                }),
                expiry,
            )
        };
        let bytes = ic_wire::encode(
            &lost,
            ic_wire::Encoding::Frame {
                method: protocol::EXECUTE,
                direction: ic_wire::Direction::Request,
            },
            protocol::LIMITS,
        )
        .unwrap();
        assert!(
            pic.update_call(
                broken,
                CanisterId::anonymous(),
                "execute_snapshot_step",
                bytes
            )
            .await
            .is_err()
        );
        assert_eq!(
            lookup(pic, broken, &lost).await.unwrap().outcome,
            Outcome::Pending
        );
        canic_qualification::upgrade_stopped(pic, broken, wasm.clone(), vec![], None)
            .await
            .unwrap();
        assert_eq!(
            execute(pic, broken, &lost).await.unwrap().outcome,
            Outcome::Pending
        );
        if step == Step::Fence {
            assert_eq!(
                canic_qualification::counter(
                    pic,
                    worker,
                    CanisterId::anonymous(),
                    example_counter::Request::Increment
                )
                .await,
                Err(example_counter::Error::Fenced)
            );
        }
        let changed = request(
            &op,
            Step::FinalStatus,
            Effect::Canister(m::CanisterArgs {
                canister_id: op.target.parse().unwrap(),
            }),
            expiry,
        );
        assert_eq!(execute(pic, broken, &changed).await, Err(Error::Busy));
    }
    // The application can reject Root's requested generation. A valid transport
    // reply from the executor retains that rejection and grants no stop.
    let (r, worker, expiry, _) = fixture(pic, wasm.clone()).await;
    canic_qualification::counter(
        pic,
        worker,
        r,
        example_counter::Request::Fence { attempt: 4 },
    )
    .await
    .unwrap();
    let op = operation(r, worker);
    let args = m::CanisterArgs {
        canister_id: op.target.parse().unwrap(),
    };
    let fence = request(
        &op,
        Step::Fence,
        Effect::Recovery {
            child: args.clone(),
            source: op.approval,
        },
        expiry,
    );
    assert!(matches!(
        execute(pic, r, &fence).await.unwrap().outcome,
        Outcome::Rejected(_, _)
    ));
    assert_eq!(
        execute(
            pic,
            r,
            &request(&op, Step::Stop, Effect::Canister(args), expiry)
        )
        .await,
        Err(Error::Busy)
    );
    // An original management rejection is terminal evidence, not an outer rejection.
    let (rejected_root, worker, expiry, _) = fixture(pic, wasm).await;
    let op = operation(rejected_root, worker);
    let invalid_snapshot = request(
        &op,
        Step::Metadata,
        Effect::Metadata(m::SnapshotArgs {
            canister_id: op.target.parse().unwrap(),
            snapshot_id: vec![0; 32],
        }),
        expiry,
    );
    let rejection = execute(pic, rejected_root, &invalid_snapshot)
        .await
        .unwrap();
    assert!(matches!(&rejection.outcome, Outcome::Rejected(_, message) if !message.is_empty()));
    assert_eq!(
        execute(pic, rejected_root, &invalid_snapshot)
            .await
            .unwrap(),
        rejection
    );
    assert_eq!(
        lookup(pic, rejected_root, &invalid_snapshot).await.unwrap(),
        rejection
    );
    assert_eq!(
        owner.close(Duration::from_secs(30)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
    assert_eq!(stopped.abandoned, 0);
}
