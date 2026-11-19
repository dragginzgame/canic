//! Exercise the checked-in fleet with two signed accounts and Root-created shards.
use super::{Client, management, readiness_tests, stored, support};
use canic::auth::{
    enrollment,
    protocol::{EstablishSession, model, wire},
};
use canic::sharding::provisioning::Progress;
use ic_auth_protocol_types::Principal;
use ic_testkit::pocket_ic::{CanisterId, PocketIcBuilder};
use std::{path::Path, time::Duration};

async fn enroll(root: &Client<'_>, target: CanisterId, grant: &enrollment::Grant, managed: bool) {
    let grant = enrollment::Grant {
        receiver: Principal::from_slice(target.as_slice()),
        ..grant.clone()
    };
    root.update::<_, Result<(), enrollment::Error>>(
        "configure_auth",
        enrollment::CONFIGURE,
        &grant,
        enrollment::LIMITS,
    )
    .await
    .unwrap();
    if managed {
        root.update::<_, Result<(), enrollment::Error>>(
            "refresh_auth_receiver",
            enrollment::TRIGGER_REFRESH,
            &grant.receiver,
            enrollment::LIMITS,
        )
        .await
        .unwrap();
    } else {
        Client {
            id: target,
            ..*root
        }
        .update::<_, Result<(), enrollment::Error>>(
            enrollment::REFRESH_METHOD,
            enrollment::REFRESH,
            &(),
            enrollment::LIMITS,
        )
        .await
        .unwrap();
    }
}

async fn query<T: serde::de::DeserializeOwned>(
    client: &Client<'_>,
    method: &str,
) -> Result<T, String> {
    let frame = |direction| ic_wire::Encoding::Frame {
        method: ic_wire::method_id(method),
        direction,
    };
    let bytes = ic_wire::encode(
        &(),
        frame(ic_wire::Direction::Request),
        canic::method::LIMITS,
    )
    .unwrap();
    let reply = client
        .pic
        .query_call(client.id, client.operator, method, bytes)
        .await
        .map_err(|e| e.reject_message)?;
    Ok(ic_wire::decode(
        &reply,
        frame(ic_wire::Direction::Response),
        canic::method::LIMITS,
    )
    .unwrap())
}

#[tokio::test]
async fn accounts_register_on_distinct_shards_with_independent_auth() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/examples/user-directory");
    let wasm = |name: &str| std::fs::read(dir.join(format!("artifacts/{name}.wasm"))).unwrap();
    let initial_roles = std::env::var_os("CANIC_PREDECESSOR_DIRECTORY_ROLES")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| dir.join("artifacts"));
    let initial_role =
        |name: &str| std::fs::read(initial_roles.join(format!("{name}.wasm"))).unwrap();
    let (mut server, mut instance) = support::instance_with(
        &canic_qualification::server_directory(),
        PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet(),
    )
    .await;
    let gateway = instance.client_mut().make_live(None).await;
    instance.client_mut().stop_live().await;
    let pic = instance.client();
    let (root, store) = readiness_tests::fixture_with_code(pic, wasm("root"), wasm("store")).await;
    let hub = pic
        .create_canister_with_settings(Some(root.operator), None)
        .await;
    pic.add_cycles(hub, 10_000_000_000_000).await;
    let mut config = canic_host::load_config(&dir.join("canic.toml")).unwrap();
    let local = config
        .apps
        .get_mut("ex-user-directory")
        .unwrap()
        .fleets
        .get_mut("local")
        .unwrap();
    local.network = canic_host::FleetNetwork::Bound(canic_host::Network::Local {
        url: gateway.to_string(),
        root_key: pic
            .root_key()
            .await
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        root_subnet: Some(pic.topology().await.get_nns().unwrap().to_text()),
        routing_canister: root.id.to_text(),
    });
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    for (name, id) in [
        ("root", root.id),
        ("store", store.id),
        ("ex-user-directory-hub", hub),
    ] {
        local.canisters.insert(
            name.into(),
            canic_host::CanisterBinding {
                id: id.to_text(),
                profile_manager: false,
            },
        );
    }
    let policy = local.shard_pool.as_mut().unwrap();
    policy.expires_at_ns = Some(now + 600_000_000_000);
    let manifest = store
        .publish_named(
            &policy.release.name,
            &policy.release.version,
            &initial_role("user_shard"),
        )
        .await;
    let nonce = std::fs::read(dir.join(policy.install_args.as_ref().unwrap())).unwrap();
    let fleet = config.select(None, None).unwrap().config;
    let pool = fleet
        .shard_pool_approval(
            manifest,
            management::application::Arguments::new(nonce).unwrap(),
        )
        .unwrap();
    pic.set_controllers(store.id, Some(root.operator), vec![root.id])
        .await
        .unwrap();
    let args = fleet
        .receiver_install_args("ex-user-directory-hub", &pool.root, vec![5; 16])
        .unwrap();
    let install = canic::auth::install::Install::decode(&args).unwrap();
    pic.install_canister(hub, initial_role("user_hub"), args, Some(root.operator))
        .await;
    root.update::<_, Result<(), management::Refusal>>(
        "configure_shards",
        stored::CONFIGURE_SHARDS,
        &pool,
        stored::LIMITS,
    )
    .await
    .unwrap();
    let grant = enrollment::Grant {
        receiver: Principal::from_slice(hub.as_slice()),
        issuer: install.issuer,
        audience: model::AudienceId {
            canonical_network_id: install.network,
            fleet_id: install.audience,
        },
        key: canic_auth_witness::KEY,
        grant: model::DelegatedRoleGrant {
            target: wire::Text::new(install.role).unwrap(),
            scopes: wire::List::new(
                ["read", "write"]
                    .map(|s| wire::Text::new(s.to_string()).unwrap())
                    .to_vec(),
            )
            .unwrap(),
        },
        not_before_ns: now,
        expires_at_ns: now + 600_000_000_000,
    };
    enroll(&root, hub, &grant, false).await;
    let mut replay_snapshot = None;
    let mut assigned = Vec::new();
    for device in [9, 10, 11] {
        // Receiver verification credit accrues against replica time.
        pic.advance_time(Duration::from_secs(3)).await;
        let (browser, input) = crate::credential::signed(
            pic.get_time().await.as_nanos_since_unix_epoch(),
            device,
            0,
            install.issuer,
            Principal::from_slice(&[device, 2]),
            &["read", "write"],
        );
        let auth = EstablishSession::encode_request(&input).unwrap();
        let login = async |id| {
            let reply = pic
                .update_call(id, browser, "establish_portable_session", auth.clone())
                .await
                .unwrap();
            EstablishSession::decode_response(&reply).unwrap()
        };
        login(hub).await.unwrap();
        let client = Client {
            pic,
            id: hub,
            operator: browser,
        };
        let register = || {
            client.update::<_, Progress>(
                "register",
                ic_wire::method_id("register"),
                &(),
                canic::method::LIMITS,
            )
        };
        if device == 11 {
            assert_eq!(register().await, Progress::PoolFull);
            assert_eq!(
                query::<Option<Principal>>(&client, "my_shard")
                    .await
                    .unwrap(),
                None
            );
            break;
        }
        assert_eq!(register().await, Progress::PlacementPending);
        super::settle(pic).await;
        if device == 9 {
            // Observe through the actual hub's Root lookup. Derive the public key independently.
            use sha2::{Digest, Sha256};
            let mut digest = Sha256::new().chain_update(b"example.user-directory.account.v1");
            for pid in [install.issuer, Principal::from_slice(&[device, 2])] {
                digest.update([u8::try_from(pid.as_slice().len()).unwrap()]);
                digest.update(pid.as_slice());
            }
            let observation: canic::sharding::Placement = Client {
                operator: hub,
                ..root
            }
            .update(
                "lookup_shard",
                canic::sharding::LOOKUP,
                &canic::sharding::Key::from(digest.finalize()),
                canic::sharding::LIMITS,
            )
            .await;
            let shard = observation.assigned_target().unwrap().unwrap().1;
            let shard = CanisterId::from_slice(shard.as_slice());
            pic.stop_canister(shard, Some(root.id)).await.unwrap();
            assert_eq!(register().await, Progress::ProvisionPending);
            pic.stop_canister(root.id, Some(root.operator))
                .await
                .unwrap();
            assert_eq!(register().await, Progress::PlacementUnavailable);
            pic.start_canister(root.id, Some(root.operator))
                .await
                .unwrap();
            pic.stop_canister(hub, Some(root.operator)).await.unwrap();
            let pending = pic
                .take_canister_snapshot(hub, Some(root.operator), None)
                .await
                .unwrap();
            pic.start_canister(hub, Some(root.operator)).await.unwrap();
            pic.start_canister(shard, Some(root.id)).await.unwrap();
            assert_eq!(register().await, Progress::Ready);
            let populated = pic.get_stable_memory(shard).await;
            // Recreate exactly a lost acknowledgement: pending hub, already committed shard.
            pic.stop_canister(hub, Some(root.operator)).await.unwrap();
            pic.load_canister_snapshot(hub, Some(root.operator), pending.id.clone())
                .await
                .unwrap();
            pic.start_canister(hub, Some(root.operator)).await.unwrap();
            assert_eq!(
                query::<Option<Principal>>(&client, "my_shard")
                    .await
                    .unwrap(),
                None
            );
            assert_eq!(register().await, Progress::Ready);
            assert!(pic.get_stable_memory(shard).await == populated);
            replay_snapshot = Some((pending.id, browser));
        } else {
            // Admit two distinct ingress messages before advancing either registration.
            let frame = |direction| ic_wire::Encoding::Frame {
                method: ic_wire::method_id("register"),
                direction,
            };
            let args = ic_wire::encode(
                &(),
                frame(ic_wire::Direction::Request),
                canic::method::LIMITS,
            )
            .unwrap();
            let first = pic
                .submit_call(hub, browser, "register", args.clone())
                .await
                .unwrap();
            let second = pic
                .submit_call(hub, browser, "register", args)
                .await
                .unwrap();
            assert_ne!(first.message_id, second.message_id);
            for ingress in [first, second] {
                let bytes = pic.await_call(ingress).await.unwrap();
                assert_eq!(
                    ic_wire::decode::<Progress>(
                        &bytes,
                        frame(ic_wire::Direction::Response),
                        canic::method::LIMITS
                    )
                    .unwrap(),
                    Progress::Ready
                );
            }
        }
        assert_eq!(register().await, Progress::Ready);
        assert_eq!(register().await, Progress::Ready);
        let shard = query::<Option<Principal>>(&client, "my_shard")
            .await
            .unwrap()
            .unwrap();
        let shard = CanisterId::from_slice(shard.as_slice());
        let child = Client {
            id: shard,
            ..client
        };
        let refusal = query::<Option<[u8; 16]>>(&child, "my_id")
            .await
            .unwrap_err();
        assert!(
            refusal.contains("user authorization: Unavailable"),
            "{refusal}"
        );
        assert_eq!(
            login(shard).await,
            Err(model::AdmissionRefusal::Unavailable)
        );
        assert_eq!(
            query::<Option<Principal>>(&client, "my_shard")
                .await
                .unwrap(),
            Some(Principal::from_slice(shard.as_slice()))
        );
        enroll(&root, shard, &grant, true).await;
        login(shard).await.unwrap();
        let id = query::<Option<[u8; 16]>>(&child, "my_id")
            .await
            .unwrap()
            .unwrap();
        if let Some(&(other, other_id)) = assigned.first() {
            assert_ne!(shard, other);
            assert_ne!(id, other_id);
            login(other).await.unwrap();
            assert_eq!(
                query::<Option<[u8; 16]>>(
                    &Client {
                        id: other,
                        ..client
                    },
                    "my_id"
                )
                .await
                .unwrap(),
                None
            );
        }
        assigned.push((shard, id));
    }
    assert_eq!(assigned.len(), 2);
    // A populated volatile shard must reopen its rows without renewing lost Auth authority.
    let (second, _) = assigned[1];
    let retained = pic.get_stable_memory(second).await;
    canic_qualification::upgrade_stopped(pic, second, wasm("user_shard"), vec![], Some(root.id))
        .await
        .unwrap();
    assert!(pic.get_stable_memory(second).await == retained);
    assert_eq!(
        root.update::<_, Result<(), enrollment::Error>>(
            "refresh_auth_receiver",
            enrollment::TRIGGER_REFRESH,
            &Principal::from_slice(second.as_slice()),
            enrollment::LIMITS,
        )
        .await,
        Err(enrollment::Error::Closed)
    );
    let (browser, fresh) = crate::credential::signed(
        pic.get_time().await.as_nanos_since_unix_epoch(),
        10,
        1,
        install.issuer,
        Principal::from_slice(&[10, 2]),
        &["read"],
    );
    let reader = Client {
        pic,
        id: second,
        operator: browser,
    };
    assert!(
        query::<Option<[u8; 16]>>(&reader, "my_id")
            .await
            .unwrap_err()
            .contains("user authorization: Unavailable")
    );
    let reply = pic
        .update_call(
            second,
            browser,
            "establish_portable_session",
            EstablishSession::encode_request(&fresh).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        EstablishSession::decode_response(&reply).unwrap(),
        Err(model::AdmissionRefusal::Unavailable)
    );
    assert_eq!(
        query::<Option<Principal>>(&Client { id: hub, ..reader }, "my_shard")
            .await
            .unwrap(),
        Some(Principal::from_slice(second.as_slice()))
    );
    let hub_memory = pic.get_stable_memory(hub).await;
    canic_qualification::upgrade_stopped(pic, hub, wasm("user_hub"), vec![], Some(root.operator))
        .await
        .unwrap();
    assert!(pic.get_stable_memory(hub).await == hub_memory);
    let (snapshot, browser) = replay_snapshot.unwrap();
    pic.stop_canister(hub, Some(root.operator)).await.unwrap();
    pic.load_canister_snapshot(hub, Some(root.operator), snapshot)
        .await
        .unwrap();
    pic.start_canister(hub, Some(root.operator)).await.unwrap();
    let client = Client {
        pic,
        id: hub,
        operator: browser,
    };
    let (shard, id) = assigned[0];
    let expected = canic::sharding::provisioning::Binding {
        id,
        account: canic::sharding::provisioning::Account {
            issuer: install.issuer,
            subject: Principal::from_slice(&[9, 2]),
        },
        shard: Principal::from_slice(shard.as_slice()),
    };
    check_receipts(&client, root.id, shard, expected).await;
    assert_eq!(
        instance.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}

// Replace just the disposable test receiver to control reply contents and delivery order.
async fn check_receipts(
    client: &Client<'_>,
    root: CanisterId,
    shard: CanisterId,
    expected: canic::sharding::provisioning::Binding<[u8; 16]>,
) {
    use canic::auth::protocol::Logout;
    use ic_testkit::held::{Catalog, Fixture, Limits, Reply};
    use ic_wire::{Direction, Encoding};
    let pic = client.pic;
    let frame = |name, direction| Encoding::Frame {
        method: ic_wire::method_id(name),
        direction,
    };
    let mut wrong = expected.clone();
    wrong.id[0] ^= 1;
    // Exercise the real shard before replacing it with the controlled responder.
    let original_rows = pic.get_stable_memory(shard).await;
    let mut wrong_target = expected.clone();
    wrong_target.shard = Principal::from_slice(root.as_slice());
    for (caller, binding) in [
        (root, expected.clone()),
        (client.id, wrong_target),
        (client.id, wrong.clone()),
    ] {
        let bytes = ic_wire::encode(
            &canic::sharding::provisioning::Request { binding, data: () },
            frame("provision_user", Direction::Request),
            canic::sharding::LIMITS,
        )
        .unwrap();
        assert!(
            pic.update_call(shard, caller, "provision_user", bytes)
                .await
                .is_err()
        );
    }
    assert!(pic.get_stable_memory(shard).await == original_rows);
    let replies = [wrong, expected].map(|binding| {
        ic_wire::encode(
            &binding,
            frame("provision_user", Direction::Response),
            canic::sharding::LIMITS,
        )
        .unwrap()
    });
    let catalog = Catalog::new(
        &replies.each_ref().map(|bytes| Reply {
            method: "provision_user",
            bytes,
        }),
        Limits {
            slots: 2,
            name_bytes: 32,
            reply_bytes: 1024,
        },
    )
    .unwrap();
    let mut held = Fixture::new(
        pic,
        shard.to_text().parse().unwrap(),
        &catalog,
        100_000_000_000,
        8192,
    )
    .await
    .unwrap();
    canic_qualification::upgrade_stopped(pic, shard, held.wasm().to_vec(), vec![], Some(root))
        .await
        .unwrap();
    let args = ic_wire::encode(
        &(),
        frame("register", Direction::Request),
        canic::method::LIMITS,
    )
    .unwrap();
    let pending_memory = pic.get_stable_memory(client.id).await;
    for slot in 0..2 {
        let call = pic
            .submit_call(client.id, client.operator, "register", args.clone())
            .await
            .unwrap();
        let mut pending = vec![];
        for _ in 0..40 {
            pic.tick().await;
            pending = held.pending(pic).await.unwrap();
            if !pending.is_empty() {
                break;
            }
        }
        let token = pending
            .pop()
            .expect("provisioning call reached assigned shard");
        assert_eq!(token.slot(), slot);
        if slot == 1 {
            let bytes = pic
                .update_call(
                    client.id,
                    client.operator,
                    "logout_portable_session",
                    Logout::encode_request(&()).unwrap(),
                )
                .await
                .unwrap();
            assert!(matches!(
                Logout::decode_response(&bytes).unwrap(),
                model::LogoutResult::Revoked(_)
            ));
        }
        held.release(pic, &token).await.unwrap();
        let err = pic.await_call(call).await.unwrap_err();
        if slot == 0 {
            assert!(
                err.reject_message.contains("changed provisioning binding"),
                "{err:?}"
            );
            assert_eq!(
                query::<Option<Principal>>(client, "my_shard")
                    .await
                    .unwrap(),
                None
            );
        } else {
            assert!(err.reject_message.contains("endpoint refused"), "{err:?}");
        }
        assert!(pic.get_stable_memory(client.id).await == pending_memory);
    }
}
