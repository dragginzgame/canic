//! Standalone Canic profile application: real deployment, private state and preservation.
use canic_profile_protocol::{
    BIND, CHECK, CheckResult, GET, INFO, Info, LIMITS, MAX_PROFILES, PUT, ProfileError,
    ProfileResult, Put, ROUTE, Route, RouteError, RouteResult,
};
use ic_testkit::{
    pic::{PocketIcStartupConfig, server_assets},
    pocket_ic::{CanisterId, PocketIcBuilder, nonblocking::PocketIc},
};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

fn cli(root: &Path, args: &[&str]) -> Value {
    let output = Command::new(std::env::var_os("CANIC_CLI").expect("run make profile-test"))
        .arg("--state-dir")
        .arg(root)
        .arg("--json")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

async fn put(
    pic: &PocketIc,
    worker: CanisterId,
    sender: CanisterId,
    request: &Put,
) -> ProfileResult {
    let reply = pic
        .update_call(
            worker,
            sender,
            "put",
            ic_wire::native::encode_request(PUT, request, LIMITS).unwrap(),
        )
        .await
        .unwrap();
    ic_wire::native::decode_response(PUT, &reply, LIMITS).unwrap()
}

async fn get(pic: &PocketIc, worker: CanisterId, sender: CanisterId, id: u64) -> ProfileResult {
    let reply = pic
        .query_call(
            worker,
            sender,
            "get",
            ic_wire::native::encode_request(GET, &id, LIMITS).unwrap(),
        )
        .await
        .unwrap();
    ic_wire::native::decode_response(GET, &reply, LIMITS).unwrap()
}

async fn info(pic: &PocketIc, worker: CanisterId) -> Info {
    let reply = pic
        .query_call(
            worker,
            CanisterId::anonymous(),
            "info",
            ic_wire::native::encode_request(INFO, &(), LIMITS).unwrap(),
        )
        .await
        .unwrap();
    ic_wire::native::decode_response(INFO, &reply, LIMITS).unwrap()
}

async fn entry_journey(
    pic: &PocketIc,
    root: &Path,
    artifacts: &Path,
    network_args: &[&str],
    worker: CanisterId,
    icp: &Path,
) -> String {
    let wasm = artifacts.join("entry.wasm");
    let entry_bytes = std::fs::read(&wasm).unwrap();
    let entry_facts = ic_host_artifacts::wasm::inspect(
        &entry_bytes,
        ic_host_artifacts::wasm::InspectionLimits {
            module_bytes: 1_500_000,
            sections: 64,
            exports: 64,
            custom_sections: 32,
        },
    )
    .unwrap();
    let mut args = vec![
        "plan",
        "--wasm",
        wasm.to_str().unwrap(),
        "--controller",
        "2vxsx-fae",
    ];
    args.extend_from_slice(network_args);
    let plan = cli(root, &args);
    let receipt = cli(
        root,
        &[
            "apply",
            plan["operation"].as_str().unwrap(),
            "--identity",
            "anonymous",
            "--icp-bin",
            icp.to_str().unwrap(),
        ],
    );
    let entry_text = receipt["canister"].as_str().unwrap().to_owned();
    let entry = CanisterId::from_text(&entry_text).unwrap();
    let outsider = CanisterId::self_authenticating(b"entry outsider");
    let resolve = ic_wire::native::encode_request(ROUTE, &(), LIMITS).unwrap();
    let check = ic_wire::native::encode_request(CHECK, &(), LIMITS).unwrap();
    let reply = pic
        .update_call(entry, outsider, "resolve", resolve.clone())
        .await
        .unwrap();
    assert_eq!(
        ic_wire::native::decode_response::<RouteResult>(ROUTE, &reply, LIMITS).unwrap(),
        Err(RouteError::Unbound)
    );
    let route = Route {
        worker: worker.to_text(),
    };
    let bind = ic_wire::native::encode_request(BIND, &route, LIMITS).unwrap();
    let reply = pic
        .update_call(entry, outsider, "bind", bind.clone())
        .await
        .unwrap();
    assert_eq!(
        ic_wire::native::decode_response::<RouteResult>(BIND, &reply, LIMITS).unwrap(),
        Err(RouteError::Forbidden)
    );
    for invalid in [
        "not a principal".to_owned(),
        "2vxsx-fae".into(),
        "aaaaa-aa".into(),
        entry_text.clone(),
    ] {
        let bytes =
            ic_wire::native::encode_request(BIND, &Route { worker: invalid }, LIMITS).unwrap();
        let reply = pic
            .update_call(entry, CanisterId::anonymous(), "bind", bytes)
            .await
            .unwrap();
        assert_eq!(
            ic_wire::native::decode_response::<RouteResult>(BIND, &reply, LIMITS).unwrap(),
            Err(RouteError::InvalidWorker)
        );
    }
    assert!(
        pic.update_call(entry, CanisterId::anonymous(), "bind", vec![0; 4097])
            .await
            .is_err()
    );
    for _ in 0..2 {
        let reply = pic
            .update_call(entry, CanisterId::anonymous(), "bind", bind.clone())
            .await
            .unwrap();
        assert_eq!(
            ic_wire::native::decode_response::<RouteResult>(BIND, &reply, LIMITS)
                .unwrap()
                .unwrap(),
            route
        );
    }
    let replacement = ic_wire::native::encode_request(
        BIND,
        &Route {
            worker: outsider.to_text(),
        },
        LIMITS,
    )
    .unwrap();
    let reply = pic
        .update_call(entry, CanisterId::anonymous(), "bind", replacement)
        .await
        .unwrap();
    assert_eq!(
        ic_wire::native::decode_response::<RouteResult>(BIND, &reply, LIMITS).unwrap(),
        Err(RouteError::Conflict)
    );
    let reply = pic
        .update_call(entry, outsider, "check", check.clone())
        .await
        .unwrap();
    assert_eq!(
        ic_wire::native::decode_response::<CheckResult>(CHECK, &reply, LIMITS).unwrap(),
        Err(RouteError::Forbidden)
    );
    let reply = pic
        .update_call(entry, CanisterId::anonymous(), "check", check.clone())
        .await
        .unwrap();
    let observation = ic_wire::native::decode_response::<CheckResult>(CHECK, &reply, LIMITS)
        .unwrap()
        .unwrap();
    assert_eq!(observation.route, route);
    assert!(observation.info.ready);
    assert_eq!(observation.info.executable, "profile-v2");

    // Real native client resolves through ICP's certified update path, then queries the worker.
    let client = std::env::var_os("CANIC_PROFILE_CLIENT").unwrap();
    let url = network_args[3];
    let key = network_args[5];
    let base = [
        "--entry",
        &entry_text,
        "--identity",
        "anonymous",
        "--network",
        url,
        "--root-key",
        key,
        "--icp-bin",
        icp.to_str().unwrap(),
    ];
    let output = Command::new(&client)
        .args(base)
        .arg("info")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["canister"], worker.to_text());
    assert_eq!(value["entry"], entry_text);
    assert_eq!(value["certified"], false);
    assert_eq!(value["result"]["executable"], "profile-v2");
    let refused = Command::new(&client)
        .args(base)
        .args(["put", "999", "--name", "anonymous stays anonymous"])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert_eq!(
        get(pic, worker, entry, 999).await,
        Err(ProfileError::NotFound)
    );

    // Same executable upgrade exercises strict reopen of the entry's separate stable cell.
    pic.upgrade_canister(entry, std::fs::read(&wasm).unwrap(), vec![], None)
        .await
        .unwrap();
    let reply = pic
        .update_call(entry, outsider, "resolve", resolve)
        .await
        .unwrap();
    assert_eq!(
        ic_wire::native::decode_response::<RouteResult>(ROUTE, &reply, LIMITS)
            .unwrap()
            .unwrap(),
        route
    );
    let reply = pic
        .update_call(entry, CanisterId::anonymous(), "bind", bind)
        .await
        .unwrap();
    assert_eq!(
        ic_wire::native::decode_response::<RouteResult>(BIND, &reply, LIMITS)
            .unwrap()
            .unwrap(),
        route
    );

    // A retained route remains resolvable after the worker is unavailable; failure is not rerouting.
    pic.stop_canister(worker, None).await.unwrap();
    let reply = pic
        .update_call(entry, CanisterId::anonymous(), "check", check)
        .await
        .unwrap();
    assert_eq!(
        ic_wire::native::decode_response::<CheckResult>(CHECK, &reply, LIMITS).unwrap(),
        Err(RouteError::Unavailable)
    );
    pic.start_canister(worker, None).await.unwrap();
    let reply = pic
        .update_call(
            entry,
            CanisterId::anonymous(),
            "check",
            ic_wire::native::encode_request(CHECK, &(), LIMITS).unwrap(),
        )
        .await
        .unwrap();
    assert!(
        ic_wire::native::decode_response::<CheckResult>(CHECK, &reply, LIMITS)
            .unwrap()
            .unwrap()
            .info
            .ready
    );
    let status = pic.canister_status(entry, None).await.unwrap();
    assert!(u64::try_from(&status.memory_metrics.stable_memory_size.0).unwrap() <= 16 * 65_536);
    println!(
        "entry: Wasm={} bytes, code_section={} bytes, functions={}, stable={} bytes; bind/replay/refusal/certified resolution/direct check/unavailable/upgrade passed",
        entry_facts.raw_bytes,
        entry_facts.code_section_bytes,
        entry_facts.defined_functions,
        status.memory_metrics.stable_memory_size
    );
    entry_text
}

#[tokio::test(flavor = "current_thread")]
async fn profile_deploy_ownership_capacity_and_upgrade() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();
    let artifacts = repo.join("target/profile");
    let original = std::fs::read(artifacts.join("v1.wasm")).unwrap();
    let candidate = std::fs::read(artifacts.join("v2.wasm")).unwrap();
    assert_ne!(original, candidate);
    let facts = ic_host_artifacts::wasm::inspect(
        &original,
        ic_host_artifacts::wasm::InspectionLimits {
            module_bytes: 1_500_000,
            sections: 64,
            exports: 64,
            custom_sections: 32,
        },
    )
    .unwrap();
    assert!(facts.raw_bytes <= 1_500_000);
    let binary = server_assets::check(&repo.join("target/m0/server")).expect("run make m0-setup");
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
    let url = owner.instance_mut().make_live(None).await.to_string();
    let root_key: String = owner
        .instance()
        .root_key()
        .await
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let routing = owner.instance().create_canister().await.to_text();
    let root = artifacts.join(format!("journey-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let icp = repo.join(".tools/ic/bin/icp");
    let plan = cli(
        &root,
        &[
            "plan",
            "--wasm",
            artifacts.join("v1.wasm").to_str().unwrap(),
            "--network",
            "local",
            "--url",
            &url,
            "--root-key",
            &root_key,
            "--routing-canister",
            &routing,
            "--controller",
            "2vxsx-fae",
        ],
    );
    let id = plan["operation"].as_str().unwrap();
    let receipt = cli(
        &root,
        &[
            "apply",
            id,
            "--identity",
            "anonymous",
            "--icp-bin",
            icp.to_str().unwrap(),
        ],
    );
    let worker_text = receipt["canister"].as_str().unwrap();
    let worker = CanisterId::from_text(worker_text).unwrap();
    let pic = owner.instance();
    let initial = pic.canister_status(worker, None).await.unwrap();
    let stable_before = u64::try_from(&initial.memory_metrics.stable_memory_size.0).unwrap();
    assert!(stable_before <= 32 * 65_536);
    assert_eq!(info(pic, worker).await.executable, "profile-v1");

    // Exercise the shipped native client and real ICP CLI with the same Wire endpoint.
    let client =
        Command::new(std::env::var_os("CANIC_PROFILE_CLIENT").expect("run make profile-test"))
            .args([
                "--canister",
                worker_text,
                "--identity",
                "anonymous",
                "--network",
                &url,
                "--root-key",
                &root_key,
                "--icp-bin",
                icp.to_str().unwrap(),
                "info",
            ])
            .output()
            .unwrap();
    assert!(
        client.status.success(),
        "{}",
        String::from_utf8_lossy(&client.stderr)
    );
    let public: Value = serde_json::from_slice(&client.stdout).unwrap();
    assert_eq!(public["certified"], false);
    assert_eq!(public["result"]["ready"], true);

    let alice = CanisterId::self_authenticating(b"profile alice");
    let bob = CanisterId::self_authenticating(b"profile bob");
    let request = Put {
        user_id: 41,
        expected_revision: 0,
        display_name: "Alice private profile".into(),
    };
    assert_eq!(
        put(pic, worker, CanisterId::anonymous(), &request).await,
        Err(ProfileError::Unauthenticated)
    );
    let first = put(pic, worker, alice, &request).await.unwrap();
    assert_eq!(put(pic, worker, alice, &request).await.unwrap(), first);
    assert_eq!(
        get(pic, worker, bob, 41).await,
        Err(ProfileError::Forbidden)
    );
    assert_eq!(
        put(pic, worker, bob, &request).await,
        Err(ProfileError::Forbidden)
    );
    assert_eq!(
        put(
            pic,
            worker,
            alice,
            &Put {
                user_id: 99,
                ..request.clone()
            }
        )
        .await,
        Err(ProfileError::AlreadyRegistered)
    );
    assert_eq!(
        put(
            pic,
            worker,
            alice,
            &Put {
                display_name: "x".repeat(129),
                ..request.clone()
            }
        )
        .await,
        Err(ProfileError::InvalidInput)
    );
    assert!(
        pic.update_call(worker, alice, "put", vec![0; 4097])
            .await
            .is_err()
    );
    let mut other_profiles = Vec::new();
    for index in 1..MAX_PROFILES {
        let principal = CanisterId::self_authenticating(index.to_le_bytes());
        let profile = put(
            pic,
            worker,
            principal,
            &Put {
                user_id: 100 + index,
                display_name: "x".repeat(128),
                ..request.clone()
            },
        )
        .await
        .unwrap();
        other_profiles.push((principal, profile));
    }
    assert_eq!(
        put(
            pic,
            worker,
            bob,
            &Put {
                user_id: 99,
                ..request.clone()
            }
        )
        .await,
        Err(ProfileError::Capacity)
    );
    let changed = put(
        pic,
        worker,
        alice,
        &Put {
            expected_revision: 1,
            display_name: "Preserve across upgrade".into(),
            ..request.clone()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        put(pic, worker, alice, &request).await,
        Err(ProfileError::RevisionConflict)
    );
    let full = pic.canister_status(worker, None).await.unwrap();
    let stable_full = u64::try_from(&full.memory_metrics.stable_memory_size.0).unwrap();
    assert!(stable_full <= 32 * 65_536);

    let fault = std::fs::read(artifacts.join("reject.wasm")).unwrap();
    assert!(
        pic.upgrade_canister(worker, fault, vec![], None)
            .await
            .is_err()
    );
    assert_eq!(
        pic.canister_status(worker, None).await.unwrap().module_hash,
        initial.module_hash
    );
    assert_eq!(get(pic, worker, alice, 41).await.unwrap(), changed);
    for (principal, profile) in &other_profiles {
        assert_eq!(
            get(pic, worker, *principal, profile.user_id).await.unwrap(),
            *profile
        );
    }
    assert_eq!(info(pic, worker).await.executable, "profile-v1");
    let upgrade = cli(
        &root,
        &[
            "plan",
            "--wasm",
            artifacts.join("v2.wasm").to_str().unwrap(),
            "--network",
            "local",
            "--url",
            &url,
            "--root-key",
            &root_key,
            "--routing-canister",
            &routing,
            "--controller",
            "2vxsx-fae",
            "--target",
            worker_text,
            "--previous-module",
            receipt["module_hash"].as_str().unwrap(),
        ],
    );
    let upgrade_id = upgrade["operation"].as_str().unwrap();
    let upgraded = cli(
        &root,
        &[
            "apply",
            upgrade_id,
            "--identity",
            "anonymous",
            "--icp-bin",
            icp.to_str().unwrap(),
        ],
    );
    assert_eq!(upgraded["canister"], worker_text);
    assert_ne!(upgraded["module_hash"], receipt["module_hash"]);
    assert_eq!(info(pic, worker).await.executable, "profile-v2");
    assert_eq!(get(pic, worker, alice, 41).await.unwrap(), changed);
    for (principal, profile) in &other_profiles {
        assert_eq!(
            get(pic, worker, *principal, profile.user_id).await.unwrap(),
            *profile
        );
    }
    assert_eq!(
        get(pic, worker, bob, 41).await,
        Err(ProfileError::Forbidden)
    );
    assert_eq!(
        put(
            pic,
            worker,
            bob,
            &Put {
                user_id: 99,
                ..request.clone()
            }
        )
        .await,
        Err(ProfileError::Capacity)
    );
    let replay = cli(
        &root,
        &[
            "apply",
            upgrade_id,
            "--identity",
            "anonymous",
            "--icp-bin",
            "/missing-signer",
        ],
    );
    assert_eq!(replay, upgraded);
    let entry = entry_journey(
        pic,
        &root,
        &artifacts,
        &[
            "--network",
            "local",
            "--url",
            &url,
            "--root-key",
            &root_key,
            "--routing-canister",
            &routing,
        ],
        worker,
        &icp,
    )
    .await;
    let report = root.join("fleet.html");
    let fleet = cli(
        &root,
        &[
            "fleet",
            "--html",
            report.to_str().unwrap(),
            "--link",
            &format!("{entry}:{worker_text}"),
        ],
    );
    assert_eq!(fleet["canisters"].as_array().unwrap().len(), 2);
    assert_eq!(fleet["operations"].as_array().unwrap().len(), 3);
    let html = std::fs::read_to_string(report).unwrap();
    assert!(html.contains("Declared application routes"));
    assert!(html.contains(&entry) && html.contains(worker_text));
    assert_eq!(
        fleet["canisters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["canister"] == worker_text)
            .unwrap()["module_hash"],
        upgraded["module_hash"]
    );
    println!(
        "profile baseline: raw_wasm_bytes={}, code_section_bytes={}, functions={}, initial_stable_bytes={stable_before}, full_stable_bytes={stable_full}, initial_heap_bytes={}, full_heap_bytes={}",
        facts.raw_bytes,
        facts.code_section_bytes,
        facts.defined_functions,
        initial.memory_metrics.wasm_memory_size,
        full.memory_metrics.wasm_memory_size
    );
    println!(
        "profile: Canic deploy/client/read/write/limits/failed-upgrade/v2 preservation/no-signer replay/fleet passed"
    );
    owner.close().await;
}
