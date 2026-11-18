//! The actual fleet CLI publishes and installs a counter through separate Root/Store owners.
use super::*;
use canic_demo::{Error, Request};
use canic_management_protocol::{Phase, stored};
use ic_testkit::pocket_ic::nonblocking::PocketIc;
use ic_wire::{Direction, Encoding};

#[path = "../support/counter.rs"]
mod counter_client;
use counter_client::counter;

async fn custody(pic: &PocketIc, root: CanisterId) -> stored::Custody {
    let reply = pic
        .update_call(
            root,
            CanisterId::anonymous(),
            "observe_stored_children",
            ic_wire::encode(
                &(),
                Encoding::Frame {
                    method: stored::CUSTODY,
                    direction: Direction::Request,
                },
                canic_management_protocol::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode(
        &reply,
        Encoding::Frame {
            method: stored::CUSTODY,
            direction: Direction::Response,
        },
        stored::LIMITS,
    )
    .unwrap()
}

async fn advance(pic: &PocketIc) {
    pic.stop_progress().await;
    pic.advance_time(Duration::from_secs(20)).await;
    for _ in 0..70 {
        pic.tick().await;
    }
    pic.auto_progress().await;
}

#[tokio::test(flavor = "current_thread")]
async fn fleet_counter_preserves_value_and_never_treats_fenced_as_ready() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_owned();
    let assets = PathBuf::from(std::env::var_os("CANIC_PUBLICATION_ARTIFACTS").unwrap());
    for fenced in [false, true] {
        let mut server = Server::start(
            PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
            Limits {
                startup: Duration::from_secs(30),
                lifetime: Duration::from_secs(240),
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
        let routing = owner.client().create_canister().await;
        let url = owner.client_mut().make_live(None).await.to_string();
        let dir = assets.join(format!("counter-{fenced}-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let counter_wasm = assets.join(if fenced {
            "counter-fenced.wasm"
        } else {
            "counter-v1.wasm"
        });
        std::fs::copy(counter_wasm, dir.join("app.wasm")).unwrap();
        std::fs::copy(
            repo.join("apps/demo/artifacts/app.client"),
            dir.join("app.client"),
        )
        .unwrap();
        let config = json!({"apps":{"demo":{
            "canisters": {
                "root":{"role":"root","wasm":assets.join("root.wasm")},
                "store":{"role":"wasm-store","parent":"root","wasm":assets.join("store.wasm")},
                "app":{"role":"app","parent":"root","wasm":dir.join("app.wasm"),
                    "build":{"manifest":repo.join("Cargo.toml"),"package":"canic-demo","client":"canic-demo-client"}}
            },
            "fleets":{"local":{"network":{"kind":"Local","url":url,
                "root_key":hex(&owner.client().root_key().await.unwrap()),
                "root_subnet":owner.client().topology().await.get_nns().unwrap().to_text(),
                "routing_canister":routing.to_text()},"journal":"journal","canisters":{}}}
        }}});
        std::fs::write(dir.join("canic.toml"), toml::to_string(&config).unwrap()).unwrap();
        let signer = repo.join(".tools/ic/bin/icp");
        let plan = || cli(&dir, &["fleet", "plan", "--controller", "2vxsx-fae"]);
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
        let initial = plan();
        let id = initial["operation"].as_str().unwrap();
        apply(id);
        advance(owner.client()).await;
        let result = apply(id);
        let report = cli(&dir, &["fleet", "status", id]);
        let root =
            CanisterId::from_text(report["root"]["receipt"]["canister"].as_str().unwrap()).unwrap();
        let state = custody(owner.client(), root).await;
        let creation = &state.children[0].creation;
        let child = CanisterId::from_text(creation.worker.as_ref().unwrap()).unwrap();
        assert_eq!(creation.attempts, [1; 3]);
        assert_eq!(
            owner
                .client()
                .canister_status(child, Some(root))
                .await
                .unwrap()
                .settings
                .controllers,
            [root]
        );
        let alice = CanisterId::self_authenticating(b"counter alice");
        let bob = CanisterId::self_authenticating(b"counter bob");
        // Even Root's external controller is not the application's installation authority.
        assert_eq!(
            counter(
                owner.client(),
                child,
                CanisterId::anonymous(),
                Request::Fence { attempt: 2 }
            )
            .await,
            Err(Error::NotOwner)
        );
        if fenced {
            assert_eq!(creation.phase, Phase::Verified, "{state:?}");
            assert_eq!(
                creation.failure,
                Some(canic_management_protocol::Failure::Mismatch)
            );
            assert!(!dir.join("journal").join(id).join("receipt.json").exists());
            let stable = owner.client().get_stable_memory(child).await;
            for _ in 0..2 {
                assert_eq!(
                    counter(owner.client(), child, alice, Request::Increment).await,
                    Err(Error::Fenced)
                );
                assert_eq!(
                    counter(owner.client(), child, bob, Request::Activate { attempt: 1 }).await,
                    Err(Error::NotOwner)
                );
                apply(id);
                advance(owner.client()).await;
                let observed = custody(owner.client(), root).await;
                assert_eq!(observed.children[0].creation.phase, Phase::Verified);
                assert_eq!(observed.children[0].creation.attempts, creation.attempts);
                assert_eq!(observed.children[0].creation.worker, creation.worker);
                assert_eq!(owner.client().get_stable_memory(child).await, stable);
            }
            assert_eq!(
                counter(owner.client(), child, alice, Request::Read)
                    .await
                    .unwrap()
                    .fence,
                Some(1)
            );
        } else {
            assert_eq!(creation.phase, Phase::Ready, "{state:?}");
            assert_eq!(
                result["children"][0]["outcome"]["canister"],
                child.to_text()
            );
            let user_home = dir.join("visitor");
            std::fs::create_dir(&user_home).unwrap();
            let (user_signer, user_principal) =
                canic_qualification::isolated_signer(&user_home, "visitor");
            assert_ne!(user_principal, root.to_text());
            assert_ne!(user_principal, "2vxsx-fae");
            let invoke = |name: &str| {
                cli(
                    &dir,
                    &[
                        "fleet",
                        "call",
                        "--canister",
                        "app",
                        "--identity",
                        "visitor",
                        "--icp-bin",
                        user_signer.to_str().unwrap(),
                        name,
                    ],
                )
            };
            let called = invoke("increment");
            assert_eq!(called["caller"], user_principal);
            assert_eq!(called["canister"], child.to_text());
            assert_eq!(called["result"]["Ok"]["value"], 1);
            let call_id = called["operation"].as_str().unwrap();
            let call_status = cli(&dir, &["fleet", "status", call_id]);
            assert_eq!(call_status["effects"]["call"], "reply_retained");
            assert!(
                call_status["receipt"].is_null(),
                "application replies are not deployment receipts"
            );
            assert_eq!(call_status["plan"]["call"]["canister"], child.to_text());
            let call_dir = dir.join("journal").join(call_id).join("call");
            let original_signed = std::fs::read(call_dir.join("signed.json")).unwrap();
            // Deliberately lose only local result custody after the real update committed.
            // A new command refuses; recovery observes the original request, leaving value at one.
            std::fs::remove_file(call_dir.join("reply.bin")).unwrap();
            assert!(
                !run(
                    &dir,
                    &[
                        "fleet",
                        "call",
                        "--canister",
                        "app",
                        "--identity",
                        "visitor",
                        "--icp-bin",
                        user_signer.to_str().unwrap(),
                        "increment"
                    ]
                )
                .status
                .success()
            );
            let recovered = cli(
                &dir,
                &[
                    "fleet",
                    "reconcile",
                    call_id,
                    "--identity",
                    "visitor",
                    "--icp-bin",
                    user_signer.to_str().unwrap(),
                ],
            );
            assert_eq!(recovered["call"]["operation"], call_id);
            assert_eq!(
                std::fs::read(call_dir.join("signed.json")).unwrap(),
                original_signed
            );
            assert_eq!(invoke("read")["result"]["Ok"]["value"], 1);
            for user in [alice, bob] {
                counter(owner.client(), child, user, Request::Increment)
                    .await
                    .unwrap();
            }
            let before = counter(owner.client(), child, bob, Request::Read)
                .await
                .unwrap();
            assert_eq!(before.value, 3);
            assert_eq!(before.executable, 1);
            std::fs::copy(assets.join("counter-v2.wasm"), dir.join("app.wasm")).unwrap();
            let publication = plan();
            assert_eq!(publication["action"], "publication");
            apply(publication["operation"].as_str().unwrap());
            let upgrade = plan();
            assert_eq!(upgrade["action"], "upgrade");
            let upgrade_id = upgrade["operation"].as_str().unwrap();
            apply(upgrade_id);
            advance(owner.client()).await;
            assert_eq!(apply(upgrade_id)["canister"], child.to_text());
            let after = counter(owner.client(), child, alice, Request::Read)
                .await
                .unwrap();
            assert_eq!(after.value, before.value);
            assert_eq!(after.fence, None);
            assert_eq!(after.executable, 2);
            assert_eq!(invoke("increment")["result"]["Ok"]["value"], 4);
            assert_eq!(plan()["action"], "unchanged");
        }
        // Store is independently controlled infrastructure; only one application was allocated.
        assert!(state.children[1].creation.approval.is_none());
        assert_eq!(
            owner
                .client()
                .get_subnet_metrics(owner.client().get_subnet(root).await.unwrap())
                .await
                .unwrap()
                .num_canisters,
            4
        );
        owner.client_mut().stop_live().await;
        assert_eq!(
            owner.close(Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        assert!(server.stop().unwrap().process.cleanup.is_empty());
    }
}
