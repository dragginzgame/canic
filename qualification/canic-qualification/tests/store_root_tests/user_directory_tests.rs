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
            &wasm("user_shard"),
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
    pic.install_canister(hub, wasm("user_hub"), args, Some(root.operator))
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
    assert_eq!(
        instance.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}
