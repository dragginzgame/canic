//! A project pays for its DB-backed Auth receiver using the fleet's TOML approval.
use super::*;
use canic::{
    auth::{
        enrollment, install,
        protocol::{EstablishSession, model, wire},
    },
    method::Contract,
    sharding::{
        Placement,
        payment::Payment,
        provisioning::{AuthSubject, Binding, Provision, Request as ProvisionRequest},
    },
};
use ic_auth_protocol_types::Principal;

#[tokio::test]
async fn paid_singleton_reopens_its_parent_bound_receiver() {
    let dir = artifacts();
    let ledger_wasm = std::fs::read(dir.join("db-role-shard.wasm")).unwrap();
    let (mut server, mut instance) = support::instance_with(
        &canic_qualification::server_directory(),
        ic_testkit::pocket_ic::PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet(),
    )
    .await;
    let url = instance.client_mut().make_live(None).await;
    instance.client_mut().stop_live().await;
    let pic = instance.client();
    let (root, store) =
        readiness_tests::fixture_with_root(pic, std::fs::read(dir.join("root.wasm")).unwrap())
            .await;
    let project_wasm = std::fs::read(dir.join("db-role-project.wasm")).unwrap();
    store.publish_named("project", "1", &project_wasm).await;
    store.publish_named("ledger", "1", &ledger_wasm).await;
    pic.set_controllers(store.id, Some(root.operator), vec![root.id])
        .await
        .unwrap();
    let registry = pic.create_canister().await;
    pic.add_cycles(registry, 10_000_000_000_000).await;
    pic.install_canister(
        registry,
        std::fs::read(dir.join("hub.wasm")).unwrap(),
        crate::store_root_tests::hub_args(&root),
        None,
    )
    .await;
    let deadline = pic.get_time().await.as_nanos_since_unix_epoch() + 600_000_000_000;
    let root_key = pic
        .root_key()
        .await
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let root_subnet = pic.topology().await.get_nns().unwrap();
    let source = format!(
        r#"
[apps.projects]
[apps.projects.canisters]
registry = {{ role = "registry" }}
project = {{ role = "project", parent = "registry", wasm = "db-role-project.wasm" }}
ledger = {{ role = "project_ledger", parent = "project", wasm = "db-role-shard.wasm" }}
[apps.projects.fleets.local]
network = {{ kind = "Local", url = "{url}", root_key = "{root_key}", routing_canister = "{registry}", root_subnet = "{root_subnet}" }}
subnet_child_limit = 4
[apps.projects.fleets.local.canisters]
root = {{ id = "{}" }}
store = {{ id = "{}" }}
registry = {{ id = "{registry}" }}
[apps.projects.fleets.local.registry]
component = "project"
root = "root"
store = "store"
release = {{ name = "project", version = "1" }}
max_instances = 1
creation_budget = 1_000_000_000_000
expires_at_ns = {deadline}
[apps.projects.fleets.local.registry.singleton]
component = "ledger"
release = {{ name = "ledger", version = "1" }}
install_args = "db-role-nonce.bin"
maximum = 1
creation_budget = 1_000_000_000_000
[apps.projects.fleets.local.receiver_policy]
root = "root"
roles = ["project_ledger"]
issuer = "{registry}"
role = "user"
network_id = "{}"
application_id = "{}"
limits = {{ default_session_secs = 60, max_session_secs = 1800, max_proof_ttl_ns = 60000000000 }}
"#,
        root.id,
        store.id,
        "11".repeat(32),
        "22".repeat(32)
    );
    let config: canic_host::Config = toml::from_str(&source).unwrap();
    let fleet = config.select(None, None).unwrap();
    let canic_host::FleetPlanning::PlacementPools {
        capacity: Some(capacity),
        instances: Some(project),
        singleton: Some(ledger),
        ..
    } = canic_host::plan_fleet(
        &dir,
        &dir.join("canic.toml"),
        &fleet,
        &root.operator.to_text(),
        None,
        None,
    )
    .unwrap()
    else {
        panic!("expected placement grants")
    };
    root.update::<_, Result<(), management::Refusal>>(
        "configure_subnet_capacity",
        stored::CONFIGURE_SUBNET_CAPACITY,
        &capacity.approval,
        stored::LIMITS,
    )
    .await
    .unwrap();
    root.update::<_, Result<(), management::Refusal>>(
        "approve_child_creation",
        stored::APPROVE_CHILD_CREATION,
        &stored::ChildCreationGrant::Dedicated(project.approval.clone()),
        stored::LIMITS,
    )
    .await
    .unwrap();
    let approve = async |grant: &stored::SingletonGrant| {
        root.update::<_, Result<(), management::Refusal>>(
            "approve_child_creation",
            stored::APPROVE_CHILD_CREATION,
            &stored::ChildCreationGrant::PerParent(grant.clone()),
            stored::LIMITS,
        )
        .await
    };
    // A previously signed frame must fail even if its payload matches the current grant.
    for retired in ["configure_singleton", "canic.configure_singleton.v2"] {
        let request = ic_wire::encode(
            &ledger.approval,
            ic_wire::Encoding::Frame {
                method: ic_wire::method_id(retired),
                direction: ic_wire::Direction::Request,
            },
            stored::LIMITS,
        )
        .unwrap();
        assert!(
            pic.update_call(root.id, root.operator, "approve_child_creation", request)
                .await
                .is_err()
        );
    }
    let mut wrong = ledger.approval.clone();
    let mut policy = install::Install::decode(wrong.receiver.as_ref().unwrap().as_slice()).unwrap();
    policy.root = Principal::from_slice(registry.as_slice());
    wrong.receiver =
        Some(management::application::Arguments::new(policy.encode().unwrap()).unwrap());
    assert_eq!(approve(&wrong).await, Err(management::Refusal::Binding));
    approve(&ledger.approval).await.unwrap();
    let place = async || {
        acquire_fixture(
            &root,
            registry,
            canic::sharding::payment::Demand::Instance([3; 32]),
        )
        .await
    };
    assert!(matches!(place().await, Placement::Pending(_)));
    settle(pic).await;
    let Placement::Assigned(_, project) = place().await else {
        panic!("expected ready project")
    };
    let project = CanisterId::from_text(project).unwrap();
    pic.add_cycles(project, 4_000_000_000_000).await;
    let payer = Client {
        id: project,
        operator: root.id,
        pic,
    };
    let fund = async |attached: u128| {
        payer
            .update::<_, Result<Payment, canic::sharding::payment::Refusal>>(
                "fund_ledger",
                ic_wire::method_id("fund_ledger"),
                &(2_000_000_000_000_u128, attached),
                canic::sharding::LIMITS,
            )
            .await
            .unwrap()
    };
    let Payment::Quoted(quote) = fund(0).await else {
        panic!("expected quote")
    };
    let charge = quote.creation_fee + quote.working_cycles;
    // Discard the settlement acknowledgement. Retry with the original demand and no cycles.
    let request = ic_wire::encode(
        &(2_000_000_000_000_u128, charge),
        ic_wire::Encoding::Frame {
            method: ic_wire::method_id("fund_ledger"),
            direction: ic_wire::Direction::Request,
        },
        canic::sharding::LIMITS,
    )
    .unwrap();
    pic.submit_call(project, root.id, "fund_ledger", request)
        .await
        .unwrap();
    settle(pic).await;
    let before = pic.cycle_balance(project).await;
    let recovered = fund(0).await;
    assert_eq!(fund(charge).await, recovered);
    assert!(before.saturating_sub(pic.cycle_balance(project).await) < charge);
    let Payment::Placed(Placement::Assigned(_, ledger), Some(receipt)) = recovered else {
        panic!("expected installed ledger: {recovered:?}")
    };
    assert_eq!(receipt, quote);
    let ledger = CanisterId::from_text(ledger).unwrap();
    let custody: management::application::Custody = root
        .update(
            "observe_child_deployments",
            management::application::CHILD_EVIDENCE,
            &(),
            stored::LIMITS,
        )
        .await;
    let receiver = custody
        .applications
        .iter()
        .flatten()
        .find_map(|life| install::Install::decode(life.creation.as_slice()).ok())
        .unwrap();
    policy.root = Principal::from_slice(root.id.as_slice());
    policy.application.clone_from(&receiver.application);
    assert_eq!(receiver.encode().unwrap(), policy.encode().unwrap());
    assert_eq!(receiver.role, "user");
    let topology = canic::topology::Install::decode(&receiver.application).unwrap();
    assert_eq!(topology.parent.as_slice(), project.as_slice());
    assert_eq!(topology.application, b"local-shard-v001");
    let subject = Principal::from_slice(&[12, 2]);
    let binding = Binding {
        id: [12; 16],
        account: AuthSubject {
            issuer: receiver.issuer,
            subject,
        },
        shard: Principal::from_slice(ledger.as_slice()),
    };
    type Bootstrap = Provision<user_directory_schema::Users>;
    let request = Bootstrap::encode_request(&ProvisionRequest {
        binding: binding.clone(),
        data: (),
    })
    .unwrap();
    assert!(
        pic.update_call(ledger, registry, Bootstrap::NAME, request.clone())
            .await
            .is_err()
    );
    let reply = pic
        .update_call(ledger, project, Bootstrap::NAME, request)
        .await
        .unwrap();
    assert_eq!(Bootstrap::decode_reply(&reply).unwrap().unwrap(), binding);
    let grant = enrollment::Grant {
        receiver: binding.shard,
        issuer: receiver.issuer,
        audience: model::AudienceId {
            canonical_network_id: [0x11; 32],
            fleet_id: [0x22; 32],
        },
        key: canic_auth_witness::KEY,
        grant: canic::auth::protocol::model::DelegatedRoleGrant {
            target: wire::Text::new("user".into()).unwrap(),
            scopes: wire::List::new(vec![wire::Text::new("read".into()).unwrap()]).unwrap(),
        },
        not_before_ns: pic.get_time().await.as_nanos_since_unix_epoch(),
        expires_at_ns: deadline,
    };
    user_directory_tests::enroll(&root, ledger, &grant, true).await;
    let (browser, proof) = crate::credential::signed(
        pic.get_time().await.as_nanos_since_unix_epoch(),
        12,
        0,
        receiver.issuer,
        subject,
        &["read"],
    );
    let reply = pic
        .update_call(
            ledger,
            browser,
            "canic_establish_portable_session",
            EstablishSession::encode_request(&proof).unwrap(),
        )
        .await
        .unwrap();
    EstablishSession::decode_response(&reply).unwrap().unwrap();
    let user = Client {
        id: ledger,
        operator: browser,
        pic,
    };
    let query = async || user_directory_tests::query::<Option<[u8; 16]>>(&user, "my_id").await;
    assert_eq!(query().await.unwrap(), Some(binding.id));
    canic_qualification::upgrade_stopped(pic, ledger, ledger_wasm, vec![], Some(root.id))
        .await
        .unwrap();
    assert!(query().await.is_err());
    user_directory_tests::enroll(&root, ledger, &grant, true).await;
    assert_eq!(query().await.unwrap(), Some(binding.id));
    assert_eq!(
        instance.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}
