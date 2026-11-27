//! Real CLI approval and Root-to-catalog installation, using the existing Store owner.
use super::{Client, actor, app, directory, frame, initialization};
use ic_testkit::{
    pocket_ic::{CanisterId, PocketIcBuilder},
    server::{Limits, PreparedServer, Server},
};
use serde_json::{Value, json};
use std::{
    path::Path,
    process::{Command, Output},
    time::Duration,
};

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(std::env::var_os("CANIC_CLI").expect("Blob runner builds the CLI"))
        .args(args)
        .args(["--json", "--config"])
        .arg(dir.join("canic.toml"))
        .output()
        .unwrap()
}

fn cli(dir: &Path, args: &[&str]) -> Value {
    let output = run(dir, args);
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[tokio::test(flavor = "current_thread")]
async fn approved_catalog_creation_retains_exact_arguments_and_actual_root_authority() {
    let assets = directory();
    let binary = PreparedServer::check(&canic_qualification::server_directory()).unwrap();
    let mut server = Server::start(
        binary,
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(300),
            stdout_bytes: 1024 * 1024,
            stderr_bytes: 1024 * 1024,
        },
    )
    .unwrap();
    let mut owner = server
        .instance(
            PocketIcBuilder::new()
                .with_nns_subnet()
                .with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    let root_bytes = std::fs::read(assets.join("root.wasm")).unwrap();
    let root_hash = hex(&canic_management_protocol::digest(&root_bytes));
    let store_hash = hex(&canic_management_protocol::digest(
        &std::fs::read(assets.join("store.wasm")).unwrap(),
    ));
    let routing = owner.client().create_canister().await;
    let url = owner.client_mut().make_live(None).await.to_string();
    let root_key = hex(&owner.client().root_key().await.unwrap());
    let dir = assets.join(format!("fleet-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let config = json!({"apps":{"assets":{
        "canisters":{"root":{"role":"root","wasm":assets.join("root.wasm")}, "store":{"role":"wasm-store","parent":"root","wasm":assets.join("store.wasm")},
            "catalog":{"parent":"root","wasm":assets.join("original.wasm")}},
        "fleets":{"local":{"network":{"kind":"Local","url":url,"root_key":root_key,"root_subnet":owner.client().topology().await.get_nns().unwrap().to_text(),"routing_canister":routing.to_text()},
            "journal":"journal", "canisters":{}}}
    }}});
    std::fs::write(dir.join("canic.toml"), toml::to_string(&config).unwrap()).unwrap();
    let signer = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join(".tools/ic/bin/icp");
    let apply = |id: &str| {
        cli(
            &dir,
            &[
                "fleet",
                "apply",
                id,
                "--identity",
                "anonymous",
                "--icp-bin",
                signer.to_str().unwrap(),
            ],
        )
    };
    let bootstrap = cli(
        &dir,
        &[
            "fleet",
            "plan",
            "--canister",
            "root",
            "--controller",
            "2vxsx-fae",
            "--cycles",
            "5000000000000",
        ],
    );
    let bootstrap = apply(bootstrap["operation"].as_str().unwrap());
    let root = CanisterId::from_text(bootstrap["canister"].as_str().unwrap()).unwrap();
    assert_eq!(bootstrap["module_hash"], root_hash);
    cli(
        &dir,
        &["fleet", "canister", "bind", "root", "--id", &root.to_text()],
    );
    let store = cli(
        &dir,
        &[
            "fleet",
            "plan",
            "--canister",
            "store",
            "--root-module",
            &root_hash,
            "--controller",
            "2vxsx-fae",
        ],
    );
    let store = apply(store["operation"].as_str().unwrap());
    cli(
        &dir,
        &[
            "fleet",
            "canister",
            "bind",
            "store",
            "--id",
            store["canister"].as_str().unwrap(),
        ],
    );
    let release = cli(
        &dir,
        &[
            "fleet",
            "plan",
            "--canister",
            "catalog",
            "--publish-version",
            "v1",
            "--store",
            "store",
            "--root-module",
            &root_hash,
            "--store-module",
            &store_hash,
            "--controller",
            "2vxsx-fae",
        ],
    );
    let release = release["operation"].as_str().unwrap();
    apply(release);
    let mut installation = initialization();
    installation.manager = root.as_slice().try_into().unwrap();
    let args = frame(app::INIT, &installation);
    let args_path = dir.join("catalog-init.bin");
    std::fs::write(&args_path, &args).unwrap();
    let planned = cli(
        &dir,
        &[
            "fleet",
            "plan",
            "--canister",
            "catalog",
            "--root-module",
            &root_hash,
            "--release",
            release,
            "--install-args",
            args_path.to_str().unwrap(),
            "--controller",
            "2vxsx-fae",
        ],
    );
    let id = planned["operation"].as_str().unwrap();
    assert_eq!(planned["plan"]["version"], 1);
    let original_plan = std::fs::read(dir.join("journal").join(id).join("plan.json")).unwrap();
    // Apply consumes the approved record, never rereads this now-corrupt input file.
    std::fs::write(&args_path, [255]).unwrap();
    assert!(
        !run(
            &dir,
            &[
                "fleet",
                "plan",
                "--canister",
                "catalog",
                "--root-module",
                &root_hash,
                "--release",
                release,
                "--install-args",
                args_path.to_str().unwrap(),
                "--controller",
                "2vxsx-fae"
            ]
        )
        .status
        .success()
    );
    assert!(
        !dir.join("journal")
            .join(id)
            .join("stage/signed.json")
            .exists()
    );
    let mut receipt = apply(id);
    std::fs::remove_file(dir.join("journal").join(id).join("stage/reply.bin")).unwrap();
    for _ in 0..30 {
        if receipt["canister"].is_string() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
        receipt = apply(id);
    }
    assert!(receipt["canister"].is_string(), "{receipt}");
    let child = CanisterId::from_text(receipt["canister"].as_str().unwrap()).unwrap();
    assert_eq!(receipt["controller"], root.to_text());
    assert_eq!(apply(id), receipt);
    assert_eq!(
        std::fs::read(dir.join("journal").join(id).join("plan.json")).unwrap(),
        original_plan
    );
    let client = Client::current(owner.client(), child);
    let draft = client.draft(1).await;
    let entry = client.authorize(actor(1), &draft).await.unwrap();
    assert_eq!(entry.request.uploader.as_slice(), actor(1).as_slice());
    assert!(
        client
            .authorize(CanisterId::anonymous(), &client.draft(2).await)
            .await
            .is_err()
    );
    let status = owner
        .client()
        .canister_status(child, Some(root))
        .await
        .unwrap();
    assert_eq!(status.settings.controllers, vec![root]);
    use canic_management_protocol::{application, children::Request, stored};
    let operation = dir.join("journal").join(id);
    let request: Request<canic_management_protocol::application::Creation> = ic_wire::decode(
        &std::fs::read(operation.join("stage/arguments.bin")).unwrap(),
        ic_wire::Encoding::Frame {
            method: application::STAGE_CHILD_CREATION,
            direction: ic_wire::Direction::Request,
        },
        application::LIMITS,
    )
    .unwrap();
    let canic_management_protocol::application::Deployment::Application(value) = request.value
    else {
        panic!("expected Application input")
    };
    let request = canic_management_protocol::children::Request {
        slot: request.slot,
        value,
    };
    assert_eq!(request.value.arguments.as_slice(), args);
    let custody_bytes = owner
        .client()
        .update_call(
            root,
            CanisterId::anonymous(),
            "observe_root_custody",
            frame(
                application::CUSTODY,
                &None::<canic_management_protocol::application::DeploymentSelector>,
            ),
        )
        .await
        .unwrap();
    let custody: application::Custody = ic_wire::decode(
        &custody_bytes,
        ic_wire::Encoding::Frame {
            method: application::CUSTODY,
            direction: ic_wire::Direction::Response,
        },
        application::LIMITS,
    )
    .unwrap();
    let position = request.slot.service_index().unwrap();
    assert_eq!(
        custody.children[position]
            .lifecycle
            .as_ref()
            .unwrap()
            .creation
            .as_slice(),
        args
    );
    assert_eq!(custody.children[position].journal.creation.attempts, [1; 3]);
    let sources = custody.children[position].sources.as_ref().unwrap();
    assert_eq!(
        sources.creation.pin(),
        request.value.pin(application::STAGE, request.slot)
    );
    assert_eq!(sources.creation.state, stored::PinState::Retained);
    assert!(
        owner
            .client()
            .update_call(
                root,
                CanisterId::anonymous(),
                "observe_stored_children",
                frame(stored::CUSTODY, &())
            )
            .await
            .is_err()
    );
    let before = owner.client().get_stable_memory(root).await;
    let mut changed = request.clone();
    changed.value.arguments = application::Arguments::new(vec![255]).unwrap();
    let bytes = owner
        .client()
        .update_call(
            root,
            CanisterId::anonymous(),
            "stage_child_creation",
            frame(
                canic_management_protocol::application::STAGE_CHILD_CREATION,
                &canic_management_protocol::children::Request {
                    slot: changed.slot,
                    value: canic_management_protocol::application::Deployment::<(), _>::Application(
                        &changed.value,
                    ),
                },
            ),
        )
        .await
        .unwrap();
    let refused: Result<canic_management_protocol::Status, canic_management_protocol::Refusal> =
        ic_wire::decode(
            &bytes,
            ic_wire::Encoding::Frame {
                method: application::STAGE_CHILD_CREATION,
                direction: ic_wire::Direction::Response,
            },
            application::LIMITS,
        )
        .unwrap();
    assert!(refused.is_err());
    assert_eq!(owner.client().get_stable_memory(root).await, before);
    // Lost Host evidence is reconstructed under the same signed stage, without a new child.
    let signed = std::fs::read(operation.join("stage/signed.json")).unwrap();
    assert_eq!(apply(id), receipt);
    assert_eq!(
        std::fs::read(operation.join("stage/signed.json")).unwrap(),
        signed
    );
    assert_eq!(owner.client().get_stable_memory(root).await, before);
    // Populate two application owners sharing the real supplier object before maintenance.
    use super::blob::supplier::protocol::Command as BlobCommand;
    client
        .command(actor(1), BlobCommand::Register(draft.root))
        .await
        .unwrap();
    client.publish(actor(1), 1).await.unwrap();
    client
        .authorize(actor(2), &client.draft(2).await)
        .await
        .unwrap();
    client.publish(actor(2), 2).await.unwrap();
    let alice = client.asset(actor(1), 1).await.unwrap();
    let bob = client.asset(actor(2), 2).await.unwrap();
    let mut unresolved = client.draft(3).await;
    unresolved.root = [43; 32];
    unresolved.chunks = vec![[43; 32]];
    client.authorize(actor(1), &unresolved).await.unwrap();
    let unresolved = client.asset(actor(1), 3).await.unwrap();
    let candidate = assets.join("candidate.wasm");
    let mut config: Value =
        toml::from_str(&std::fs::read_to_string(dir.join("canic.toml")).unwrap()).unwrap();
    config["apps"]["assets"]["canisters"]["catalog"]["wasm"] = json!(candidate);
    std::fs::write(dir.join("canic.toml"), toml::to_string(&config).unwrap()).unwrap();
    let published = cli(
        &dir,
        &[
            "fleet",
            "plan",
            "--canister",
            "catalog",
            "--publish-version",
            "v2",
            "--store",
            "store",
            "--root-module",
            &root_hash,
            "--store-module",
            &store_hash,
            "--controller",
            "2vxsx-fae",
        ],
    );
    let release = published["operation"].as_str().unwrap();
    apply(release);
    std::fs::write(&args_path, frame(app::UPGRADE, &())).unwrap();
    let planned = cli(
        &dir,
        &[
            "fleet",
            "plan",
            "--canister",
            "catalog",
            "--root-module",
            &root_hash,
            "--release",
            release,
            "--install-args",
            args_path.to_str().unwrap(),
            "--blob-attempt",
            "1",
            "--target",
            &child.to_text(),
            "--previous-module",
            receipt["module_hash"].as_str().unwrap(),
            "--controller",
            "2vxsx-fae",
        ],
    );
    let upgrade = planned["operation"].as_str().unwrap();
    set_fault(owner.client(), root, request.slot, 20).await;
    let mut progress = apply(upgrade);
    // Lose each asynchronous callback separately. Reopening Root and new CLI processes
    // must recover original outcomes, never issue replacement effects.
    for (point, phase, next) in [
        (20, application::MaintenancePhase::DrainIssued, 21),
        (21, application::MaintenancePhase::StopIssued, 8),
        (8, application::MaintenancePhase::Stopped, 22),
        (22, application::MaintenancePhase::StartIssued, 0),
    ] {
        let mut observed = read_custody(owner.client(), root).await;
        for _ in 0..60 {
            let state = observed.children[position]
                .lifecycle
                .as_ref()
                .unwrap()
                .upgrades
                .last()
                .unwrap();
            let installed = observed.children[position]
                .journal
                .upgrades
                .last()
                .unwrap()
                .phase;
            if state.phase == phase
                && (point != 8
                    || installed == canic_management_protocol::upgrade::Phase::InstallIssued)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
            observed = read_custody(owner.client(), root).await;
        }
        assert_eq!(
            observed.children[position]
                .lifecycle
                .as_ref()
                .unwrap()
                .upgrades
                .last()
                .unwrap()
                .phase,
            phase,
            "boundary {point}"
        );
        let stable = owner.client().get_stable_memory(root).await;
        canic_qualification::upgrade_stopped(
            owner.client(),
            root,
            std::fs::read(assets.join("root.wasm")).unwrap(),
            vec![],
            None,
        )
        .await
        .unwrap();
        assert_eq!(owner.client().get_stable_memory(root).await, stable);
        assert_eq!(read_custody(owner.client(), root).await, observed);
        set_fault(owner.client(), root, request.slot, next).await;
        progress = cli(
            &dir,
            &[
                "fleet",
                "reconcile",
                upgrade,
                "--identity",
                "anonymous",
                "--icp-bin",
                signer.to_str().unwrap(),
            ],
        );
    }
    let mut retained = read_custody(owner.client(), root).await;
    for _ in 0..25 {
        if retained.children[position]
            .lifecycle
            .as_ref()
            .unwrap()
            .upgrades
            .last()
            .is_some_and(|m| m.phase == application::MaintenancePhase::Inspected)
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
        progress = apply(upgrade);
        retained = read_custody(owner.client(), root).await;
    }
    let maintenance = retained.children[position]
        .lifecycle
        .as_ref()
        .unwrap()
        .upgrades
        .last()
        .unwrap();
    assert_eq!(
        maintenance.phase,
        application::MaintenancePhase::Inspected,
        "{progress}"
    );
    assert_eq!(maintenance.before, maintenance.after);
    assert_eq!(client.asset(actor(1), 3).await.unwrap(), unresolved);
    assert!(maintenance.requests[..3].iter().all(Option::is_some));
    assert!(maintenance.requests[3].is_none());
    assert!(!client.ready().await);
    assert_eq!(client.asset(actor(1), 1).await.unwrap(), alice);
    assert_eq!(client.asset(actor(2), 2).await.unwrap(), bob);
    let upgrade_request: Request<canic_management_protocol::application::Replacement> =
        ic_wire::decode(
            &std::fs::read(
                dir.join("journal")
                    .join(upgrade)
                    .join("stage/arguments.bin"),
            )
            .unwrap(),
            ic_wire::Encoding::Frame {
                method: application::STAGE_CHILD_UPGRADE,
                direction: ic_wire::Direction::Request,
            },
            application::LIMITS,
        )
        .unwrap();
    let canic_management_protocol::application::Deployment::Application(value) =
        upgrade_request.value
    else {
        panic!("expected Application input")
    };
    let upgrade_request = canic_management_protocol::children::Request {
        slot: upgrade_request.slot,
        value,
    };
    let stable = owner.client().get_stable_memory(root).await;
    let mut changed = upgrade_request.clone();
    changed.value.attempt = 2;
    assert!(
        owner
            .client()
            .update_call(
                root,
                CanisterId::anonymous(),
                "activate_child_upgrade",
                frame(application::ACTIVATE, &changed)
            )
            .await
            .is_err()
    );
    let mut malformed = frame(application::ACTIVATE, &upgrade_request);
    malformed.pop();
    assert!(
        owner
            .client()
            .update_call(
                root,
                CanisterId::anonymous(),
                "activate_child_upgrade",
                malformed
            )
            .await
            .is_err()
    );
    owner
        .client()
        .update_canister_settings(
            root,
            None,
            ic_testkit::pocket_ic::CanisterSettings {
                controllers: Some(vec![CanisterId::anonymous(), actor(9)]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(
        owner
            .client()
            .update_call(
                root,
                actor(9),
                "activate_child_upgrade",
                frame(application::ACTIVATE, &upgrade_request)
            )
            .await
            .is_err()
    );
    assert!(
        !run(
            &dir,
            &[
                "fleet",
                "activate",
                upgrade,
                "--identity",
                "anonymous",
                "--icp-bin",
                signer.to_str().unwrap()
            ]
        )
        .status
        .success()
    );
    assert!(
        !dir.join("journal")
            .join(upgrade)
            .join("activate/signed.json")
            .exists()
    );
    owner
        .client()
        .update_canister_settings(
            root,
            None,
            ic_testkit::pocket_ic::CanisterSettings {
                controllers: Some(vec![CanisterId::anonymous()]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(owner.client().get_stable_memory(root).await, stable);
    // External controller and ordinary users are not the child's Blob manager.
    assert!(
        client
            .command(CanisterId::anonymous(), BlobCommand::Activate(1))
            .await
            .is_err()
    );
    assert!(
        client
            .command(actor(1), BlobCommand::Activate(1))
            .await
            .is_err()
    );
    let activate = || {
        cli(
            &dir,
            &[
                "fleet",
                "activate",
                upgrade,
                "--identity",
                "anonymous",
                "--icp-bin",
                signer.to_str().unwrap(),
            ],
        )
    };
    set_fault(owner.client(), root, request.slot, 23).await;
    let activation = run(
        &dir,
        &[
            "fleet",
            "activate",
            upgrade,
            "--identity",
            "anonymous",
            "--icp-bin",
            signer.to_str().unwrap(),
        ],
    );
    assert!(
        !activation.status.success(),
        "lost activation callback must not fabricate a reply"
    );
    let before_recovery = read_custody(owner.client(), root).await;
    assert_eq!(
        before_recovery.children[position]
            .lifecycle
            .as_ref()
            .unwrap()
            .upgrades
            .last()
            .unwrap()
            .phase,
        application::MaintenancePhase::ActivateIssued
    );
    assert!(
        client.ready().await,
        "the earlier Blob effect committed despite Root's callback trap"
    );
    set_fault(owner.client(), root, request.slot, 0).await;
    cli(
        &dir,
        &[
            "fleet",
            "reconcile",
            upgrade,
            "--identity",
            "anonymous",
            "--icp-bin",
            signer.to_str().unwrap(),
        ],
    );
    let mut complete = apply(upgrade);
    for _ in 0..15 {
        if complete["canister"].is_string() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
        complete = apply(upgrade);
    }
    assert_eq!(complete["canister"], child.to_text(), "{complete}");
    assert!(client.ready().await);
    client.remove(actor(1), 1).await.unwrap();
    let surviving = client.asset(actor(2), 2).await.unwrap();
    assert_eq!(surviving.entry, bob.entry);
    assert_eq!(surviving.reference, bob.reference);
    assert_eq!(surviving.object.live_references, 1);
    assert!(client.publish(actor(1), 1).await.is_err());
    let after = owner.client().get_stable_memory(child).await;
    assert_eq!(activate(), complete);
    assert_eq!(owner.client().get_stable_memory(child).await, after);
    // The controller explicitly admits a second Root upgrade using the already
    // retained original release. Old activation cannot reopen this newer drain.
    let mut next = upgrade_request.clone();
    next.value.attempt = 2;
    next.value.source.source.reference = request.value.source.reference.clone();
    let approval = &mut next.value.source.source.approval;
    approval.previous_module = approval.module_hash;
    approval.module_hash = request.value.source.approval.module_hash;
    approval.execute_at_ns = owner.client().get_time().await.as_nanos_since_unix_epoch();
    approval.expires_at_ns = approval.execute_at_ns + 600_000_000_000;
    set_fault(owner.client(), root, request.slot, 20).await;
    let reply = owner
        .client()
        .update_call(
            root,
            CanisterId::anonymous(),
            "stage_child_upgrade",
            frame(
                canic_management_protocol::application::STAGE_CHILD_UPGRADE,
                &canic_management_protocol::children::Request {
                    slot: next.slot,
                    value: canic_management_protocol::application::Deployment::<(), _>::Application(
                        &next.value,
                    ),
                },
            ),
        )
        .await
        .unwrap();
    let staged: Result<
        canic_management_protocol::upgrade::Status,
        canic_management_protocol::Refusal,
    > = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: application::STAGE_CHILD_UPGRADE,
            direction: ic_wire::Direction::Response,
        },
        application::LIMITS,
    )
    .unwrap();
    staged.unwrap();
    for _ in 0..60 {
        if !client.ready().await {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(!client.ready().await);
    assert!(
        owner
            .client()
            .update_call(
                root,
                CanisterId::anonymous(),
                "activate_child_upgrade",
                frame(application::ACTIVATE, &upgrade_request)
            )
            .await
            .is_err()
    );
    assert!(!client.ready().await);
    assert_eq!(client.asset(actor(2), 2).await.unwrap(), surviving);
    // Recover the second drain, then lose installation evidence. Expiry before
    // recovery may establish installation, but must not authorize a fresh start.
    canic_qualification::upgrade_stopped(
        owner.client(),
        root,
        std::fs::read(assets.join("root.wasm")).unwrap(),
        vec![],
        None,
    )
    .await
    .unwrap();
    set_fault(owner.client(), root, request.slot, 8).await;
    owner
        .client()
        .update_call(
            root,
            CanisterId::anonymous(),
            "reconcile_child_upgrade",
            frame(
                canic_management_protocol::application::RECONCILE_CHILD_UPGRADE,
                &canic_management_protocol::children::Request {
                    slot: next.slot,
                    value: canic_management_protocol::application::Deployment::<(), _>::Application(
                        &next.value,
                    ),
                },
            ),
        )
        .await
        .unwrap();
    let mut pending = read_custody(owner.client(), root).await;
    for _ in 0..60 {
        if pending.children[position]
            .journal
            .upgrades
            .last()
            .unwrap()
            .phase
            == canic_management_protocol::upgrade::Phase::InstallIssued
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        pending = read_custody(owner.client(), root).await;
    }
    assert_eq!(
        pending.children[position]
            .journal
            .upgrades
            .last()
            .unwrap()
            .phase,
        canic_management_protocol::upgrade::Phase::InstallIssued
    );
    let stopped_bytes = owner.client().get_stable_memory(child).await;
    owner.client().stop_progress().await;
    owner.client().advance_time(Duration::from_secs(610)).await;
    canic_qualification::upgrade_stopped(
        owner.client(),
        root,
        std::fs::read(assets.join("root.wasm")).unwrap(),
        vec![],
        None,
    )
    .await
    .unwrap();
    owner
        .client()
        .update_call(
            root,
            CanisterId::anonymous(),
            "reconcile_child_upgrade",
            frame(
                canic_management_protocol::application::RECONCILE_CHILD_UPGRADE,
                &canic_management_protocol::children::Request {
                    slot: next.slot,
                    value: canic_management_protocol::application::Deployment::<(), _>::Application(
                        &next.value,
                    ),
                },
            ),
        )
        .await
        .unwrap();
    for _ in 0..30 {
        owner.client().tick().await;
    }
    let expired = read_custody(owner.client(), root).await;
    let lifecycle = expired.children[position]
        .lifecycle
        .as_ref()
        .unwrap()
        .upgrades
        .last()
        .unwrap();
    assert_eq!(lifecycle.phase, application::MaintenancePhase::Stopped);
    assert!(
        lifecycle.requests[2].is_none(),
        "expiry must prevent a new start"
    );
    let install = expired.children[position].journal.upgrades.last().unwrap();
    assert_eq!(
        install.phase,
        canic_management_protocol::upgrade::Phase::Verified
    );
    assert_eq!(install.attempts, [1, 1]);
    assert!(install.failure.is_some());
    assert_eq!(owner.client().get_stable_memory(child).await, stopped_bytes);
    let execution = owner
        .client()
        .canister_status(child, Some(root))
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(execution.status).unwrap(),
        json!("stopped")
    );
    owner.client_mut().stop_live().await;
    std::fs::remove_dir_all(dir).unwrap();
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

async fn set_fault(
    pic: &ic_testkit::pocket_ic::nonblocking::PocketIc,
    root: CanisterId,
    slot: canic_management_protocol::children::ChildId,
    value: u8,
) {
    use canic_management_protocol::children;
    pic.update_call(
        root,
        CanisterId::anonymous(),
        "inject_child_fault",
        frame(children::INJECT, &children::Request { slot, value }),
    )
    .await
    .unwrap();
}

async fn read_custody(
    pic: &ic_testkit::pocket_ic::nonblocking::PocketIc,
    root: CanisterId,
) -> canic_management_protocol::application::Custody {
    use canic_management_protocol::application;
    let bytes = pic
        .update_call(
            root,
            CanisterId::anonymous(),
            "observe_root_custody",
            frame(
                application::CUSTODY,
                &None::<canic_management_protocol::application::DeploymentSelector>,
            ),
        )
        .await
        .unwrap();
    ic_wire::decode(
        &bytes,
        ic_wire::Encoding::Frame {
            method: application::CUSTODY,
            direction: ic_wire::Direction::Response,
        },
        application::LIMITS,
    )
    .unwrap()
}
