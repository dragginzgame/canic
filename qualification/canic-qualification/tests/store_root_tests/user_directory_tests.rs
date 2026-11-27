//! Exercise the checked-in fleet with two signed accounts and Root-created shards.
use super::{Client, management, readiness_tests, stored, support};
use canic::auth::{
    enrollment,
    protocol::{EstablishSession, model, wire},
};
use canic::sharding::provisioning::{AuthSubject, Binding, Progress, Request};
use ic_auth_protocol_types::Principal;
use ic_testkit::pocket_ic::{CanisterId, PocketIcBuilder};
use std::{path::Path, time::Duration};

use user_directory_schema::ProvisionRefusal;

type ShardReply = Result<Binding<[u8; 16]>, ProvisionRefusal>;
type Registration = Result<Progress, ProvisionRefusal>;

pub(super) async fn enroll(
    root: &Client<'_>,
    target: CanisterId,
    grant: &enrollment::Grant,
    managed: bool,
) {
    let grant = enrollment::Grant {
        receiver: Principal::from_slice(target.as_slice()),
        ..grant.clone()
    };
    root.update::<_, Result<(), enrollment::Error>>(
        "grant_auth_access",
        enrollment::CONFIGURE,
        &grant,
        enrollment::LIMITS,
    )
    .await
    .unwrap();
    refresh(root, target, managed).await;
}

async fn refresh(root: &Client<'_>, target: CanisterId, managed: bool) {
    if managed {
        root.update::<_, Result<(), enrollment::Error>>(
            "refresh_auth_receiver",
            enrollment::TRIGGER_REFRESH,
            &Principal::from_slice(target.as_slice()),
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

pub(super) async fn query<T: serde::de::DeserializeOwned>(
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
    directory(false, false).await;
}

#[tokio::test]
async fn enrollment_revocation_during_delivery_fences_registration() {
    directory(true, false).await;
}

#[tokio::test]
async fn stable_sessions_reopen_database_roles_after_root_refresh() {
    directory(false, true).await;
}

async fn directory(revoke: bool, upgrade_after_first: bool) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/examples/user-directory");
    let wasm = |name: &str| std::fs::read(dir.join(format!("artifacts/{name}.wasm"))).unwrap();
    let initial_roles = std::env::var_os("CANIC_PREDECESSOR_DIRECTORY_ROLES")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| dir.join("artifacts"));
    let initial_role = |name: &str| {
        let path = if upgrade_after_first {
            dir.join("artifacts").join(format!("{name}.wasm"))
        } else {
            initial_roles.join(format!("{name}.wasm"))
        };
        std::fs::read(path).unwrap()
    };
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
    let (root, store) =
        readiness_tests::fixture_with_code(pic, wasm("root"), wasm("wasm_store")).await;
    let hub = pic
        .create_canister_with_settings(Some(root.operator), None)
        .await;
    pic.add_cycles(hub, 10_000_000_000_000).await;
    let mut config = canic_host::load_config(&dir.join("canic.toml")).unwrap();
    let local = config
        .apps
        .get_mut("user-directory")
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
    for (name, id) in [("root", root.id), ("store", store.id), ("hub", hub)] {
        local.canisters.insert(
            name.into(),
            canic_host::CanisterBinding { id: id.to_text() },
        );
    }
    let policy = local.policy.shard_pools.first_mut().unwrap();
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
            "shard",
            manifest,
            management::application::Arguments::new(nonce).unwrap(),
        )
        .unwrap();
    pic.set_controllers(store.id, Some(root.operator), vec![root.id])
        .await
        .unwrap();
    let service = fleet
        .bind_hub_operator("hub", &root.operator.to_text(), vec![5; 16])
        .unwrap();
    let args = fleet
        .receiver_install_args("hub", &pool.root, service)
        .unwrap();
    let install = canic::auth::install::Install::decode(&args).unwrap();
    pic.set_controllers(hub, Some(root.operator), vec![root.operator, root.id])
        .await
        .unwrap();
    pic.install_canister(hub, initial_role("user_hub"), args, Some(root.id))
        .await;
    root.update::<_, Result<(), management::Refusal>>(
        "approve_child_creation",
        stored::APPROVE_CHILD_CREATION,
        &stored::ChildCreationGrant::Shared(pool.clone()),
        stored::LIMITS,
    )
    .await
    .unwrap();
    super::configure_routes(&root, hub, &pool.pool, 1)
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
                .update_call(
                    id,
                    browser,
                    "canic_establish_portable_session",
                    auth.clone(),
                )
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
            client.update::<_, Registration>(
                "register",
                ic_wire::method_id("register"),
                &(),
                canic::method::LIMITS,
            )
        };
        if device == 11 {
            assert_eq!(register().await, Ok(Progress::PoolFull));
            assert_eq!(
                query::<Option<Principal>>(&client, "my_shard")
                    .await
                    .unwrap(),
                None
            );
            break;
        }
        assert_eq!(register().await, Ok(Progress::PlacementPending));
        super::settle(pic).await;
        if upgrade_after_first {
            assert_eq!(register().await, Ok(Progress::Ready));
        } else if device == 9 {
            use canic::method::Contract;
            let raw = pic
                .update_call(
                    root.id,
                    hub,
                    canic::sharding::payment::AcquirePlacement::NAME,
                    crate::store_root_tests::placement_request(
                        canic::sharding::payment::Demand::ShardCapacity("shard".into(), 0),
                    ),
                )
                .await
                .unwrap();
            let shard = crate::store_root_tests::placed(&raw)
                .assigned_target()
                .unwrap()
                .unwrap()
                .1;
            let shard = CanisterId::from_slice(shard.as_slice());
            pic.stop_canister(shard, Some(root.id)).await.unwrap();
            assert_eq!(register().await, Ok(Progress::ProvisionPending));
            pic.stop_canister(root.id, Some(root.operator))
                .await
                .unwrap();
            assert_eq!(register().await, Ok(Progress::ProvisionPending));
            pic.start_canister(root.id, Some(root.operator))
                .await
                .unwrap();
            pic.stop_canister(hub, Some(root.operator)).await.unwrap();
            let pending = pic
                .take_canister_snapshot(hub, Some(root.operator), None)
                .await
                .unwrap();
            pic.start_canister(hub, Some(root.operator)).await.unwrap();
            let empty = pic
                .take_canister_snapshot(shard, Some(root.id), None)
                .await
                .unwrap();
            pic.start_canister(shard, Some(root.id)).await.unwrap();

            // Give the real shard a conflicting account ID before retrying the hub reservation.
            let conflict = Binding {
                id: [0xff; 16],
                account: AuthSubject {
                    issuer: install.issuer,
                    subject: Principal::from_slice(&[device, 2]),
                },
                shard: Principal::from_slice(shard.as_slice()),
            };
            let parent = Client {
                pic,
                id: shard,
                operator: hub,
            };
            assert_eq!(
                parent
                    .update::<_, ShardReply>(
                        "provision_user",
                        ic_wire::method_id("provision_user"),
                        &Request {
                            binding: conflict.clone(),
                            data: ()
                        },
                        canic::sharding::LIMITS
                    )
                    .await,
                Ok(conflict)
            );
            let pending_rows = pic.get_stable_memory(hub).await;
            let conflicting_rows = pic.get_stable_memory(shard).await;
            assert_eq!(register().await, Err(ProvisionRefusal::BindingConflict));
            assert!(pic.get_stable_memory(hub).await == pending_rows);
            assert!(pic.get_stable_memory(shard).await == conflicting_rows);

            // Restore the empty test shard and exercise exact replay under the same assignment.
            pic.stop_canister(shard, Some(root.id)).await.unwrap();
            pic.load_canister_snapshot(shard, Some(root.id), empty.id)
                .await
                .unwrap();
            pic.start_canister(shard, Some(root.id)).await.unwrap();
            assert_eq!(register().await, Ok(Progress::Ready));
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
            assert_eq!(register().await, Ok(Progress::Ready));
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
                    ic_wire::decode::<Registration>(
                        &bytes,
                        frame(ic_wire::Direction::Response),
                        canic::method::LIMITS
                    )
                    .unwrap(),
                    Ok(Progress::Ready)
                );
            }
        }
        assert_eq!(register().await, Ok(Progress::Ready));
        assert_eq!(register().await, Ok(Progress::Ready));
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
        if upgrade_after_first {
            for (target, role, controller) in [
                (hub, "user_hub", root.operator),
                (shard, "user_shard", root.id),
            ] {
                canic_qualification::upgrade_stopped(
                    pic,
                    target,
                    initial_role(role),
                    vec![],
                    Some(controller),
                )
                .await
                .unwrap();
            }
            assert!(
                query::<Option<Principal>>(&client, "my_shard")
                    .await
                    .unwrap_err()
                    .contains("Unavailable")
            );
            assert!(
                query::<Option<[u8; 16]>>(&child, "my_id")
                    .await
                    .unwrap_err()
                    .contains("Unavailable")
            );
            refresh(&root, hub, false).await;
            refresh(&root, shard, true).await;
            assert_eq!(
                query::<Option<Principal>>(&client, "my_shard")
                    .await
                    .unwrap(),
                Some(Principal::from_slice(shard.as_slice()))
            );
            assert_eq!(
                query::<Option<[u8; 16]>>(&child, "my_id").await.unwrap(),
                Some(id)
            );
            assert_eq!(
                instance.close(Duration::from_secs(10)).await,
                ic_testkit::server::Deletion::Attempted
            );
            assert!(server.stop().unwrap().process.cleanup.is_empty());
            return;
        }
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
    // Reopen the populated shard, then require a fresh Root enrollment.
    let (second, second_id) = assigned[1];
    canic_qualification::upgrade_stopped(pic, second, wasm("user_shard"), vec![], Some(root.id))
        .await
        .unwrap();
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
            "canic_establish_portable_session",
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
    refresh(&root, second, true).await;
    assert_eq!(
        query::<Option<[u8; 16]>>(&reader, "my_id").await.unwrap(),
        Some(second_id)
    );
    canic_qualification::upgrade_stopped(pic, hub, wasm("user_hub"), vec![], Some(root.operator))
        .await
        .unwrap();
    refresh(&root, hub, false).await;
    assert_eq!(
        query::<Option<Principal>>(&Client { id: hub, ..reader }, "my_shard")
            .await
            .unwrap(),
        Some(Principal::from_slice(second.as_slice()))
    );
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
        account: canic::sharding::provisioning::AuthSubject {
            issuer: install.issuer,
            subject: Principal::from_slice(&[9, 2]),
        },
        shard: Principal::from_slice(shard.as_slice()),
    };
    check_receipts(&client, &root, shard, expected, revoke.then_some(&grant)).await;
    assert_eq!(
        instance.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}

// Replace just the disposable test receiver to control reply contents and delivery order.
async fn check_receipts(
    client: &Client<'_>,
    owner: &Client<'_>,
    shard: CanisterId,
    expected: canic::sharding::provisioning::Binding<[u8; 16]>,
    revoke: Option<&enrollment::Grant>,
) {
    let root = owner.id;
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
    let mut invalid_subject = expected.clone();
    invalid_subject.account.subject = Principal::anonymous();
    let encode = |binding| {
        ic_wire::encode(
            &canic::sharding::provisioning::Request { binding, data: () },
            frame("provision_user", Direction::Request),
            canic::sharding::LIMITS,
        )
        .unwrap()
    };
    assert!(
        pic.update_call(shard, root, "provision_user", encode(expected.clone()))
            .await
            .is_err()
    );
    for binding in [wrong_target, invalid_subject, wrong.clone()] {
        let reply = pic
            .update_call(shard, client.id, "provision_user", encode(binding))
            .await
            .unwrap();
        assert_eq!(
            ic_wire::decode::<ShardReply>(
                &reply,
                frame("provision_user", Direction::Response),
                canic::sharding::LIMITS
            )
            .unwrap(),
            Err(ProvisionRefusal::BindingConflict)
        );
    }
    assert!(pic.get_stable_memory(shard).await == original_rows);
    let issuer = expected.account.issuer;
    let replies = [Ok::<_, ProvisionRefusal>(wrong), Ok(expected)].map(|reply| {
        ic_wire::encode(
            &reply,
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
        if slot == 0 {
            // A second browser completes a scoped read while the first write is suspended.
            pic.advance_time(Duration::from_secs(3)).await;
            let (browser, input) = crate::credential::signed(
                pic.get_time().await.as_nanos_since_unix_epoch(),
                12,
                0,
                issuer,
                Principal::from_slice(&[12, 2]),
                &["read"],
            );
            let reply = pic
                .update_call(
                    client.id,
                    browser,
                    "canic_establish_portable_session",
                    EstablishSession::encode_request(&input).unwrap(),
                )
                .await
                .unwrap();
            EstablishSession::decode_response(&reply).unwrap().unwrap();
            let denied = pic
                .update_call(client.id, browser, "register", args.clone())
                .await
                .unwrap_err();
            assert!(
                denied.reject_message.contains("ScopeRejected"),
                "{denied:?}"
            );
            assert_eq!(
                query::<Option<Principal>>(
                    &Client {
                        operator: browser,
                        ..*client
                    },
                    "my_shard"
                )
                .await
                .unwrap(),
                None
            );
        }
        if slot == 1
            && let Some(grant) = revoke
        {
            owner
                .update::<_, Result<(), enrollment::Error>>(
                    "revoke_auth_access",
                    enrollment::REVOKE,
                    grant,
                    enrollment::LIMITS,
                )
                .await
                .unwrap();
            Client {
                id: client.id,
                ..*owner
            }
            .update::<_, Result<(), enrollment::Error>>(
                enrollment::REFRESH_METHOD,
                enrollment::REFRESH,
                &(),
                enrollment::LIMITS,
            )
            .await
            .unwrap();
        } else if slot == 1 {
            let bytes = pic
                .update_call(
                    client.id,
                    client.operator,
                    "canic_logout_portable_session",
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
