//! Two durable components under one application-owned bootstrap and upgrade boundary.
use super::*;
use canic_profile_protocol::{GET, PUT, ProfileError, ProfileResult, Put};

async fn profile(client: &Client<'_>, user: u8, id: u64) -> ProfileResult {
    client.query(actor(user), "profile_get", GET, &id).await
}

#[tokio::test(flavor = "current_thread")]
async fn composed_catalog_rollback_and_capacity_leave_profiles_usable() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let client = install_bytes(
        pic,
        std::fs::read(directory().join("composed.wasm")).unwrap(),
    )
    .await;
    let mut profiles = Vec::new();
    for user in [1, 2] {
        let saved: ProfileResult = client
            .update(
                actor(user),
                "profile_put",
                PUT,
                &Put {
                    user_id: u64::from(user),
                    expected_revision: 0,
                    display_name: format!("Owner {user}"),
                },
            )
            .await;
        profiles.push(saved.unwrap());
    }

    // Includes actual Blob/catalog mutations followed by persistence and reply
    // traps. Its whole-backing comparisons now cover both profile allocations too.
    catalog_validation_and_rollback(&client).await;
    for saved in &profiles {
        assert_eq!(
            profile(&client, u8::try_from(saved.user_id).unwrap(), saved.user_id).await,
            Ok(saved.clone())
        );
    }
    let asset = client.asset(actor(2), 2).await.unwrap();
    let change = Put {
        user_id: 1,
        expected_revision: 1,
        display_name: "Catalog full; profile still writable".into(),
    };
    let changed: ProfileResult = client.update(actor(1), "profile_put", PUT, &change).await;
    assert_eq!(changed.as_ref().unwrap().revision, 2);
    assert_eq!(profile(&client, 1, 1).await, changed);
    assert_eq!(client.asset(actor(2), 2).await.unwrap(), asset);

    // Cross-mount, truncated and oversized input must fail before profile writes.
    let valid = frame(PUT, &change);
    let before = pic.get_stable_memory(client.canister).await;
    for bytes in [
        frame(app::AUTHORIZE, &client.draft(5).await),
        valid[..valid.len() - 1].to_vec(),
        vec![0; canic_profile_protocol::LIMITS.bytes + 1],
    ] {
        assert!(
            pic.update_call(client.canister, actor(1), "profile_put", bytes)
                .await
                .is_err()
        );
        assert_eq!(pic.get_stable_memory(client.canister).await, before);
    }
    let stale = Put {
        display_name: "Stale revision must not commit".into(),
        ..change
    };
    let refused: ProfileResult = client.update(actor(1), "profile_put", PUT, &stale).await;
    assert_eq!(refused, Err(ProfileError::RevisionConflict));
    assert_eq!(pic.get_stable_memory(client.canister).await, before);
    assert_eq!(profile(&client, 1, 1).await, changed);
    assert_eq!(client.asset(actor(2), 2).await.unwrap(), asset);
    assert!(client.ready().await);

    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

#[tokio::test(flavor = "current_thread")]
async fn corrupt_profile_participant_refuses_upgrade_without_reinitializing_other_owners() {
    use ic_storage::{
        AllocationGrants, GenericAllocationPolicy, MemoryAuthority, MemoryRuntime,
        SealedDeclarationSnapshot, StableKey, VectorMemory,
    };

    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    for key in ["profile.rows.v1", "profile.owners.v1"] {
        let client = install_bytes(
            pic,
            std::fs::read(directory().join("composed.wasm")).unwrap(),
        )
        .await;
        first(&client).await;
        let saved: ProfileResult = client
            .update(
                actor(1),
                "profile_put",
                PUT,
                &Put {
                    user_id: 1,
                    expected_revision: 0,
                    display_name: "retained profile".into(),
                },
            )
            .await;
        saved.unwrap();
        client.command(actor(8), Command::Drain(1)).await.unwrap();
        let management = client.management().await;
        let asset = client.asset(actor(1), 1).await.unwrap();
        let original = pic.get_stable_memory(client.canister).await;

        // Address the selected allocation through Storage, never guessed physical
        // offsets. Keep the directory, catalog and real Blob record intact.
        let backing = VectorMemory::new(std::cell::RefCell::new(original.clone()));
        {
            let mut runtime = MemoryRuntime::new(backing.clone()).unwrap();
            let requests = canic_profile_worker::state::memory_requests();
            let authority = MemoryAuthority::new(
                canic_profile_worker::state::STORAGE_AUTHORITY,
                canic_profile_worker::state::STORAGE_PREFIX,
            )
            .unwrap();
            runtime
                .bootstrap(
                    &SealedDeclarationSnapshot::new(requests.to_vec()).unwrap(),
                    &AllocationGrants::new(vec![authority]).unwrap(),
                    &GenericAllocationPolicy,
                )
                .unwrap();
            assert_eq!(
                *backing.borrow(),
                original,
                "readmission must not rewrite backing"
            );
            let memory = runtime
                .open_memory(&StableKey::parse(key).unwrap())
                .unwrap();
            memory.write(0, &[0; 8]);
            assert_eq!(runtime.memory_allocations().unwrap().len(), 4);
        }
        let damaged = backing.borrow().clone();
        assert_ne!(damaged, original);
        pic.set_stable_memory(
            client.canister,
            damaged.clone(),
            ic_testkit::pocket_ic::common::rest::BlobCompression::NoCompression,
        )
        .await;
        assert!(
            client
                .upgrade("composed-candidate.wasm", frame(app::UPGRADE, &()))
                .await
                .is_err(),
            "corrupt {key} must refuse reopening"
        );
        assert_eq!(
            pic.get_stable_memory(client.canister).await,
            damaged,
            "failed participant cannot trigger initialization or change another owner"
        );
        assert_eq!(client.management().await, management);
        assert_eq!(client.asset(actor(1), 1).await.unwrap(), asset);
        assert!(!client.ready().await);
        assert!(
            pic.query_call(client.canister, actor(1), "profile_get", frame(GET, &1_u64))
                .await
                .is_err()
        );
    }
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

#[tokio::test(flavor = "current_thread")]
async fn composed_profiles_and_blob_preserve_one_storage_and_activation_boundary() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let client = install_bytes(
        pic,
        std::fs::read(directory().join("composed.wasm")).unwrap(),
    )
    .await;
    first(&client).await;
    let asset = client.asset(actor(1), 1).await.unwrap();
    let alice = Put {
        user_id: 1,
        expected_revision: 0,
        display_name: "Alice".into(),
    };
    let saved: ProfileResult = client.update(actor(1), "profile_put", PUT, &alice).await;
    assert_eq!(saved.as_ref().unwrap().revision, 1);
    let bob = Put {
        user_id: 2,
        display_name: "Bob".into(),
        ..alice.clone()
    };
    let second: ProfileResult = client.update(actor(2), "profile_put", PUT, &bob).await;
    assert_eq!(profile(&client, 1, 1).await, saved);
    assert_eq!(profile(&client, 2, 1).await, Err(ProfileError::Forbidden));
    assert_eq!(
        profile(&client, 9, 1).await,
        Err(ProfileError::Forbidden),
        "controller is not a user"
    );

    client.fault(2).await;
    let before = pic.get_stable_memory(client.canister).await;
    let change = Put {
        expected_revision: 1,
        display_name: "must roll back".into(),
        ..alice
    };
    assert!(
        pic.update_call(
            client.canister,
            actor(1),
            "profile_put",
            frame(PUT, &change)
        )
        .await
        .is_err()
    );
    assert_eq!(pic.get_stable_memory(client.canister).await, before);
    client.fault(0).await;
    assert_eq!(profile(&client, 1, 1).await, saved);
    assert_eq!(client.asset(actor(1), 1).await.unwrap(), asset);

    assert!(
        client
            .upgrade("composed-candidate.wasm", frame(app::UPGRADE, &()))
            .await
            .is_err()
    );
    client.command(actor(8), Command::Drain(1)).await.unwrap();
    let closed = pic.get_stable_memory(client.canister).await;
    assert!(
        pic.query_call(client.canister, actor(1), "profile_get", frame(GET, &1_u64))
            .await
            .is_err()
    );
    assert!(
        pic.update_call(
            client.canister,
            actor(1),
            "profile_put",
            frame(PUT, &change)
        )
        .await
        .is_err()
    );
    assert_eq!(pic.get_stable_memory(client.canister).await, closed);
    assert!(
        client
            .upgrade("original.wasm", frame(app::UPGRADE, &()))
            .await
            .is_err(),
        "cannot silently remove a durable participant"
    );
    assert_eq!(pic.get_stable_memory(client.canister).await, closed);
    client
        .upgrade("composed-candidate.wasm", frame(app::UPGRADE, &()))
        .await
        .unwrap();
    assert_eq!(pic.get_stable_memory(client.canister).await, closed);
    assert!(!client.ready().await);
    assert!(
        pic.query_call(client.canister, actor(1), "profile_get", frame(GET, &1_u64))
            .await
            .is_err()
    );
    client
        .command(actor(8), Command::Activate(1))
        .await
        .unwrap();
    assert_eq!(profile(&client, 1, 1).await, saved);
    assert_eq!(profile(&client, 2, 2).await, second);
    assert_eq!(client.asset(actor(1), 1).await.unwrap(), asset);
    client.command(actor(8), Command::Drain(2)).await.unwrap();
    client
        .upgrade("composed.wasm", frame(app::UPGRADE, &()))
        .await
        .unwrap();
    assert_eq!(
        client.command(actor(8), Command::Activate(1)).await,
        Err(BlobError::MaintenanceAttempt)
    );
    assert!(
        pic.query_call(client.canister, actor(1), "profile_get", frame(GET, &1_u64))
            .await
            .is_err()
    );
    client
        .command(actor(8), Command::Activate(2))
        .await
        .unwrap();
    assert_eq!(profile(&client, 2, 2).await, second);

    // Adding the participant to an existing Blob-only installation is also a
    // migration, not permission to create empty profile stores during reopen.
    let base = install(pic).await;
    first(&base).await;
    base.command(actor(8), Command::Drain(1)).await.unwrap();
    let previous = pic.get_stable_memory(base.canister).await;
    assert!(
        base.upgrade("composed.wasm", frame(app::UPGRADE, &()))
            .await
            .is_err()
    );
    assert_eq!(pic.get_stable_memory(base.canister).await, previous);
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
