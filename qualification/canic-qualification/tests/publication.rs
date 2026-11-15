//! Real CLI Store bootstrap and publication through Root with retained recovery.
mod store_upgrade_tests;
use ic_testkit::{
    pic::{PocketIcStartupConfig, server_assets},
    pocket_ic::{CanisterId, CanisterSettings, PocketIcBuilder},
};
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{Duration, Instant},
};

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(std::env::var_os("CANIC_CLI").expect("make publication-test"))
        .args(args)
        .args(["--json", "--config"])
        .arg(dir.join("canic.json"))
        .output()
        .unwrap()
}
fn cli(dir: &Path, args: &[&str]) -> Value {
    let output = run(dir, args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// Count retained completed Put requests, not network retransmissions or management calls.
fn publication_payload(dir: &Path, operation: &str) -> (usize, usize) {
    use canic_management_protocol::publication as protocol;
    let mut chunks = 0;
    let mut bytes = 0;
    for entry in std::fs::read_dir(dir.join("journal").join(operation).join("publication")).unwrap()
    {
        let entry = entry.unwrap();
        if !entry.file_name().to_str().unwrap().starts_with("chunk-") {
            continue;
        }
        let path = entry.path();
        assert!(path.join("signed.json").exists());
        assert!(path.join("reply.bin").exists());
        let request: protocol::Request = ic_wire::decode(
            &std::fs::read(path.join("arguments.bin")).unwrap(),
            ic_wire::Encoding::Frame {
                method: protocol::EXECUTE,
                direction: ic_wire::Direction::Request,
            },
            protocol::REQUEST_LIMITS,
        )
        .unwrap();
        let protocol::Command::Put(put) = request.command else {
            panic!("chunk journal must retain Put");
        };
        chunks += 1;
        bytes += put.bytes.len();
    }
    (chunks, bytes)
}
fn padded(mut bytes: Vec<u8>) -> Vec<u8> {
    // Valid custom section: more than one transfer, without changing executable behavior.
    let payload = 2_000_000_u32;
    bytes.push(0);
    let mut length = payload;
    loop {
        let byte = (length & 127) as u8;
        length >>= 7;
        bytes.push(byte | if length == 0 { 0 } else { 128 });
        if length == 0 {
            break;
        }
    }
    bytes.push(0);
    bytes.resize(bytes.len() + payload as usize - 1, 42);
    bytes
}

async fn maintain_root(
    dir: &Path,
    pic: &ic_testkit::pocket_ic::nonblocking::PocketIc,
    root: CanisterId,
    store: CanisterId,
    signer: &Path,
) {
    let planned = cli(dir, &["fleet", "plan", "--controller", "2vxsx-fae"]);
    assert_eq!(planned["action"], "maintenance", "{planned}");
    let id = planned["operation"].as_str().unwrap();
    let path = dir.join("journal").join(id);
    let sources = planned["plan"]["maintenance_store"]["sources"]
        .as_array()
        .unwrap();
    assert_eq!(sources.len(), 2);
    assert!(
        sources
            .iter()
            .all(|s| s["upgrades"].as_array().unwrap().len() == 1)
    );
    // Drift in supplier authority blocks maintenance before any installation signature.
    pic.update_canister_settings(
        store,
        Some(root),
        CanisterSettings {
            controllers: Some(vec![root, CanisterId::anonymous()]),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let apply = [
        "fleet",
        "apply",
        id,
        "--identity",
        "anonymous",
        "--icp-bin",
        signer.to_str().unwrap(),
    ];
    assert!(!run(dir, &apply).status.success());
    assert!(!path.join("install/signed.json").exists());
    pic.update_canister_settings(
        store,
        Some(root),
        CanisterSettings {
            controllers: Some(vec![root]),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let pause = dir.join("pause-maintenance");
    std::fs::write(&pause, "#!/bin/sh\nfor arg in \"$@\"; do\n case \"$arg\" in */custody-after/arguments.bin) exit 73;; esac\ndone\nexec \"$CANIC_REAL_SIGNER\" \"$@\"\n").unwrap();
    std::fs::set_permissions(&pause, std::fs::Permissions::from_mode(0o700)).unwrap();
    let interrupted = Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(["fleet", "apply", id, "--json", "--config"])
        .arg(dir.join("canic.json"))
        .args(["--identity", "anonymous", "--icp-bin"])
        .arg(&pause)
        .env("CANIC_REAL_SIGNER", signer)
        .output()
        .unwrap();
    assert!(!interrupted.status.success());
    assert!(
        path.join("install/reply.bin").exists(),
        "{}",
        String::from_utf8_lossy(&interrupted.stderr)
    );
    assert!(!path.join("receipt.json").exists());
    let signed = std::fs::read(path.join("install/signed.json")).unwrap();
    std::fs::remove_file(path.join("install/reply.bin")).unwrap();
    assert_eq!(
        cli(dir, &["fleet", "plan", "--controller", "2vxsx-fae"]),
        planned
    );
    let completed = cli(dir, &apply);
    assert_eq!(completed["canister"], root.to_text());
    assert_eq!(
        completed["module_hash"],
        planned["plan"]["artifact"]["sha256"]
    );
    assert_eq!(
        std::fs::read(path.join("install/signed.json")).unwrap(),
        signed
    );
    assert_eq!(
        std::fs::read(path.join("custody-before/reply.bin")).unwrap(),
        std::fs::read(path.join("custody-after/reply.bin")).unwrap()
    );
    assert_eq!(cli(dir, &apply), completed);
    assert_eq!(
        cli(dir, &["fleet", "plan", "--controller", "2vxsx-fae"])["action"],
        "unchanged"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn fleet_bootstrap_gates_both_application_slots_on_store_readiness() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_owned();
    let assets = PathBuf::from(std::env::var_os("CANIC_PUBLICATION_ARTIFACTS").unwrap());
    let mut owner = PocketIcStartupConfig::Spawn {
        server_binary: server_assets::check(&canic_qualification::server_directory()).unwrap(),
        timeout: Duration::from_secs(30),
        server_hard_ttl: Some(Duration::from_secs(300)),
        server_idle_ttl: None,
        server_output_files: None,
    }
    .build_instance(
        PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet(),
    )
    .await
    .unwrap();
    let routing = owner.instance().create_canister().await;
    let subnet = owner.instance().get_subnet(routing).await.unwrap();
    let url = owner.instance_mut().make_live(None).await.to_string();
    let root_key = hex(&owner.instance().root_key().await.unwrap());
    let dir = repo
        .join("target/publication")
        .join(format!("fleet-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let app_bytes = padded(std::fs::read(assets.join("app.wasm")).unwrap());
    std::fs::write(dir.join("app.wasm"), &app_bytes).unwrap();
    std::fs::write(dir.join("worker.wasm"), &app_bytes).unwrap();
    let mut config = json!({"apps":{"demo":{
        "canisters":{"root":{"role":"root","wasm":assets.join("root.wasm")},
            "store":{"role":"wasm-store","parent":"root","wasm":assets.join("store.wasm")},
            "app":{"role":"app","parent":"root","wasm":dir.join("app.wasm")},
            "worker":{"role":"worker","parent":"root","wasm":dir.join("worker.wasm")}},
        "fleets":{"local":{"network":{"kind":"Local","url":url,"root_key":root_key,"routing_canister":routing.to_text()},"journal":"journal","canisters":{}}}
    }}});
    std::fs::write(dir.join("canic.json"), serde_json::to_vec(&config).unwrap()).unwrap();
    let dispatch = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 60)
        .to_string();
    let planned = cli(
        &dir,
        &[
            "fleet",
            "plan",
            "--controller",
            "2vxsx-fae",
            "--execute-at",
            &dispatch,
        ],
    );
    let id = planned["operation"].as_str().unwrap();
    let path = dir.join("journal").join(id);
    assert_eq!(
        planned["fleet_plan"]["root"]["creation_cycles"],
        4_000_000_000_000_u64
    );
    assert_eq!(
        planned["fleet_plan"]["store"]["creation_cycles"],
        1_000_000_000_000_u64
    );
    let human = Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(["fleet", "status", id, "--config"])
        .arg(dir.join("canic.json"))
        .output()
        .unwrap();
    assert!(human.status.success());
    let text = String::from_utf8(human.stdout).unwrap();
    assert!(text.contains("store [WASM STORE]"));
    assert!(text.contains("Total operator debit ceiling: 5T"));
    assert!(text.contains("Store release: app /"));
    let signer = repo.join(".tools/ic/bin/icp");
    let paused = dir.join("pause-store-ready");
    std::fs::write(&paused, "#!/bin/sh\nfor arg in \"$@\"; do\n case \"$arg\" in */store/store-observe/0/arguments.bin) exit 71;; esac\ndone\nexec \"$CANIC_REAL_SIGNER\" \"$@\"\n").unwrap();
    std::fs::set_permissions(&paused, std::fs::Permissions::from_mode(0o700)).unwrap();
    let stopped = Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(["fleet", "apply", id, "--json", "--config"])
        .arg(dir.join("canic.json"))
        .args(["--identity", "anonymous", "--icp-bin"])
        .arg(&paused)
        .env("CANIC_REAL_SIGNER", &signer)
        .output()
        .unwrap();
    assert!(!stopped.status.success());
    assert!(
        path.join("store/handover/reply.bin").exists(),
        "{}",
        String::from_utf8_lossy(&stopped.stderr)
    );
    for leaf in ["child", "sibling"] {
        assert!(!path.join(leaf).join("plan.json").exists());
    }
    assert_eq!(
        owner
            .instance()
            .get_subnet_metrics(subnet)
            .await
            .unwrap()
            .num_canisters,
        3
    );
    assert!(!run(&dir, &["fleet", "cancel", id]).status.success());
    assert_eq!(
        cli(&dir, &["fleet", "plan", "--controller", "2vxsx-fae"])["operation"],
        id
    );
    let root = CanisterId::from_text(
        cli(&dir, &["fleet", "status", id])["root"]["receipt"]["canister"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let root_before = owner.instance().get_stable_memory(root).await;
    let signed = std::fs::read(path.join("store/handover/signed.json")).unwrap();
    std::fs::remove_file(path.join("store/handover/reply.bin")).unwrap();
    // Reconcile may complete Store, but cannot admit either application child.
    let reconciled = run(
        &dir,
        &[
            "fleet",
            "reconcile",
            id,
            "--identity",
            "anonymous",
            "--icp-bin",
            signer.to_str().unwrap(),
        ],
    );
    assert!(!reconciled.status.success());
    let state = cli(&dir, &["fleet", "status", id]);
    assert!(!state["store"]["state"]["receipt"].is_null());
    assert!(
        state["children"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["state"].is_null())
    );
    assert_eq!(owner.instance().get_stable_memory(root).await, root_before);
    assert_eq!(
        std::fs::read(path.join("store/handover/signed.json")).unwrap(),
        signed
    );
    // Pause the second publication after the first completed. Neither child may be admitted.
    std::fs::write(&paused, "#!/bin/sh\nfor arg in \"$@\"; do\n case \"$arg\" in */sibling/publication/publication/observe/1/arguments.bin) exit 72;; esac\ndone\nexec \"$CANIC_REAL_SIGNER\" \"$@\"\n").unwrap();
    let stopped = Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(["fleet", "apply", id, "--json", "--config"])
        .arg(dir.join("canic.json"))
        .args(["--identity", "anonymous", "--icp-bin"])
        .arg(&paused)
        .env("CANIC_REAL_SIGNER", &signer)
        .output()
        .unwrap();
    assert!(!stopped.status.success(), "pause must reach publication");
    assert!(
        path.join("child/publication/publication.json").exists(),
        "{}",
        String::from_utf8_lossy(&stopped.stderr)
    );
    assert!(!path.join("child/plan.json").exists());
    assert!(!path.join("sibling/plan.json").exists());
    let prepared = path.join("sibling/publication/publication/prepare");
    let signed_prepare = std::fs::read(prepared.join("signed.json")).unwrap();
    std::fs::remove_file(prepared.join("reply.bin")).unwrap();
    assert_eq!(
        cli(&dir, &["fleet", "plan", "--controller", "2vxsx-fae"])["operation"],
        id
    );
    assert!(
        !run(
            &dir,
            &[
                "fleet",
                "reconcile",
                id,
                "--identity",
                "anonymous",
                "--icp-bin",
                signer.to_str().unwrap()
            ]
        )
        .status
        .success()
    );
    assert!(!path.join("child/plan.json").exists());
    let apply = || {
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
    let admitted = apply();
    assert_eq!(
        std::fs::read(prepared.join("signed.json")).unwrap(),
        signed_prepare
    );
    assert!(
        !path
            .join("sibling/publication/publication/chunk-0")
            .exists(),
        "shared content must not upload twice"
    );
    let staged = path.join("child/stage");
    let signed_stage = std::fs::read(staged.join("signed.json")).unwrap();
    assert!(
        std::fs::metadata(staged.join("arguments.bin"))
            .unwrap()
            .len()
            < 8192
    );
    std::fs::remove_file(staged.join("reply.bin")).unwrap();
    apply();
    assert_eq!(
        std::fs::read(staged.join("signed.json")).unwrap(),
        signed_stage
    );
    assert_eq!(admitted["children"].as_array().unwrap().len(), 2);
    let store = CanisterId::from_text(admitted["store"]["canister"].as_str().unwrap()).unwrap();
    assert_eq!(owner.instance().get_subnet(store).await, Some(subnet));
    owner.instance().stop_progress().await;
    owner.instance().advance_time(Duration::from_secs(65)).await;
    for _ in 0..70 {
        owner.instance().tick().await;
    }
    owner.instance().auto_progress().await;
    let complete = apply();
    for child in complete["children"].as_array().unwrap() {
        assert_eq!(child["outcome"]["controller"], root.to_text(), "{complete}");
        assert!(child["outcome"]["canister"].is_string());
    }
    assert_eq!(
        owner
            .instance()
            .get_subnet_metrics(subnet)
            .await
            .unwrap()
            .num_canisters,
        5
    );
    assert_eq!(apply(), complete);
    assert_eq!(
        cli(&dir, &["fleet", "plan", "--controller", "2vxsx-fae"])["action"],
        "unchanged"
    );
    // Each changed child first gets publication authority, then an independently approved upgrade.
    let apply_operation = |operation: &str| {
        cli(
            &dir,
            &[
                "fleet",
                "apply",
                operation,
                "--identity",
                "anonymous",
                "--icp-bin",
                signer.to_str().unwrap(),
            ],
        )
    };
    let mut first_publication = String::new();
    for (index, (name, marker)) in [("app", 43), ("worker", 44), ("app", 45), ("app", 43)]
        .into_iter()
        .enumerate()
    {
        if index == 2 {
            config["apps"]["demo"]["canisters"]["root"]["wasm"] =
                json!(assets.join("root-maintenance.wasm"));
            std::fs::write(dir.join("canic.json"), serde_json::to_vec(&config).unwrap()).unwrap();
            maintain_root(&dir, owner.instance(), root, store, &signer).await;
        }
        let mut next = app_bytes.clone();
        *next.last_mut().unwrap() = marker;
        std::fs::write(dir.join(format!("{name}.wasm")), &next).unwrap();
        let dispatch = (owner
            .instance()
            .get_time()
            .await
            .as_nanos_since_unix_epoch()
            / 1_000_000_000
            + 30)
            .to_string();
        let plan = || {
            cli(
                &dir,
                &[
                    "fleet",
                    "plan",
                    "--controller",
                    "2vxsx-fae",
                    "--execute-at",
                    &dispatch,
                ],
            )
        };
        if index < 3 {
            let publication = plan();
            assert_eq!(publication["action"], "publication", "{publication}");
            let publication_id = publication["operation"].as_str().unwrap();
            if index == 0 {
                first_publication = publication_id.into();
            }
            assert_eq!(
                plan(),
                publication,
                "pending publication must retain its original approval"
            );
            assert!(
                !dir.join("journal")
                    .join(publication_id)
                    .join("receipt.json")
                    .exists()
            );
            let published = apply_operation(publication_id);
            assert!(
                published["publication"]["status"]["published"]
                    .as_bool()
                    .unwrap()
            );
        } else {
            // A release published before Root maintenance is reusable only with its original completion proof.
            let receipt = dir
                .join("journal")
                .join(&first_publication)
                .join("publication.json");
            let retained = std::fs::read(&receipt).unwrap();
            std::fs::remove_file(&receipt).unwrap();
            assert!(
                !run(
                    &dir,
                    &[
                        "fleet",
                        "plan",
                        "--controller",
                        "2vxsx-fae",
                        "--execute-at",
                        &dispatch
                    ]
                )
                .status
                .success()
            );
            std::fs::write(receipt, retained).unwrap();
        }
        let upgrade = plan();
        assert_eq!(upgrade["action"], "upgrade", "{upgrade}");
        let upgrade_id = upgrade["operation"].as_str().unwrap();
        assert_ne!(first_publication, upgrade_id);
        assert_eq!(
            plan(),
            upgrade,
            "unfinished upgrade must retain its original approval"
        );
        apply_operation(upgrade_id);
        owner
            .instance()
            .clear_chunk_store(store, Some(root))
            .await
            .unwrap();
        owner.instance().stop_progress().await;
        owner.instance().advance_time(Duration::from_secs(35)).await;
        for _ in 0..70 {
            owner.instance().tick().await;
        }
        owner.instance().auto_progress().await;
        let completed = apply_operation(upgrade_id);
        assert_eq!(
            completed["module_hash"],
            hex(&ic_wasm_store_protocol::digest(&next)),
            "{completed}"
        );
        assert_eq!(apply_operation(upgrade_id), completed);
        assert_eq!(plan()["action"], "unchanged");
    }
    let view = cli(&dir, &["fleet", "show"]);
    assert_eq!(view["canisters"].as_array().unwrap().len(), 4);
    assert!(view["issues"].as_array().unwrap().is_empty(), "{view}");
    assert_eq!(view["stores"].as_array().unwrap().len(), 1);
    assert_eq!(view["stores"][0]["root"], root.to_text());
    assert_eq!(view["stores"][0]["store"], store.to_text());
    assert!(view["stores"][0]["reported"].is_null());
    assert!(view["stores"][0]["observation_error"].is_null(), "{view}");
    // Readiness custody is part of fleet admission, not a disposable display summary.
    let proof = path.join("store/store-observe/0/reply.bin");
    let bytes = std::fs::read(&proof).unwrap();
    std::fs::remove_file(&proof).unwrap();
    assert!(
        !run(&dir, &["fleet", "plan", "--controller", "2vxsx-fae"])
            .status
            .success()
    );
    std::fs::write(proof, bytes).unwrap();
    observe_stopped_store(&dir, owner.instance(), root, store, &assets).await;
    owner.instance_mut().stop_live().await;
}

async fn observe_stopped_store(
    dir: &Path,
    pic: &ic_testkit::pocket_ic::nonblocking::PocketIc,
    root: CanisterId,
    store: CanisterId,
    assets: &Path,
) {
    use canic_management_protocol::store_maintenance as wire;
    let approval = wire::Approval {
        store: store.to_text(),
        previous_module: ic_wasm_store_protocol::digest(
            &std::fs::read(assets.join("store.wasm")).unwrap(),
        ),
        module_hash: [99; 32],
        attempt: ic_wasm_store_protocol::MaintenanceAttempt {
            sequence: 1,
            operation: [123; 32],
        },
        expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 3_600_000_000_000,
    };
    let before = pic.get_stable_memory(root).await;
    let refused = pic
        .update_call(
            root,
            CanisterId::anonymous(),
            "stage_store_maintenance",
            ic_wire::encode(
                &approval,
                ic_wire::Encoding::Frame {
                    method: wire::STAGE,
                    direction: ic_wire::Direction::Request,
                },
                wire::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        ic_wire::decode::<Result<wire::Status, canic_management_protocol::Refusal>>(
            &refused,
            ic_wire::Encoding::Frame {
                method: wire::STAGE,
                direction: ic_wire::Direction::Response
            },
            wire::LIMITS
        )
        .unwrap(),
        Err(canic_management_protocol::Refusal::Binding)
    );
    assert_eq!(pic.get_stable_memory(root).await, before);
    let operator = CanisterId::self_authenticating(b"Store report maintenance operator");
    pic.update_canister_settings(
        root,
        Some(CanisterId::anonymous()),
        CanisterSettings {
            controllers: Some(vec![operator]),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let response = pic
        .update_call(
            root,
            operator,
            "stage_store_maintenance",
            ic_wire::encode(
                &approval,
                ic_wire::Encoding::Frame {
                    method: wire::STAGE,
                    direction: ic_wire::Direction::Request,
                },
                wire::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode::<Result<wire::Status, canic_management_protocol::Refusal>>(
        &response,
        ic_wire::Encoding::Frame {
            method: wire::STAGE,
            direction: ic_wire::Direction::Response,
        },
        wire::LIMITS,
    )
    .unwrap()
    .unwrap();
    let mut stopped = false;
    for _ in 0..128 {
        let response = pic
            .update_call(
                root,
                operator,
                "advance_store_maintenance",
                ic_wire::encode(
                    &approval,
                    ic_wire::Encoding::Frame {
                        method: wire::ADVANCE,
                        direction: ic_wire::Direction::Request,
                    },
                    wire::LIMITS,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let status: wire::Status = ic_wire::decode(
            &response,
            ic_wire::Encoding::Frame {
                method: wire::ADVANCE,
                direction: ic_wire::Direction::Response,
            },
            wire::LIMITS,
        )
        .unwrap();
        if status.phase == wire::Phase::Stopped {
            stopped = true;
            break;
        }
    }
    assert!(stopped);
    let before_root = pic.get_stable_memory(root).await;
    let before_store = pic.get_stable_memory(store).await;
    for _ in 0..2 {
        let snapshot = cli(dir, &["fleet", "show"]);
        assert_eq!(snapshot["stores"].as_array().unwrap().len(), 1);
        let report = &snapshot["stores"][0];
        assert_eq!(report["reported"]["phase"], "Stopped", "{snapshot}");
        assert!(report["observation_error"].is_null());
        assert!(report["reported"]["installation"].is_null());
        assert_eq!(snapshot["canisters"].as_array().unwrap().len(), 4);
    }
    assert_eq!(pic.get_stable_memory(root).await, before_root);
    assert_eq!(pic.get_stable_memory(store).await, before_store);
}

#[tokio::test(flavor = "current_thread")]
async fn bootstrap_store_then_publish_resume_and_retain_versions() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_owned();
    let assets = PathBuf::from(std::env::var_os("CANIC_PUBLICATION_ARTIFACTS").unwrap());
    let binary = server_assets::check(&canic_qualification::server_directory())
        .expect("make qualification-setup");
    let mut owner = PocketIcStartupConfig::Spawn {
        server_binary: binary,
        timeout: Duration::from_secs(30),
        server_hard_ttl: Some(Duration::from_secs(300)),
        server_idle_ttl: None,
        server_output_files: None,
    }
    .build_instance(
        PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet(),
    )
    .await
    .unwrap();
    let operator = CanisterId::anonymous();
    let root_bytes = std::fs::read(assets.join("root.wasm")).unwrap();
    let store_bytes = std::fs::read(assets.join("store.wasm")).unwrap();
    let pic = owner.instance();
    let root = pic.create_canister().await;
    pic.add_cycles(root, 10_000_000_000_000).await;
    pic.install_canister(root, root_bytes.clone(), vec![], Some(operator))
        .await;
    let root_before = pic.get_stable_memory(root).await;
    let url = owner.instance_mut().make_live(None).await.to_string();
    let root_key = hex(&owner.instance().root_key().await.unwrap());
    let dir = repo
        .join("target/publication")
        .join(format!("run-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let wasm = dir.join("app.wasm");
    std::fs::write(
        &wasm,
        padded(std::fs::read(assets.join("app.wasm")).unwrap()),
    )
    .unwrap();
    let config = json!({"apps":{"assets":{
        "canisters": {"root":{"role":"root"}, "store":{"role":"wasm-store","parent":"root","wasm":assets.join("store.wasm")},
            "app":{"parent":"root","wasm":wasm}},
        "fleets":{"local":{"network":{"kind":"Local","url":url,"root_key":root_key,"routing_canister":root.to_text()},
            "journal":"journal", "canisters":{"root":{"id":root.to_text()}}}}
    }}});
    std::fs::write(dir.join("canic.json"), serde_json::to_vec(&config).unwrap()).unwrap();
    let inspected = Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(["artifact", "inspect", "--json"])
        .arg(&wasm)
        .output()
        .unwrap();
    assert!(
        inspected.status.success(),
        "{}",
        String::from_utf8_lossy(&inspected.stderr)
    );
    let inspected: Value = serde_json::from_slice(&inspected.stdout).unwrap();
    assert!(inspected["bytes"].as_u64().unwrap() > 1_500_000);
    let root_hash = hex(&ic_wasm_store_protocol::digest(&root_bytes));
    let store_hash = hex(&ic_wasm_store_protocol::digest(&store_bytes));
    let signer = repo.join(".tools/ic/bin/icp");
    let bootstrap = cli(
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
    let bootstrap_id = bootstrap["operation"].as_str().unwrap();
    let bootstrap_dir = dir.join("journal").join(bootstrap_id);
    assert_eq!(bootstrap["plan"]["creation_cycles"], 1_000_000_000_000_u64);
    assert_eq!(bootstrap["plan"]["store_root"]["canister"], root.to_text());
    // Changed Root control must refuse before any creation signing.
    owner
        .instance()
        .update_canister_settings(
            root,
            Some(operator),
            CanisterSettings {
                controllers: Some(vec![operator, CanisterId::self_authenticating(b"other")]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(
        !run(
            &dir,
            &[
                "fleet",
                "apply",
                bootstrap_id,
                "--identity",
                "anonymous",
                "--icp-bin",
                signer.to_str().unwrap()
            ]
        )
        .status
        .success()
    );
    assert!(!bootstrap_dir.join("create/signed.json").exists());
    owner
        .instance()
        .update_canister_settings(
            root,
            Some(operator),
            CanisterSettings {
                controllers: Some(vec![operator]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let subnet = owner.instance().get_subnet(root).await.unwrap();
    let before_creation = owner
        .instance()
        .get_subnet_metrics(subnet)
        .await
        .unwrap()
        .num_canisters;
    // Stop at the real signer boundary after controller handover has committed.
    let paused = dir.join("pause-readiness-icp");
    std::fs::write(&paused, "#!/bin/sh\nfor arg in \"$@\"; do\n case \"$arg\" in */store-observe/0/arguments.bin) exit 71;; esac\ndone\nexec \"$CANIC_REAL_SIGNER\" \"$@\"\n").unwrap();
    std::fs::set_permissions(&paused, std::fs::Permissions::from_mode(0o700)).unwrap();
    let stopped = Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(["fleet", "apply", bootstrap_id, "--json", "--config"])
        .arg(dir.join("canic.json"))
        .args(["--identity", "anonymous", "--icp-bin"])
        .arg(&paused)
        .env("CANIC_REAL_SIGNER", &signer)
        .output()
        .unwrap();
    assert!(!stopped.status.success());
    assert!(
        bootstrap_dir.join("handover/reply.bin").exists(),
        "{}",
        String::from_utf8_lossy(&stopped.stderr)
    );
    assert!(!bootstrap_dir.join("receipt.json").exists());
    let attempts: Vec<_> = ["create", "install", "handover"]
        .iter()
        .map(|step| std::fs::read(bootstrap_dir.join(step).join("signed.json")).unwrap())
        .collect();
    // Simulate lost local handover reply; certified sole control reconstructs its effect.
    std::fs::remove_file(bootstrap_dir.join("handover/reply.bin")).unwrap();
    let completed_store = cli(
        &dir,
        &[
            "fleet",
            "reconcile",
            bootstrap_id,
            "--identity",
            "anonymous",
            "--icp-bin",
            signer.to_str().unwrap(),
        ],
    );
    let store_text = completed_store["canister"].as_str().unwrap();
    let store = CanisterId::from_text(store_text).unwrap();
    assert_eq!(owner.instance().get_subnet(store).await, Some(subnet));
    assert_eq!(
        serde_json::from_slice::<String>(
            &std::fs::read(bootstrap_dir.join("store-subnet.json")).unwrap()
        )
        .unwrap(),
        subnet.to_text()
    );
    assert_eq!(completed_store["controller"], root.to_text());
    assert_eq!(completed_store["module_hash"], store_hash);
    let after_creation = owner
        .instance()
        .get_subnet_metrics(subnet)
        .await
        .unwrap()
        .num_canisters;
    assert_eq!(after_creation, before_creation + 1);
    assert_eq!(
        cli(
            &dir,
            &[
                "fleet",
                "apply",
                bootstrap_id,
                "--identity",
                "anonymous",
                "--icp-bin",
                signer.to_str().unwrap()
            ]
        ),
        completed_store
    );
    assert_eq!(
        owner
            .instance()
            .get_subnet_metrics(subnet)
            .await
            .unwrap()
            .num_canisters,
        after_creation
    );
    for (step, bytes) in ["create", "install", "handover"].iter().zip(attempts) {
        assert_eq!(
            std::fs::read(bootstrap_dir.join(step).join("signed.json")).unwrap(),
            bytes
        );
    }
    assert!(!bootstrap_dir.join("handover/reply.bin").exists());
    assert_eq!(owner.instance().get_stable_memory(root).await, root_before);
    let readiness_path = bootstrap_dir.join("store-observe/0/reply.bin");
    let readiness = std::fs::read(&readiness_path).unwrap();
    std::fs::remove_file(&readiness_path).unwrap();
    assert!(
        !run(&dir, &["fleet", "status", bootstrap_id])
            .status
            .success()
    );
    std::fs::write(readiness_path, readiness).unwrap();
    cli(
        &dir,
        &["fleet", "canister", "bind", "store", "--id", store_text],
    );
    let plan = |version: &str, module: &str| {
        cli(
            &dir,
            &[
                "fleet",
                "plan",
                "--canister",
                "app",
                "--publish-version",
                version,
                "--store",
                "store",
                "--root-module",
                &root_hash,
                "--store-module",
                module,
                "--controller",
                "2vxsx-fae",
            ],
        )
    };
    let started = Instant::now();
    let first = plan("v1", &store_hash);
    let cold_plan_ms = started.elapsed().as_millis();
    assert_eq!(first["plan"]["artifact"], inspected);
    let id = first["operation"].as_str().unwrap();
    let retained = dir.join("journal").join(id);
    // Fail before signing the first chunk. Prepare has really committed through Root.
    let interrupted = dir.join("interrupted-icp");
    std::fs::write(&interrupted, "#!/bin/sh\nfor arg in \"$@\"; do\n case \"$arg\" in */publication/chunk-0/arguments.bin) exit 71;; esac\ndone\nexec \"$CANIC_REAL_SIGNER\" \"$@\"\n").unwrap();
    std::fs::set_permissions(&interrupted, std::fs::Permissions::from_mode(0o700)).unwrap();
    let started = Instant::now();
    let failed = Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(["fleet", "apply", id, "--json", "--config"])
        .arg(dir.join("canic.json"))
        .args(["--identity", "anonymous", "--icp-bin"])
        .arg(&interrupted)
        .env("CANIC_REAL_SIGNER", &signer)
        .output()
        .unwrap();
    let interrupted_prepare_ms = started.elapsed().as_millis();
    assert!(!failed.status.success());
    assert!(
        String::from_utf8_lossy(&failed.stderr).contains("signing failed"),
        "{}",
        String::from_utf8_lossy(&failed.stderr)
    );
    let prepare = retained.join("publication/prepare/signed.json");
    let signed = std::fs::read(&prepare).unwrap();
    assert!(retained.join("publication/prepare/reply.bin").exists());
    assert!(!retained.join("publication/chunk-0/signed.json").exists());
    let store_before = owner.instance().get_stable_memory(store).await;
    let apply = |action: &str, operation: &str| {
        cli(
            &dir,
            &[
                "fleet",
                action,
                operation,
                "--identity",
                "anonymous",
                "--icp-bin",
                signer.to_str().unwrap(),
            ],
        )
    };
    let started = Instant::now();
    let observed = apply("reconcile", id);
    let interrupted_reconcile_ms = started.elapsed().as_millis();
    assert_eq!(observed["publication"]["status"]["published"], false);
    assert!(
        !observed["publication"]["status"]["missing"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        owner.instance().get_stable_memory(store).await,
        store_before
    );
    let started = Instant::now();
    let completed = apply("apply", id);
    let cold_resume_ms = started.elapsed().as_millis();
    assert_eq!(completed["publication"]["status"]["published"], true);
    assert_eq!(std::fs::read(&prepare).unwrap(), signed);
    assert!(!retained.join("receipt.json").exists());
    let status = cli(&dir, &["fleet", "status", id]);
    assert!(status["receipt"].is_null());
    assert_eq!(status["publication"], completed["publication"]);
    let human = Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(["fleet", "status", id, "--config"])
        .arg(dir.join("canic.json"))
        .output()
        .unwrap();
    assert!(human.status.success());
    let text = String::from_utf8(human.stdout).unwrap();
    assert!(text.contains("Published receipt retained: true"));
    assert!(!text.contains("deployment completion is not established"));
    let view = cli(&dir, &["fleet", "show"]);
    assert_eq!(view["operations"].as_array().unwrap().len(), 1);
    assert_eq!(view["operations"][0]["operation"], bootstrap_id);
    assert!(view["issues"].as_array().unwrap().is_empty());
    assert_eq!(view["canisters"].as_array().unwrap().len(), 2);
    assert_eq!(plan("v1", &store_hash)["operation"], id);
    let published = owner.instance().get_stable_memory(store).await;
    assert_eq!(apply("apply", id), completed);
    assert_eq!(owner.instance().get_stable_memory(store).await, published);
    // A new immutable label can reuse existing chunks; it gets no new chunk signatures.
    let started = Instant::now();
    let second = plan("v2", &store_hash);
    let warm_plan_ms = started.elapsed().as_millis();
    let second_id = second["operation"].as_str().unwrap();
    let started = Instant::now();
    assert_eq!(
        apply("apply", second_id)["publication"]["status"]["published"],
        true
    );
    let warm_apply_ms = started.elapsed().as_millis();
    let (cold_chunks, cold_bytes) = publication_payload(&dir, id);
    let (warm_chunks, warm_bytes) = publication_payload(&dir, second_id);
    let input = std::fs::read(&wasm).unwrap();
    let unique_chunks: std::collections::BTreeMap<_, _> = input
        .chunks(ic_wasm_store_protocol::CHUNK_BYTES)
        .map(|chunk| (ic_wasm_store_protocol::digest(chunk), chunk.len()))
        .collect();
    assert_eq!(cold_chunks, unique_chunks.len());
    assert_eq!(cold_bytes, unique_chunks.values().sum::<usize>());
    assert!(
        cold_bytes < input.len(),
        "fixture must exercise intra-Wasm chunk reuse"
    );
    assert!(cold_chunks > 1, "witness must exercise multiple chunks");
    assert_eq!((warm_chunks, warm_bytes), (0, 0));
    let measurements = json!({
        "scope": "local installed publication; interrupted cold upload and warm version reuse",
        "clock": "host monotonic wall time; milliseconds",
        "excludes": ["build", "replica setup", "Store bootstrap", "child deployment", "cycles", "network retransmissions"],
        "inputs": {"root_sha256": root_hash, "store_sha256": store_hash, "artifact": inspected,
            "unique_chunk_payload_bytes": unique_chunks.values().sum::<usize>()},
        "cold": {"operation": id, "plan_ms": cold_plan_ms,
            "interrupted_prepare_ms": interrupted_prepare_ms,
            "reconcile_ms": interrupted_reconcile_ms, "resume_ms": cold_resume_ms,
            "retained_put_requests": cold_chunks, "retained_put_payload_bytes": cold_bytes},
        "warm": {"operation": second_id, "plan_ms": warm_plan_ms, "apply_ms": warm_apply_ms,
            "retained_put_requests": warm_chunks, "retained_put_payload_bytes": warm_bytes}
    });
    assert!(
        !dir.join("journal")
            .join(second_id)
            .join("publication/chunk-0")
            .exists()
    );
    assert_eq!(apply("reconcile", id), completed);
    assert_eq!(owner.instance().get_stable_memory(root).await, root_before);
    let bad = plan("v3", &"00".repeat(32));
    let refused = run(
        &dir,
        &[
            "fleet",
            "apply",
            bad["operation"].as_str().unwrap(),
            "--identity",
            "anonymous",
            "--icp-bin",
            signer.to_str().unwrap(),
        ],
    );
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("code differs"));
    // Removing the signed proof invalidates a saved completion summary.
    let index = completed["publication"]["observation"].as_u64().unwrap();
    let proof = retained.join(format!("publication/observe/{index}/signed.json"));
    let evidence = std::fs::read(&proof).unwrap();
    std::fs::remove_file(&proof).unwrap();
    assert!(!run(&dir, &["fleet", "status", id]).status.success());
    std::fs::write(&proof, evidence).unwrap();
    assert_eq!(
        cli(&dir, &["fleet", "status", id])["publication"],
        completed["publication"]
    );
    // Complete the real application journey from retained publication, without inline staging.
    let deploy = |publication: &str, target: Option<(&str, &str)>| {
        let dispatch = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + if target.is_some() { 120 } else { 60 })
        .to_string();
        let mut args = vec![
            "fleet",
            "plan",
            "--canister",
            "app",
            "--root-module",
            &root_hash,
            "--controller",
            "2vxsx-fae",
            "--release",
            publication,
            "--execute-at",
            &dispatch,
        ];
        if let Some((child, previous)) = target {
            args.extend(["--target", child, "--previous-module", previous]);
        }
        cli(&dir, &args)
    };
    let child_plan = deploy(id, None);
    let child_id = child_plan["operation"].as_str().unwrap();
    let child_dir = dir.join("journal").join(child_id);
    assert_eq!(
        child_plan["plan"]["management"]["source"]["manifest"]["release"]["version"],
        "v1"
    );
    // A changed Store controller must fail before child admission is signed.
    owner
        .instance()
        .update_canister_settings(
            store,
            Some(root),
            CanisterSettings {
                controllers: Some(vec![operator]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(
        !run(
            &dir,
            &[
                "fleet",
                "apply",
                child_id,
                "--identity",
                "anonymous",
                "--icp-bin",
                signer.to_str().unwrap()
            ]
        )
        .status
        .success()
    );
    assert!(!child_dir.join("stage/signed.json").exists());
    owner
        .instance()
        .update_canister_settings(
            store,
            Some(operator),
            CanisterSettings {
                controllers: Some(vec![root]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let admitted = apply("apply", child_id);
    assert_eq!(admitted["status"]["phase"], "Scheduled");
    let signed_stage = std::fs::read(child_dir.join("stage/signed.json")).unwrap();
    assert!(
        std::fs::metadata(child_dir.join("stage/arguments.bin"))
            .unwrap()
            .len()
            < 8192
    );
    std::fs::remove_file(child_dir.join("stage/reply.bin")).unwrap();
    let recovered = apply("apply", child_id);
    assert_eq!(recovered["status"]["phase"], "Scheduled");
    assert_eq!(
        std::fs::read(child_dir.join("stage/signed.json")).unwrap(),
        signed_stage
    );
    assert!(!child_dir.join("stage/reply.bin").exists());
    assert_eq!(deploy(id, None)["operation"], child_id);
    // Same bytes under a different immutable label cannot replace an admitted source.
    assert!(
        !run(
            &dir,
            &[
                "fleet",
                "plan",
                "--canister",
                "app",
                "--root-module",
                &root_hash,
                "--controller",
                "2vxsx-fae",
                "--release",
                second_id
            ]
        )
        .status
        .success()
    );
    owner.instance().stop_progress().await;
    owner.instance().advance_time(Duration::from_secs(65)).await;
    for _ in 0..70 {
        owner.instance().tick().await;
    }
    owner.instance().auto_progress().await;
    let created = apply("apply", child_id);
    let child = created["canister"].as_str().unwrap();
    assert_eq!(created["module_hash"], inspected["sha256"]);
    assert_eq!(created["controller"], root.to_text());
    let canister_count = owner
        .instance()
        .get_subnet_metrics(subnet)
        .await
        .unwrap()
        .num_canisters;
    assert_eq!(apply("apply", child_id), created);
    assert_eq!(
        owner
            .instance()
            .get_subnet_metrics(subnet)
            .await
            .unwrap()
            .num_canisters,
        canister_count
    );
    // A new executable/version upgrades the same child; cache loss is reconstructed by Store.
    let mut next = std::fs::read(&wasm).unwrap();
    *next.last_mut().unwrap() = 43;
    std::fs::write(&wasm, &next).unwrap();
    let next_publication = plan("v4", &store_hash);
    let next_id = next_publication["operation"].as_str().unwrap();
    assert_eq!(
        apply("apply", next_id)["publication"]["status"]["published"],
        true
    );
    let upgrade = deploy(
        next_id,
        Some((child, created["module_hash"].as_str().unwrap())),
    );
    let upgrade_id = upgrade["operation"].as_str().unwrap();
    let admitted = apply("apply", upgrade_id);
    assert_eq!(admitted["upgrade"]["phase"], "Scheduled");
    owner
        .instance()
        .clear_chunk_store(store, Some(root))
        .await
        .unwrap();
    owner.instance().stop_progress().await;
    owner.instance().advance_time(Duration::from_secs(65)).await;
    for _ in 0..70 {
        owner.instance().tick().await;
    }
    owner.instance().auto_progress().await;
    let upgraded = apply("apply", upgrade_id);
    assert_eq!(upgraded["canister"], child);
    assert_eq!(
        upgraded["module_hash"],
        hex(&ic_wasm_store_protocol::digest(&next))
    );
    assert_eq!(apply("reconcile", upgrade_id), upgraded);
    assert_eq!(
        cli(&dir, &["fleet", "call", upgrade_id, "--name", "Store"])["greeting"],
        "Hello from Canic, Store!"
    );
    assert_eq!(
        owner
            .instance()
            .get_subnet_metrics(subnet)
            .await
            .unwrap()
            .num_canisters,
        canister_count
    );
    let view = cli(&dir, &["fleet", "show"]);
    assert!(view["issues"].as_array().unwrap().is_empty(), "{view}");
    let measurements_path = dir.join("publication-measurements.json");
    std::fs::write(
        &measurements_path,
        serde_json::to_vec_pretty(&measurements).unwrap(),
    )
    .unwrap();
    println!("Publication measurements: {}", measurements_path.display());
    owner.instance_mut().stop_live().await;
}
