//! One Root fills, replenishes, and admits a pre-existing surplus principal.
use super::*;
use canic::sharding::{
    AssignWorker, Placement,
    standby::{Asset, ImportStandby, InspectStandby, Source},
};

#[tokio::test(flavor = "current_thread")]
async fn fleet_reserve_fills_replenishes_and_imports_once() {
    let base =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/examples/miner-pool/artifacts");
    let (mut server, mut owner) = support::instance_with_lifetime(
        &canic_qualification::server_directory(),
        ic_testkit::pocket_ic::PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet(),
        Duration::from_secs(180),
    )
    .await;
    let pic = owner.client();
    let spare = pic.create_canister().await;
    pic.add_cycles(spare, 1_000_000_000_000).await;
    let registry = CanisterId::from_text("rwlgt-iiaaa-aaaaa-aaaaa-cai").unwrap();
    pic.create_canister_with_id(None, None, registry)
        .await
        .unwrap();
    pic.add_cycles(registry, 1_000_000_000_000).await;
    let foreign = pic
        .create_canister_on_subnet(None, None, pic.topology().await.get_nns().unwrap())
        .await;
    let foreign_id = foreign.to_text().parse().unwrap();
    let foreign_lookup =
        ic_platform_protocol::placement::encode_get_subnet_for_canister_args(foreign_id, 256)
            .unwrap();
    pic.install_canister(
        registry,
        wat::parse_str(include_str!("registry.wat")).unwrap(),
        foreign_lookup,
        None,
    )
    .await;
    let fleet = crate::miner_pool_fixture::install(&mut owner, &base, |fleet| {
        fleet.subnet_child_limit = Some(7);
        let w = fleet.worker_pool.as_mut().unwrap();
        w.creation_budget = 0;
        let spending_deadline = w.expires_at_ns.unwrap();
        w.expires_at_ns = Some(spending_deadline + 7_200_000_000_000);
        fleet.standby_pool = Some(canic_host::StandbyPoolConfig {
            root: "root".into(),
            min_available_spares: 2,
            maximum_creations: 3,
            expires_at_ns: spending_deadline,
            minimum_import_cycles: 1_000_000u64.try_into().unwrap(),
            imports: vec![canic_host::StandbyImports {
                network: "local".into(),
                principals: vec![spare.to_text()],
                previous_root: Some(registry.to_text()),
            }],
        });
    })
    .await;
    let root = &fleet.root;
    let pic = root.pic;
    configure(root, &fleet.capacity, &fleet.workers).await;
    let grant = fleet.standby.unwrap();
    let approve = async |p: &stored::StandbyPolicy| {
        root.update::<_, Result<(), String>>(
            "configure_standby",
            stored::CONFIGURE_STANDBY,
            p,
            stored::LIMITS,
        )
        .await
    };
    let inspect = async || {
        root.update::<_, <InspectStandby as Contract>::Output>(
            InspectStandby::NAME,
            InspectStandby::ID,
            &(),
            InspectStandby::REPLY,
        )
        .await
    };
    let imported: <ImportStandby as Contract>::Input = spare.to_text().parse().unwrap();
    let import = async || {
        root.update::<_, Result<Asset, String>>(
            ImportStandby::NAME,
            ImportStandby::ID,
            &imported,
            ImportStandby::REPLY,
        )
        .await
    };
    assert!(
        pic.update_call(root.id, spare, "configure_standby", grant.request.clone())
            .await
            .is_err()
    );
    let client = base.join("root.client");
    let request = codec::encode(
        &client,
        &[
            "configure-standby".into(),
            serde_json::to_string(&grant.approval).unwrap(),
        ],
    )
    .unwrap();
    assert_eq!(request.arguments, grant.request);
    let reply = pic
        .update_call(root.id, root.operator, &request.method, request.arguments)
        .await
        .unwrap();
    assert_eq!(codec::decode(&client, &reply).unwrap()["approved"], true);
    settle(pic).await;
    assert!(matches!(
        inspect().await.as_slice(),
        [
            (Source::Imported(_), Asset::AwaitingRelease(_)),
            (Source::Automatic(_), Asset::Empty(_)),
            (Source::Automatic(_), Asset::Empty(_))
        ]
    ));
    assert_eq!(import().await.unwrap(), Asset::AwaitingRelease(imported));
    assert!(
        pic.update_call(
            root.id,
            spare,
            ImportStandby::NAME,
            ImportStandby::encode_request(&imported).unwrap()
        )
        .await
        .is_err()
    );
    assert_eq!(import().await.unwrap(), Asset::AwaitingRelease(imported));
    let former_release = async |candidate| {
        let reply = pic
            .update_call(
                root.id,
                registry,
                ImportStandby::NAME,
                ImportStandby::encode_request(&candidate).unwrap(),
            )
            .await
            .unwrap();
        ImportStandby::decode_reply(&reply).unwrap()
    };
    for _ in 0..2 {
        assert_eq!(
            former_release(imported).await.unwrap(),
            Asset::AwaitingHandoff(imported)
        );
    }
    assert!(import().await.is_err());
    pic.set_controllers(spare, None, vec![root.id])
        .await
        .unwrap();
    let first = import().await.unwrap();
    assert!(matches!(first,Asset::Quarantined(id,_) if id==imported));
    assert_eq!(import().await.unwrap(), first);
    let mut changed = grant.approval.clone();
    changed.maximum_creations = 4;
    assert!(approve(&changed).await.is_err());
    let claim = async |ordinal| {
        let reply = pic
            .update_call(
                root.id,
                fleet.pool,
                AssignWorker::NAME,
                AssignWorker::encode_request(&ordinal).unwrap(),
            )
            .await
            .unwrap();
        AssignWorker::decode_reply(&reply).unwrap()
    };
    assert!(matches!(claim(0).await, Placement::Pending(_)));
    settle(pic).await;
    let rows = inspect().await;
    assert_eq!(
        rows.iter()
            .filter(|(_, a)| matches!(a, Asset::Empty(_)))
            .count(),
        2
    );
    assert!(matches!(rows.last(),Some((Source::Automatic(a),Asset::Empty(_))) if a.sequence==2));
    assert!(matches!(claim(1).await, Placement::Pending(_)));
    settle(pic).await;
    approve(&grant.approval).await.unwrap();
    assert_eq!(inspect().await.len(), 4);
    let reopen = async || {
        canic_qualification::upgrade_stopped(
            pic,
            root.id,
            std::fs::read(base.join("root.wasm")).unwrap(),
            vec![],
            Some(root.operator),
        )
        .await
        .unwrap()
    };
    reopen().await;
    assert_eq!(import().await.unwrap(), first);
    pic.advance_time(Duration::from_secs(7200)).await;
    assert!(matches!(import().await.unwrap(),Asset::Available(id,_) if id==imported));
    reopen().await;
    approve(&grant.approval).await.unwrap();
    assert!(
        matches!(inspect().await.first(),Some((Source::Imported(id),Asset::Available(asset,_))) if *id==imported && id==asset)
    );
    assert!(matches!(claim(2).await, Placement::Pending(_)));
    settle(pic).await;
    assert!(matches!(claim(2).await,Placement::Assigned(_,id) if id==spare.to_text()));
    // Additional unassigned surplus can arrive after the spending deadline.
    let extra = pic.create_canister_with_settings(Some(root.id), None).await;
    pic.add_cycles(extra, 1_000_000_000_000).await;
    assert_ne!(
        pic.get_subnet(root.id).await.unwrap(),
        pic.get_subnet(foreign).await.unwrap()
    );
    pic.set_controllers(foreign, None, vec![root.id])
        .await
        .unwrap();
    pic.add_cycles(foreign, 1_000_000_000_000).await;
    let mut extended = grant.approval.clone();
    extended.imports.push(stored::StandbyImport {
        principal: extra.to_text(),
        previous_root: None,
    });
    extended.imports.push(stored::StandbyImport {
        principal: foreign.to_text(),
        previous_root: None,
    });
    approve(&extended).await.unwrap();
    approve(&grant.approval).await.unwrap();
    let extra_id: <ImportStandby as Contract>::Input = extra.to_text().parse().unwrap();
    let before_release = inspect().await;
    assert_eq!(
        former_release(extra_id).await.unwrap_err(),
        "wrong former Root"
    );
    assert_eq!(inspect().await, before_release);
    let observe_extra = async || {
        root.update::<_, Result<Asset, String>>(
            ImportStandby::NAME,
            ImportStandby::ID,
            &extra_id,
            ImportStandby::REPLY,
        )
        .await
    };
    let quarantined = observe_extra().await.unwrap();
    assert!(matches!(quarantined,Asset::Quarantined(id,_) if id==extra_id));
    pic.stop_canister(extra, Some(root.id)).await.unwrap();
    pic.advance_time(Duration::from_secs(7200)).await;
    let before_admission = inspect().await;
    assert_eq!(
        observe_extra().await.unwrap_err(),
        format!("cannot import {extra}: candidate must be running, observed Stopped")
    );
    assert_eq!(inspect().await, before_admission);
    pic.start_canister(extra, Some(root.id)).await.unwrap();
    let admitted = observe_extra().await.unwrap();
    assert!(matches!(admitted,Asset::Available(id,_) if id==extra_id));
    assert_eq!(observe_extra().await.unwrap(), admitted);
    reopen().await;
    let before = inspect().await;
    assert!(
        before
            .iter()
            .any(|(_, asset)| *asset == Asset::AwaitingHandoff(foreign_id))
    );
    for _ in 0..2 {
        let error = root
            .update::<_, Result<Asset, String>>(
                ImportStandby::NAME,
                ImportStandby::ID,
                &foreign_id,
                ImportStandby::REPLY,
            )
            .await
            .unwrap_err();
        assert!(
            error.contains(&foreign.to_text()) && error.contains("but Root requires"),
            "{error}"
        );
        assert_eq!(inspect().await, before);
    }
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert_eq!(server.stop().unwrap().abandoned, 0);
}

#[tokio::test(flavor = "current_thread")]
async fn two_roots_hold_spare_until_destination_confirms_custody() {
    use canic::sharding::standby::{ExportStandby, Refill, RefillStandby, TransferPhase};
    let base =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/examples/miner-pool/artifacts");
    let wasm = std::fs::read(base.join("root.wasm")).unwrap();
    let (mut server, mut owner) = support::instance_with_lifetime(
        &canic_qualification::server_directory(),
        ic_testkit::pocket_ic::PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet(),
        Duration::from_secs(180),
    )
    .await;
    let pic = owner.client();
    pic.set_time((std::time::SystemTime::now() - Duration::from_secs(7260)).into())
        .await;
    let registry = CanisterId::from_text("rwlgt-iiaaa-aaaaa-aaaaa-cai").unwrap();
    pic.create_canister_with_id(None, None, registry)
        .await
        .unwrap();
    pic.add_cycles(registry, 1_000_000_000_000).await;
    pic.install_canister(
        registry,
        wat::parse_str(include_str!("registry.wat")).unwrap(),
        vec![0],
        None,
    )
    .await;
    let root = async |label: &[u8]| {
        let operator = CanisterId::self_authenticating(label);
        let id = pic
            .create_canister_with_settings(Some(operator), None)
            .await;
        pic.add_cycles(id, 10_000_000_000_000).await;
        pic.install_canister(id, wasm.clone(), vec![], Some(operator))
            .await;
        let client = Client { pic, id, operator };
        client
            .update::<_, Result<(), management::Refusal>>(
                "configure_capacity",
                stored::CONFIGURE_CAPACITY,
                &stored::SubnetCapacity {
                    root: id.to_text(),
                    max_children: 1,
                },
                stored::LIMITS,
            )
            .await
            .unwrap();
        client
    };
    let source = root(b"source").await;
    let destination = root(b"destination").await;
    let deadline = pic.get_time().await.as_nanos_since_unix_epoch() + 600_000_000_000;
    let refill = async |sequence| {
        source
            .update::<_, Result<Asset, String>>(
                RefillStandby::NAME,
                RefillStandby::ID,
                &Refill {
                    root: source.id.to_text().parse().unwrap(),
                    sequence,
                    cycles: management::CREATION_CYCLES,
                    expires_at_ns: deadline,
                },
                RefillStandby::REPLY,
            )
            .await
    };
    let Asset::Empty(spare) = refill(0).await.unwrap() else {
        panic!("empty source spare")
    };
    let approval: <ExportStandby as Contract>::Input =
        (spare, destination.id.to_text().parse().unwrap(), deadline);
    let export = async |input| {
        source
            .update::<_, Result<TransferPhase, String>>(
                ExportStandby::NAME,
                ExportStandby::ID,
                &input,
                ExportStandby::REPLY,
            )
            .await
    };
    assert!(export(approval).await.is_err());
    destination
        .update::<_, Result<(), String>>(
            "configure_standby",
            stored::CONFIGURE_STANDBY,
            &stored::StandbyPolicy {
                root: destination.id.to_text(),
                minimum_available: 0,
                maximum_creations: 0,
                expires_at_ns: deadline,
                minimum_import_cycles: 1_000_000u64.try_into().unwrap(),
                imports: vec![stored::StandbyImport {
                    principal: spare.to_text(),
                    previous_root: Some(source.id.to_text()),
                }],
            },
            stored::LIMITS,
        )
        .await
        .unwrap();
    assert!(
        pic.update_call(
            source.id,
            destination.operator,
            ExportStandby::NAME,
            ExportStandby::encode_request(&approval).unwrap()
        )
        .await
        .is_err()
    );
    // Discard the original operator reply and recover through the same approval.
    let _lost_reply = pic
        .submit_call(
            source.id,
            source.operator,
            ExportStandby::NAME,
            ExportStandby::encode_request(&approval).unwrap(),
        )
        .await
        .unwrap();
    settle(pic).await;
    let upgrade = async |r: &Client<'_>| {
        canic_qualification::upgrade_stopped(pic, r.id, wasm.clone(), vec![], Some(r.operator))
            .await
            .unwrap()
    };
    upgrade(&source).await;
    assert_eq!(
        export(approval).await.unwrap(),
        TransferPhase::ControllersIssued
    );
    assert!(refill(1).await.is_err());
    assert!(export((spare, approval.1, deadline - 1)).await.is_err());
    let target = CanisterId::from_text(spare.to_text()).unwrap();
    assert_eq!(
        pic.canister_status(target, Some(destination.id))
            .await
            .unwrap()
            .settings
            .controllers,
        vec![destination.id]
    );
    let import = async || {
        destination
            .update::<_, Result<Asset, String>>(
                ImportStandby::NAME,
                ImportStandby::ID,
                &spare,
                ImportStandby::REPLY,
            )
            .await
            .unwrap()
    };
    assert!(matches!(import().await,Asset::Quarantined(p,_) if p==spare));
    upgrade(&destination).await;
    assert_eq!(export(approval).await.unwrap(), TransferPhase::Confirmed);
    assert!(matches!(refill(1).await.unwrap(),Asset::Empty(p) if p!=spare));
    upgrade(&source).await;
    assert_eq!(export(approval).await.unwrap(), TransferPhase::Confirmed);
    pic.advance_time(Duration::from_secs(7200)).await;
    assert!(matches!(import().await,Asset::Available(p,_) if p==spare));
    let destination_id = destination.id.to_text();
    let root_key = pic
        .root_key()
        .await
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let root_subnet = pic.topology().await.get_nns().unwrap();
    let gateway = owner.client_mut().make_live(None).await;
    let directory = artifacts().join(format!("spare-report-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let config = directory.join("canic.toml");
    std::fs::write(&config,format!(r#"
[apps.demo.canisters.app]
[apps.demo.fleets.local]
network = {{ kind="Local", url="{gateway}", root_key="{root_key}", root_subnet="{root_subnet}", routing_canister="{destination_id}" }}
subnet_child_limit = 1
[apps.demo.fleets.local.canisters.root]
id = "{destination_id}"
[apps.demo.fleets.local.standby_pool]
root = "root"
min_available_spares = 0
maximum_creations = 0
expires_at_ns = {deadline}
minimum_import_cycles = 1000000
"#)).unwrap();
    let cli = std::env::var_os("CANIC_CLI").expect("make miner-test supplies the built CLI");
    let result = std::process::Command::new(cli)
        .args(["fleet", "show", "--config"])
        .arg(&config)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let output = String::from_utf8(result.stdout).unwrap();
    assert!(
        output.contains("Min available spares: 0 (subnet fleet) · 1 ready · target met"),
        "{output}"
    );
    owner.client_mut().stop_live().await;
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert_eq!(server.stop().unwrap().abandoned, 0);
}
