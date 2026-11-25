//! Store bootstrap readiness belongs to the actual controller path, not greeting slots.
use super::{Client, artifacts, custody, inject, maintain, store, support};
use canic_management_protocol::{self as management, publication, stored::STORE_LIMITS};
use ic_testkit::pocket_ic::{CanisterId, CanisterSettings, nonblocking::PocketIc};

pub(super) async fn fixture(pic: &PocketIc) -> (Client<'_>, Client<'_>) {
    fixture_with_root(
        pic,
        std::fs::read(artifacts().join("root-faults.wasm")).unwrap(),
    )
    .await
}

pub(super) async fn fixture_with_root(
    pic: &PocketIc,
    root_wasm: Vec<u8>,
) -> (Client<'_>, Client<'_>) {
    fixture_with_code(
        pic,
        root_wasm,
        std::fs::read(artifacts().join("original.wasm")).unwrap(),
    )
    .await
}

pub(super) use crate::local_store::fixture_with_code;

async fn request(root: &Client<'_>, supplier: &Client<'_>) -> publication::InfoRequest {
    publication::InfoRequest {
        store: supplier.id.to_text(),
        expires_at_ns: root.pic.get_time().await.as_nanos_since_unix_epoch() + 240_000_000_000,
    }
}

async fn observe(root: &Client<'_>, request: &publication::InfoRequest) -> publication::InfoReply {
    match root
        .update::<_, publication::ObservationReply>(
            "observe_store",
            publication::OBSERVATION,
            &publication::ObservationRequest::Info(request.clone()),
            STORE_LIMITS,
        )
        .await?
    {
        publication::Observation::Info(info) => Ok(info),
        _ => panic!("expected Store info"),
    }
}

async fn handover(root: &Client<'_>, supplier: &Client<'_>) {
    root.pic
        .update_canister_settings(
            supplier.id,
            Some(root.operator),
            CanisterSettings {
                controllers: Some(vec![root.id]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn store_custody_observation_preserves_all_owners_without_root_copies() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let (root, supplier) = fixture(pic).await;
    let first = supplier.publish_bytes("one", b"\0asm\x01\0\0\0").await;
    let second = store::Manifest {
        release: store::Release {
            version: "two".into(),
            ..first.release.clone()
        },
        ..first.clone()
    };
    supplier
        .update::<_, Result<store::Status, store::Error>>(
            "prepare",
            store::PREPARE,
            &second,
            STORE_LIMITS,
        )
        .await
        .unwrap();
    let other = CanisterId::self_authenticating(b"previous Store controller");
    pic.update_canister_settings(
        supplier.id,
        Some(root.operator),
        CanisterSettings {
            controllers: Some(vec![root.operator, other]),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    for (principal, manifest) in [(root.operator, first.clone()), (other, second.clone())] {
        Client {
            pic,
            id: supplier.id,
            operator: principal,
        }
        .pin(&store::Pin {
            operation: [7; 32],
            manifest,
        })
        .await;
    }
    let expected: Result<store::Custody, store::Error> =
        supplier.release_custody(&first.release).await;
    let expected = expected.unwrap();
    assert_eq!(expected.pins.len(), 2);
    assert_eq!(expected.pins[0].owner, root.operator.as_slice());
    assert_eq!(expected.pins[1].owner, other.as_slice());
    assert_eq!(expected.pins[1].release, second.release);

    let mut pending = first.clone();
    pending.release.version = "pending".into();
    pending.hash = [8; 32];
    pending.chunks = vec![[9; 32]];
    supplier
        .update::<_, Result<store::Status, store::Error>>(
            "prepare",
            store::PREPARE,
            &pending,
            STORE_LIMITS,
        )
        .await
        .unwrap();
    let staged: Result<store::Custody, store::Error> =
        supplier.release_custody(&pending.release).await;
    let staged = staged.unwrap();
    assert!(!staged.status.published);
    assert_eq!(staged.status.missing, vec![0]);
    assert!(staged.pins.is_empty());

    // Qualification controls Store directly here; this is not a Host-driven maintenance flow.
    pic.stop_canister(supplier.id, Some(root.operator))
        .await
        .unwrap();
    pic.upgrade_canister(
        supplier.id,
        std::fs::read(artifacts().join("candidate.wasm")).unwrap(),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
    pic.start_canister(supplier.id, Some(root.operator))
        .await
        .unwrap();
    assert!(supplier.info().await.maintenance_candidate);
    handover(&root, &supplier).await;
    let input = publication::CustodyRequest {
        store: supplier.id.to_text(),
        expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 240_000_000_000,
        release: first.release.clone(),
    };

    async fn read(
        root: &Client<'_>,
        input: publication::CustodyRequest,
    ) -> publication::CustodyReply {
        match root
            .update::<_, publication::ObservationReply>(
                "observe_store",
                publication::OBSERVATION,
                &publication::ObservationRequest::Custody(input),
                STORE_LIMITS,
            )
            .await?
        {
            publication::Observation::Custody(custody) => Ok(custody),
            _ => panic!("expected Store custody"),
        }
    }
    let root_bytes = pic.get_stable_memory(root.id).await;
    let store_bytes = pic.get_stable_memory(supplier.id).await;
    assert_eq!(read(&root, input.clone()).await.unwrap(), expected);
    assert_eq!(
        read(
            &root,
            publication::CustodyRequest {
                release: pending.release,
                ..input.clone()
            }
        )
        .await
        .unwrap(),
        staged
    );
    assert_eq!(
        read(
            &root,
            publication::CustodyRequest {
                release: second.release,
                ..input.clone()
            }
        )
        .await
        .unwrap()
        .pins,
        expected.pins
    );
    let frame = ic_wire::encode(
        &publication::ObservationRequest::Custody(input.clone()),
        ic_wire::Encoding::Frame {
            method: publication::OBSERVATION,
            direction: ic_wire::Direction::Request,
        },
        STORE_LIMITS,
    )
    .unwrap();
    assert!(
        pic.update_call(
            root.id,
            CanisterId::anonymous(),
            "observe_store",
            frame.clone()
        )
        .await
        .is_err()
    );
    assert!(
        pic.update_call(
            supplier.id,
            root.operator,
            "observe",
            ic_wire::encode(
                &store::Observe::Custody(first.release.clone()),
                ic_wire::Encoding::Frame {
                    method: store::OBSERVE,
                    direction: ic_wire::Direction::Request
                },
                STORE_LIMITS
            )
            .unwrap()
        )
        .await
        .is_err()
    );
    assert!(
        pic.update_call(
            root.id,
            root.operator,
            "observe_store",
            vec![0; STORE_LIMITS.bytes + 1]
        )
        .await
        .is_err()
    );
    assert!(
        pic.update_call(root.id, root.operator, "observe_store", vec![0, 1])
            .await
            .is_err()
    );
    assert_eq!(
        read(
            &root,
            publication::CustodyRequest {
                expires_at_ns: 0,
                ..input.clone()
            }
        )
        .await,
        Err(publication::Error::Expired)
    );
    assert_eq!(
        read(
            &root,
            publication::CustodyRequest {
                store: root.id.to_text(),
                ..input.clone()
            }
        )
        .await,
        Err(publication::Error::Invalid)
    );
    assert_eq!(
        read(
            &root,
            publication::CustodyRequest {
                release: store::Release {
                    name: "greeting".into(),
                    version: "absent".into()
                },
                ..input.clone()
            }
        )
        .await,
        Err(publication::Error::Store(store::Error::NotFound))
    );
    inject(&root, 12).await;
    assert!(
        pic.update_call(root.id, root.operator, "observe_store", frame)
            .await
            .is_err()
    );
    inject(&root, 0).await;
    assert_eq!(read(&root, input.clone()).await.unwrap(), expected);
    maintain(&root).await;
    assert_eq!(read(&root, input).await.unwrap(), expected);
    assert_eq!(pic.get_stable_memory(root.id).await, root_bytes);
    assert_eq!(pic.get_stable_memory(supplier.id).await, store_bytes);
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

#[tokio::test(flavor = "current_thread")]
async fn store_readiness_follows_handover_without_reserving_an_application_slot() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let (root, supplier) = fixture(pic).await;
    let store::Observation::Info(expected) = supplier.observe(&store::Observe::Info).await.unwrap()
    else {
        panic!("expected Store info")
    };
    assert_eq!(expected.releases, 0);
    assert_eq!(expected.reserved_bytes, 0);
    let input = request(&root, &supplier).await;
    // An installed Store is not yet available through Root before authority handover.
    assert_eq!(
        observe(&root, &input).await,
        Err(publication::Error::Uncertain)
    );
    let before = custody(&root).await;
    assert!(
        before
            .children
            .iter()
            .all(|slot| slot.creation.phase == management::Phase::Empty
                && slot.creation.approval.is_none()
                && slot.upgrades.is_empty())
    );
    assert!(before.sources.iter().all(Option::is_none));
    let root_bytes = pic.get_stable_memory(root.id).await;
    let store_bytes = pic.get_stable_memory(supplier.id).await;
    handover(&root, &supplier).await;
    assert!(
        pic.update_call(
            supplier.id,
            root.operator,
            "observe",
            ic_wire::encode(
                &store::Observe::Info,
                ic_wire::Encoding::Frame {
                    method: store::OBSERVE,
                    direction: ic_wire::Direction::Request
                },
                STORE_LIMITS
            )
            .unwrap()
        )
        .await
        .is_err()
    );
    assert_eq!(observe(&root, &input).await.unwrap(), expected);
    assert_eq!(observe(&root, &input).await.unwrap(), expected);
    inject(&root, 12).await;
    assert!(
        pic.update_call(
            root.id,
            root.operator,
            "observe_store",
            ic_wire::encode(
                &publication::ObservationRequest::Info(input.clone()),
                ic_wire::Encoding::Frame {
                    method: publication::OBSERVATION,
                    direction: ic_wire::Direction::Request
                },
                STORE_LIMITS
            )
            .unwrap()
        )
        .await
        .is_err()
    );
    inject(&root, 0).await;
    assert_eq!(observe(&root, &input).await.unwrap(), expected);
    maintain(&root).await;
    assert_eq!(
        observe(&root, &request(&root, &supplier).await)
            .await
            .unwrap(),
        expected
    );
    assert_eq!(custody(&root).await, before);
    assert_eq!(pic.get_stable_memory(root.id).await, root_bytes);
    assert_eq!(pic.get_stable_memory(supplier.id).await, store_bytes);
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

#[tokio::test(flavor = "current_thread")]
async fn store_readiness_refusals_never_turn_missing_authority_into_ready() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let (root, supplier) = fixture(pic).await;
    handover(&root, &supplier).await;
    let root_bytes = pic.get_stable_memory(root.id).await;
    let store_bytes = pic.get_stable_memory(supplier.id).await;
    let input = request(&root, &supplier).await;
    let frame = ic_wire::encode(
        &publication::ObservationRequest::Info(input.clone()),
        ic_wire::Encoding::Frame {
            method: publication::OBSERVATION,
            direction: ic_wire::Direction::Request,
        },
        STORE_LIMITS,
    )
    .unwrap();
    assert!(
        pic.update_call(
            root.id,
            CanisterId::anonymous(),
            "observe_store",
            frame.clone()
        )
        .await
        .is_err()
    );
    for target in [
        "aaaaa-aa".to_string(),
        "2vxsx-fae".into(),
        root.id.to_text(),
        "invalid".into(),
    ] {
        let bad = publication::InfoRequest {
            store: target,
            ..input.clone()
        };
        assert_eq!(observe(&root, &bad).await, Err(publication::Error::Invalid));
    }
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    assert_eq!(
        observe(
            &root,
            &publication::InfoRequest {
                expires_at_ns: now,
                ..input.clone()
            }
        )
        .await,
        Err(publication::Error::Expired)
    );
    assert_eq!(
        observe(
            &root,
            &publication::InfoRequest {
                expires_at_ns: now + 2 * publication::MAX_WINDOW_NS,
                ..input.clone()
            }
        )
        .await,
        Err(publication::Error::Invalid)
    );
    let empty = pic.create_canister().await;
    assert_eq!(
        observe(
            &root,
            &publication::InfoRequest {
                store: empty.to_text(),
                ..input.clone()
            }
        )
        .await,
        Err(publication::Error::Uncertain)
    );
    for malformed in [
        vec![],
        frame[..frame.len() - 1].to_vec(),
        ic_wire::encode(
            &input,
            ic_wire::Encoding::Frame {
                method: publication::EXECUTE,
                direction: ic_wire::Direction::Request,
            },
            STORE_LIMITS,
        )
        .unwrap(),
        ic_wire::encode(
            &publication::ObservationRequest::Info(input.clone()),
            ic_wire::Encoding::Frame {
                method: publication::OBSERVATION,
                direction: ic_wire::Direction::Response,
            },
            STORE_LIMITS,
        )
        .unwrap(),
        vec![0; management::LIMITS.bytes + 1],
    ] {
        assert!(
            pic.update_call(root.id, root.operator, "observe_store", malformed)
                .await
                .is_err()
        );
    }
    assert_eq!(observe(&root, &input).await.unwrap().releases, 0);
    assert_eq!(pic.get_stable_memory(root.id).await, root_bytes);
    assert_eq!(pic.get_stable_memory(supplier.id).await, store_bytes);
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
