//! Real CLI, signed queries and strict reopening of the mounted database.
use super::{Client, Principal, row, wasm};
use ic_testkit::{
    pocket_ic::PocketIcBuilder,
    server::{Deletion, Limits, PreparedServer, Server},
};
use std::{process::Command, time::Duration};

#[tokio::test]
async fn signed_inspection_tracks_authority_and_admission_across_upgrades() {
    // Public synthetic test identity, never an application credential.
    let dir = tempfile::tempdir().unwrap();
    let key = dir.path().join("test-identity.pem");
    std::fs::write(
        &key,
        // Literal PKCS#8 encoding of the public synthetic [1; 32] seed below.
        "-----BEGIN PRIVATE KEY-----\n\
         MC4CAQAwBQYDK2VwBCIEIAEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEB\n\
         -----END PRIVATE KEY-----\n",
    )
    .unwrap();
    // DER public key corresponding to the fixed test seed, independently derived.
    let owner = Principal::self_authenticating([
        48, 42, 48, 5, 6, 3, 43, 101, 112, 3, 33, 0, 138, 136, 227, 221, 116, 9, 241, 149, 253, 82,
        219, 45, 60, 186, 93, 114, 202, 103, 9, 191, 29, 148, 18, 27, 243, 116, 136, 1, 180, 15,
        111, 92,
    ]);

    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(180),
            stdout_bytes: 1024 * 1024,
            stderr_bytes: 1024 * 1024,
        },
    )
    .unwrap();
    let mut instance = server
        .instance(
            PocketIcBuilder::new()
                .with_nns_subnet()
                .with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    let peer = instance.client_mut();
    let target = peer.create_canister().await;
    peer.add_cycles(target, 10_000_000_000_000).await;
    let normal = wasm("CANIC_DB_WASM");
    peer.install_canister(target, normal.clone(), owner.as_slice().to_vec(), None)
        .await;
    let client = Client {
        pic: peer,
        canister: target,
        owner,
    };
    client.write(&[row(10, 1), row(20, 2)]).await.unwrap();
    let url = peer.make_live(None).await;
    let root: String = peer
        .root_key()
        .await
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let subnet = peer.topology().await.get_nns().unwrap().to_text();
    let call = |extra: &[&str], anonymous: bool, trust: &str| {
        let mut cmd = Command::new(std::env::var_os("CANIC_CLI").expect("run make db-test"));
        cmd.args([
            "db",
            "inspect",
            "--network",
            "local",
            "--url",
            url.as_str(),
            "--root-key",
            trust,
            "--root-subnet",
            &subnet,
            "--routing-canister",
            &target.to_text(),
            "--canister",
            &target.to_text(),
        ]);
        if anonymous {
            cmd.arg("--anonymous");
        } else {
            cmd.arg("--identity").arg(&key);
        }
        cmd.args(extra).output().unwrap()
    };
    let before = peer.get_stable_memory(target).await;
    for (args, anonymous, trust) in [
        (&["--json"][..], true, root.as_str()),
        (&["--entity", "Missing", "--json"], false, root.as_str()),
        (&["--count", "--json"], false, root.as_str()),
        (&["--json"], false, "00"),
    ] {
        let refused = call(args, anonymous, trust);
        assert!(!refused.status.success());
        assert!(refused.stdout.is_empty());
    }
    let result = call(&["--json"], false, &root);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(json["caller"], owner.to_text());
    assert_eq!(json["inspection"]["schema"].as_array().unwrap().len(), 32);
    assert!(json["inspection"]["admission"]["Readable"]["incarnation"].is_array());
    let result = call(&[], false, &root);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let table = String::from_utf8(result.stdout).unwrap();
    assert_eq!(table.matches("query call (db_inspect)").count(), 1);
    assert!(table.contains("Executable schema") && table.contains("Readable"));
    assert_eq!(peer.get_stable_memory(target).await, before);
    peer.stop_live().await;
    for (image, state) in [
        (normal, "Readable"),
        (wasm("CANIC_DB_FAULT_WASM"), "Maintenance"),
        (wasm("CANIC_DB_INCOMPATIBLE_WASM"), "Incompatible"),
    ] {
        canic_qualification::upgrade_stopped(peer, target, image, vec![], None)
            .await
            .unwrap();
        if state == "Maintenance" {
            Client {
                pic: peer,
                canister: target,
                owner,
            }
            .control(2)
            .await;
        }
        let before = peer.get_stable_memory(target).await;
        peer.make_live(Some(url.port().unwrap())).await;
        let result = call(&["--json"], false, &root);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        let observed = &json["inspection"];
        if state == "Readable" {
            assert!(observed["admission"].get(state).is_some());
        } else {
            assert_eq!(observed["admission"], state);
        }
        peer.stop_live().await;
        assert_eq!(peer.get_stable_memory(target).await, before);
        if state == "Maintenance" {
            Client {
                pic: peer,
                canister: target,
                owner,
            }
            .control(3)
            .await;
        }
    }
    assert_eq!(
        instance.close(Duration::from_secs(10)).await,
        Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}
