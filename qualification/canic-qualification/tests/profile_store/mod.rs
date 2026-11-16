//! Store-backed profile composition through the existing fleet CLI and readiness owner.
use super::{
    CanisterId, Duration, PathBuf, PocketIc, PocketIcBuilder, PocketIcStartupConfig, cli, repo,
    server_assets, status,
};
use canic_management_protocol::upgrade;
use canic_profile_protocol::{
    GET, INFO, Info, LIMITS, PUT, Profile, ProfileError, ProfileResult, Put,
};
use serde_json::{Value, json};

async fn complete(config: &std::path::Path, operation: &str, signer: &std::path::Path) -> Value {
    for _ in 0..100 {
        let result = cli(
            config,
            &[
                "fleet",
                "apply",
                operation,
                "--identity",
                "profile-fixture",
                "--icp-bin",
                signer.to_str().unwrap(),
            ],
        );
        if result["canister"].is_string()
            || result["children"][0]["outcome"]["canister"].is_string()
            || result["publication"]["status"]["published"] == true
        {
            return result;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("profile operation did not complete: {operation}");
}

async fn get(pic: &PocketIc, child: CanisterId, caller: CanisterId, user: u64) -> ProfileResult {
    let reply = pic
        .query_call(
            child,
            caller,
            "get",
            ic_wire::encode(
                &user,
                ic_wire::Encoding::Frame {
                    method: GET,
                    direction: ic_wire::Direction::Request,
                },
                LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: GET,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
}

async fn put(pic: &PocketIc, child: CanisterId, caller: CanisterId, input: &Put) -> ProfileResult {
    let reply = pic
        .update_call(
            child,
            caller,
            "put",
            ic_wire::encode(
                input,
                ic_wire::Encoding::Frame {
                    method: PUT,
                    direction: ic_wire::Direction::Request,
                },
                LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: PUT,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
}

async fn verify_profiles(
    pic: &PocketIc,
    child: CanisterId,
    root: CanisterId,
    profiles: &[Profile],
) {
    for profile in profiles {
        let owner = CanisterId::from_slice(&profile.owner);
        assert_eq!(
            get(pic, child, owner, profile.user_id).await,
            Ok(profile.clone())
        );
        assert_eq!(
            get(pic, child, root, profile.user_id).await,
            Err(ProfileError::Forbidden)
        );
        for other in profiles.iter().filter(|other| other.owner != profile.owner) {
            assert_eq!(
                get(
                    pic,
                    child,
                    CanisterId::from_slice(&other.owner),
                    profile.user_id
                )
                .await,
                Err(ProfileError::Forbidden)
            );
        }
    }
}

async fn info(pic: &PocketIc, child: CanisterId) -> Info {
    let reply = pic
        .query_call(
            child,
            CanisterId::anonymous(),
            "info",
            ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method: INFO,
                    direction: ic_wire::Direction::Request,
                },
                LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: INFO,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
}

async fn upgrade_status(pic: &PocketIc, root: CanisterId) -> upgrade::Status {
    let reply = pic
        .query_call(
            root,
            CanisterId::anonymous(),
            "upgrade_status",
            ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method: upgrade::STATUS,
                    direction: ic_wire::Direction::Request,
                },
                canic_management_protocol::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: upgrade::STATUS,
            direction: ic_wire::Direction::Response,
        },
        canic_management_protocol::LIMITS,
    )
    .unwrap()
}

#[tokio::test]
async fn profile_store_fleet_preserves_users_across_child_and_owner_upgrades() {
    let mut replica = PocketIcStartupConfig::Spawn {
        server_binary: server_assets::check(&canic_qualification::server_directory()).unwrap(),
        timeout: Duration::from_secs(30),
        server_hard_ttl: Some(Duration::from_secs(240)),
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
    let routing = replica.instance().create_canister().await;
    let url = replica.instance_mut().make_live(None).await.to_string();
    let pic = replica.instance();
    let key: String = pic
        .root_key()
        .await
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let assets = repo().join("target/root");
    let directory = assets.join(format!("profile-store-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let (signer, controller) = canic_qualification::isolated_signer(&directory, "profile-fixture");
    let config = directory.join("canic.json");
    let mut value = json!({"apps":{"profiles":{
        "canisters":{
            "root":{"role":"root","wasm":assets.join("profile-manager.wasm")},
            "store":{"role":"wasm-store","parent":"root","wasm":assets.join("store.wasm")},
            "worker":{"role":"worker","parent":"root","wasm":assets.join("profile-worker.wasm")}
        },
        "fleets":{"local":{"network":{"kind":"Local","url":url,"root_key":key,"routing_canister":routing.to_text()},"canisters":{}}}
    }}});
    std::fs::write(&config, serde_json::to_vec(&value).unwrap()).unwrap();
    let plan = || cli(&config, &["fleet", "plan", "--controller", &controller]);
    let initial = plan();
    let created = complete(&config, initial["operation"].as_str().unwrap(), &signer).await;
    let root = CanisterId::from_text(created["root"]["canister"].as_str().unwrap()).unwrap();
    let child = CanisterId::from_text(
        created["children"][0]["outcome"]["canister"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let store = CanisterId::from_text(created["store"]["canister"].as_str().unwrap()).unwrap();
    assert_eq!(
        created["children"][0]["outcome"]["controller"],
        root.to_text()
    );
    assert_eq!(
        status(pic, root).await.phase,
        canic_management_protocol::Phase::Ready
    );
    assert!(info(pic, child).await.ready);
    assert_eq!(info(pic, child).await.executable, "profile-v1");
    let mut profiles = Vec::new();
    for (user, name) in [(7_u64, "Alice"), (8_u64, "Bob")] {
        let caller = CanisterId::self_authenticating(name.as_bytes());
        let input = Put {
            user_id: user,
            expected_revision: 0,
            display_name: name.into(),
        };
        profiles.push(put(pic, child, caller, &input).await.unwrap());
    }
    verify_profiles(pic, child, root, &profiles).await;
    let application_state = pic.get_stable_memory(child).await;

    // Bootstrap uses the normal product. Select the existing qualification-only
    // fault owner through maintenance before exercising callback uncertainty.
    value["apps"]["profiles"]["canisters"]["root"]["wasm"] =
        json!(assets.join("profile-faults.wasm"));
    std::fs::write(&config, serde_json::to_vec(&value).unwrap()).unwrap();
    let fault_owner = plan();
    assert_eq!(fault_owner["action"], "maintenance");
    complete(&config, fault_owner["operation"].as_str().unwrap(), &signer).await;
    pic.update_call(
        root,
        CanisterId::from_text(&controller).unwrap(),
        "inject_fault",
        ic_wire::encode(
            &8_u8,
            ic_wire::Encoding::Frame {
                method: canic_management_protocol::INJECT,
                direction: ic_wire::Direction::Request,
            },
            canic_management_protocol::LIMITS,
        )
        .unwrap(),
    )
    .await
    .unwrap();
    value["apps"]["profiles"]["canisters"]["worker"]["wasm"] =
        json!(assets.join("profile-worker-v2.wasm"));
    std::fs::write(&config, serde_json::to_vec(&value).unwrap()).unwrap();
    let publication = plan();
    assert_eq!(publication["action"], "publication");
    complete(&config, publication["operation"].as_str().unwrap(), &signer).await;
    let upgrade = plan();
    assert_eq!(upgrade["action"], "upgrade");
    let id = upgrade["operation"].as_str().unwrap();
    cli(
        &config,
        &[
            "fleet",
            "apply",
            id,
            "--identity",
            "profile-fixture",
            "--icp-bin",
            signer.to_str().unwrap(),
        ],
    );
    let mut interrupted = upgrade_status(pic, root).await;
    for _ in 0..100 {
        if interrupted.phase == upgrade::Phase::InstallIssued
            && info(pic, child).await.executable == "profile-v2"
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        interrupted = upgrade_status(pic, root).await;
    }
    assert_eq!(interrupted.phase, upgrade::Phase::InstallIssued);
    assert_eq!(info(pic, child).await.executable, "profile-v2");
    assert_eq!(interrupted.attempts[1], 1);
    assert!(interrupted.requests[1].is_some());
    assert!(interrupted.observed_version.is_none());
    pic.stop_progress().await;
    pic.advance_time(Duration::from_secs(3)).await;
    for _ in 0..20 {
        pic.tick().await;
    }
    assert_eq!(upgrade_status(pic, root).await, interrupted);
    assert!(
        super::metrics::read(pic, root)
            .await
            .timers
            .unwrap()
            .abandoned
            .total
            > 0
    );
    pic.auto_progress().await;
    assert!(
        pic.get_stable_memory(child).await == application_state,
        "interrupted callback changed application storage"
    );
    verify_profiles(pic, child, root, &profiles).await;
    assert_eq!(plan(), upgrade);
    let pending = cli(
        &config,
        &[
            "fleet",
            "apply",
            id,
            "--identity",
            "profile-fixture",
            "--icp-bin",
            signer.to_str().unwrap(),
        ],
    );
    assert!(pending["canister"].is_null());
    assert_eq!(upgrade_status(pic, root).await, interrupted);
    let journal = PathBuf::from(upgrade["state_dir"].as_str().unwrap()).join(id);
    assert!(!journal.join("receipt.json").exists());
    let signed = std::fs::read(journal.join("stage/signed.json")).unwrap();
    cli(
        &config,
        &[
            "fleet",
            "reconcile",
            id,
            "--identity",
            "profile-fixture",
            "--icp-bin",
            signer.to_str().unwrap(),
        ],
    );
    let upgraded = complete(&config, upgrade["operation"].as_str().unwrap(), &signer).await;
    let recovered = upgrade_status(pic, root).await;
    assert_eq!(recovered.phase, upgrade::Phase::Ready);
    assert_eq!(recovered.approval, interrupted.approval);
    assert_eq!(recovered.attempts, interrupted.attempts);
    assert_eq!(recovered.requests, interrupted.requests);
    assert_eq!(recovered.previous_version, interrupted.previous_version);
    assert_eq!(
        recovered.observed_version,
        interrupted.previous_version.map(|v| v + 1)
    );
    assert_eq!(
        std::fs::read(journal.join("stage/signed.json")).unwrap(),
        signed
    );
    assert_eq!(upgraded["canister"], child.to_text());
    assert_eq!(info(pic, child).await.executable, "profile-v2");
    assert!(
        pic.get_stable_memory(child).await == application_state,
        "worker upgrade changed application storage"
    );
    verify_profiles(pic, child, root, &profiles).await;
    value["apps"]["profiles"]["canisters"]["store"]["wasm"] =
        json!(assets.join("store-maintenance.wasm"));
    std::fs::write(&config, serde_json::to_vec(&value).unwrap()).unwrap();
    let store_upgrade = plan();
    assert_eq!(store_upgrade["action"], "upgrade");
    assert_eq!(
        store_upgrade["plan"]["store_upgrade_root"]["canister"],
        root.to_text()
    );
    let maintained_store = complete(
        &config,
        store_upgrade["operation"].as_str().unwrap(),
        &signer,
    )
    .await;
    assert_eq!(maintained_store["canister"], store.to_text());
    let store_status = cli(
        &config,
        &[
            "fleet",
            "status",
            store_upgrade["operation"].as_str().unwrap(),
        ],
    );
    let maintenance: canic_management_protocol::store_maintenance::Status =
        serde_json::from_value(store_status["store_maintenance"].clone()).unwrap();
    let canic_management_protocol::store_maintenance::Scan::Entries(Some(preservation)) =
        &maintenance.preservation
    else {
        panic!("new Store maintenance must retain the explicit entries scan");
    };
    assert!(preservation.complete);
    assert_eq!(preservation.observed, 2);
    assert_eq!(
        maintenance.preservation,
        maintenance.installation.as_ref().unwrap().verification
    );
    // Activation retains a new attempt fence. Compare complete Store backing
    // after that legitimate transition, across the following owner replacement.
    let store_state = pic.get_stable_memory(store).await;
    verify_profiles(pic, child, root, &profiles).await;
    value["apps"]["profiles"]["canisters"]["root"]["wasm"] =
        json!(assets.join("profile-maintenance.wasm"));
    std::fs::write(&config, serde_json::to_vec(&value).unwrap()).unwrap();
    let maintenance = plan();
    assert_eq!(maintenance["action"], "maintenance");
    assert_eq!(
        maintenance["plan"]["maintenance_store"]["sources"][0]["upgrades"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let maintained = complete(&config, maintenance["operation"].as_str().unwrap(), &signer).await;
    assert_eq!(maintained["canister"], root.to_text());
    assert!(
        pic.get_stable_memory(store).await == store_state,
        "owner upgrade changed Store storage"
    );
    assert!(
        pic.get_stable_memory(child).await == application_state,
        "owner upgrade changed application storage"
    );
    verify_profiles(pic, child, root, &profiles).await;
    assert_eq!(plan()["action"], "unchanged");

    // Preservation must leave a usable application. Both users write through the
    // replaced infrastructure; retries and refusals must not change their state.
    for profile in &mut profiles {
        let owner = CanisterId::from_slice(&profile.owner);
        let input = Put {
            user_id: profile.user_id,
            expected_revision: profile.revision,
            display_name: format!("{} after maintenance", profile.display_name),
        };
        let updated = put(pic, child, owner, &input).await.unwrap();
        assert_eq!(updated.user_id, profile.user_id);
        assert_eq!(updated.owner, profile.owner);
        assert_eq!(updated.revision, profile.revision + 1);
        assert_eq!(updated.display_name, input.display_name);
        let retained = pic.get_stable_memory(child).await;
        assert_eq!(put(pic, child, owner, &input).await, Ok(updated.clone()));
        let conflict = Put {
            display_name: "stale conflicting write".into(),
            ..input
        };
        assert_eq!(
            put(pic, child, owner, &conflict).await,
            Err(ProfileError::RevisionConflict)
        );
        assert_eq!(
            put(pic, child, root, &conflict).await,
            Err(ProfileError::Forbidden)
        );
        assert!(
            pic.get_stable_memory(child).await == retained,
            "retry or refused write changed application storage"
        );
        *profile = updated;
    }
    verify_profiles(pic, child, root, &profiles).await;
    let updated_state = pic.get_stable_memory(child).await;

    // Re-select a compatible retained executable under a fresh approval. This
    // changes code, never restores an old application snapshot or old authority.
    value["apps"]["profiles"]["canisters"]["worker"]["wasm"] =
        json!(assets.join("profile-worker.wasm"));
    std::fs::write(&config, serde_json::to_vec(&value).unwrap()).unwrap();
    let publication = plan();
    assert_eq!(publication["action"], "publication");
    complete(&config, publication["operation"].as_str().unwrap(), &signer).await;
    let next = plan();
    assert_eq!(next["action"], "upgrade");
    assert_ne!(next["operation"], upgrade["operation"]);
    assert_eq!(next["plan"]["previous_module"], upgraded["module_hash"]);
    assert_eq!(
        next["plan"]["management"]["root_module"],
        maintained["module_hash"]
    );
    assert_eq!(
        next["plan"]["management"]["source"]["store"]["module"],
        maintained_store["module_hash"]
    );
    assert_eq!(
        next["plan"]["management"]["source"]["maintenance"]["sequence"],
        1
    );
    let continued = complete(&config, next["operation"].as_str().unwrap(), &signer).await;
    assert_eq!(continued["canister"], child.to_text());
    assert_eq!(info(pic, child).await.executable, "profile-v1");
    assert!(
        pic.get_stable_memory(child).await == updated_state,
        "subsequent upgrade changed current application storage"
    );
    verify_profiles(pic, child, root, &profiles).await;
    assert_eq!(plan()["action"], "unchanged");

    // Grow the real application after maintenance. A name sorting before worker
    // must not move its allocation or route either user's existing data.
    cli(
        &config,
        &[
            "app",
            "canister",
            "add",
            "archive",
            "--parent",
            "root",
            "--role",
            "worker",
            "--wasm",
            assets.join("profile-worker.wasm").to_str().unwrap(),
        ],
    );
    value = serde_json::from_slice(&std::fs::read(&config).unwrap()).unwrap();
    let publication = plan();
    assert_eq!(publication["action"], "publication");
    complete(&config, publication["operation"].as_str().unwrap(), &signer).await;
    let addition = plan();
    assert_eq!(addition["action"], "add_child");
    assert_eq!(addition["plan"]["management"]["slot"], "Second");
    let added = complete(&config, addition["operation"].as_str().unwrap(), &signer).await;
    let archive = CanisterId::from_text(added["canister"].as_str().unwrap()).unwrap();
    assert_ne!(archive, child);
    assert_eq!(added["controller"], root.to_text());
    assert_eq!(info(pic, archive).await.executable, "profile-v1");
    let mut archive_profiles = Vec::new();
    for profile in &profiles {
        let caller = CanisterId::from_slice(&profile.owner);
        assert_eq!(
            get(pic, archive, caller, profile.user_id).await,
            Err(ProfileError::NotFound)
        );
        archive_profiles.push(
            put(
                pic,
                archive,
                caller,
                &Put {
                    user_id: profile.user_id,
                    expected_revision: 0,
                    display_name: format!("{} in archive", profile.display_name),
                },
            )
            .await
            .unwrap(),
        );
    }
    verify_profiles(pic, archive, root, &archive_profiles).await;
    verify_profiles(pic, child, root, &profiles).await;
    assert_eq!(pic.get_stable_memory(child).await, updated_state);
    let archive_state = pic.get_stable_memory(archive).await;

    value["apps"]["profiles"]["canisters"]["archive"]["wasm"] =
        json!(assets.join("profile-worker-v2.wasm"));
    std::fs::write(&config, serde_json::to_vec(&value).unwrap()).unwrap();
    let publication = plan();
    assert_eq!(publication["action"], "publication");
    complete(&config, publication["operation"].as_str().unwrap(), &signer).await;
    let archive_upgrade = plan();
    assert_eq!(archive_upgrade["action"], "upgrade");
    assert_eq!(archive_upgrade["plan"]["target"], archive.to_text());
    complete(
        &config,
        archive_upgrade["operation"].as_str().unwrap(),
        &signer,
    )
    .await;
    assert_eq!(info(pic, archive).await.executable, "profile-v2");
    assert_eq!(info(pic, child).await.executable, "profile-v1");
    assert_eq!(pic.get_stable_memory(archive).await, archive_state);
    assert_eq!(pic.get_stable_memory(child).await, updated_state);
    let before_maintenance = cli(&config, &["fleet", "show"]);
    let timers = &before_maintenance["metrics"][0]["reported"]["timers"];
    assert!(
        timers["accepted"]["total"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 0
    );
    assert_eq!(timers["accepted"]["overflowed"], false);

    // Root replacement must include the later child's custody as well as the
    // original worker, without changing either application's records or Store.
    value["apps"]["profiles"]["canisters"]["root"]["wasm"] =
        json!(assets.join("profile-manager.wasm"));
    std::fs::write(&config, serde_json::to_vec(&value).unwrap()).unwrap();
    let maintenance = plan();
    assert_eq!(maintenance["action"], "maintenance");
    assert_eq!(
        maintenance["plan"]["maintenance"]["creation"]["worker"],
        child.to_text()
    );
    assert_eq!(
        maintenance["plan"]["maintenance_sibling"]["creation"]["worker"],
        archive.to_text()
    );
    assert_eq!(
        maintenance["plan"]["maintenance_sibling"]["upgrades"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let root_state = pic.get_stable_memory(root).await;
    let store_state = pic.get_stable_memory(store).await;
    complete(&config, maintenance["operation"].as_str().unwrap(), &signer).await;
    assert_eq!(pic.get_stable_memory(root).await, root_state);
    assert_eq!(pic.get_stable_memory(store).await, store_state);
    assert_eq!(pic.get_stable_memory(archive).await, archive_state);
    assert_eq!(pic.get_stable_memory(child).await, updated_state);
    verify_profiles(pic, archive, root, &archive_profiles).await;
    verify_profiles(pic, child, root, &profiles).await;
    assert_eq!(plan()["action"], "unchanged");
    let view = cli(&config, &["fleet", "show"]);
    assert_eq!(view["canisters"].as_array().unwrap().len(), 4);
    assert!(view["issues"].as_array().unwrap().is_empty(), "{view}");
    assert_eq!(view["managers"][0]["reported"]["phase"], "Ready");
    assert_eq!(view["managers"][0]["upgrade"]["reported"]["phase"], "Ready");
    assert_eq!(view["stores"][0]["reported"]["phase"], "Ready");
    assert_eq!(view["metrics"].as_array().unwrap().len(), 1);
    assert_eq!(view["metrics"][0]["canister"], root.to_text());
    let timer_report = &view["metrics"][0]["reported"]["timers"];
    // Volatile diagnostics start a new epoch; durable effect custody above survives.
    assert_eq!(timer_report["accepted"]["total"], "0");
    assert_ne!(
        timer_report["started_at_version"],
        timers["started_at_version"]
    );
    assert_eq!(timer_report["accepted"]["overflowed"], false);

    // A valid, installed greeting is not profile readiness. Keep the same
    // management owner and prove the incompatible application remains blocked.
    value["apps"]["profiles"]["canisters"]
        .as_object_mut()
        .unwrap()
        .remove("archive");
    value["apps"]["profiles"]["canisters"]["worker"]["wasm"] = json!(assets.join("app.wasm"));
    value["apps"]["profiles"]["fleets"]["local"]["journal"] = json!("refused-journal");
    let refused_config = directory.join("refused.json");
    std::fs::write(&refused_config, serde_json::to_vec(&value).unwrap()).unwrap();
    let refused = cli(
        &refused_config,
        &["fleet", "plan", "--controller", &controller],
    );
    let id = refused["operation"].as_str().unwrap();
    let pending = cli(
        &refused_config,
        &[
            "fleet",
            "apply",
            id,
            "--identity",
            "profile-fixture",
            "--icp-bin",
            signer.to_str().unwrap(),
        ],
    );
    let refused_root =
        CanisterId::from_text(pending["root"]["canister"].as_str().unwrap()).unwrap();
    let mut observed = status(pic, refused_root).await;
    for _ in 0..100 {
        if observed.failure.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        observed = status(pic, refused_root).await;
    }
    assert_eq!(observed.phase, canic_management_protocol::Phase::Verified);
    assert!(observed.failure.is_some());
    assert_eq!(observed.attempts, [1, 1, 1]);
    assert_eq!(
        observed.observed_module,
        Some(observed.approval.as_ref().unwrap().module_hash.to_vec())
    );
    let retained = cli(
        &refused_config,
        &[
            "fleet",
            "apply",
            id,
            "--identity",
            "profile-fixture",
            "--icp-bin",
            signer.to_str().unwrap(),
        ],
    );
    assert!(retained["children"][0]["outcome"]["canister"].is_null());
    let journal = PathBuf::from(refused["state_dir"].as_str().unwrap()).join(id);
    assert!(!journal.join("child/receipt.json").exists());
    assert_eq!(status(pic, refused_root).await.attempts, [1, 1, 1]);
    assert!(
        super::metrics::read(pic, refused_root)
            .await
            .timers
            .unwrap()
            .expected_failures
            .total
            > 0
    );
    replica.instance_mut().stop_live().await;
}
