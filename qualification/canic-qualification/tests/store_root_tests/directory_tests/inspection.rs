//! Inspect the running fixture using a separate CLI process and the operator's query identity.
use super::*;
use std::process::Command;

pub(super) async fn second_terminal(
    pic: &mut PocketIc,
    root: CanisterId,
    operator: CanisterId,
    store: CanisterId,
    project: CanisterId,
) {
    let dir = tempfile::tempdir().unwrap();
    let pem = dir.path().join("fixture.pem");
    // Public synthetic PKCS#8 fixture seed [1; 32].
    std::fs::write(&pem, "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIAEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEB\n-----END PRIVATE KEY-----\n").unwrap();
    let url = pic.make_live(None).await;
    let network = canic_host::Network::Local {
        url: url.to_string(),
        routing_canister: root.to_text(),
        root_subnet: Some(pic.topology().await.get_nns().unwrap().to_text()),
        root_key: pic
            .root_key()
            .await
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    };
    let reader = canic_host::InspectionClient::new(&network, &root.to_text(), Some(&pem))
        .unwrap()
        .caller();
    pic.set_controllers(
        root,
        Some(operator),
        vec![operator, CanisterId::from_slice(reader.as_slice())],
    )
    .await
    .unwrap();
    let path = dir.path().join("canic.toml");
    let mut definition = canic_host::load_config(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/examples/counter/canic.toml"),
    )
    .unwrap();
    let app = definition.apps.get_mut("counter").unwrap();
    app.fleets.retain(|name, _| name == "local");
    let fleet = app.fleets.get_mut("local").unwrap();
    fleet.network = canic_host::FleetNetwork::Bound(network);
    fleet.lifetime = None;
    for (slot, id) in [("root", root), ("store", store)] {
        fleet.canisters.insert(
            slot.into(),
            canic_host::CanisterBinding { id: id.to_text() },
        );
    }
    canic_host::init_config(&path, &definition).unwrap();
    let call = |args: &[&str]| {
        let output = Command::new(std::env::var_os("CANIC_CLI").unwrap())
            .args(["fleet", "show", "--config"])
            .arg(&path)
            .arg("--identity")
            .arg(&pem)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    };
    let selected: serde_json::Value = serde_json::from_str(&call(&[
        "--json",
        "--canister",
        &project.to_text(),
        "--wasm",
        artifacts().join("directory.wasm").to_str().unwrap(),
    ]))
    .unwrap();
    assert!(
        selected["endpoints"]
            .as_array()
            .is_some_and(|methods| methods
                .iter()
                .any(|m| m["name"] == "canic_readiness" && m["mode"] == "query")),
        "{selected}"
    );
    assert_eq!(
        selected["diagnostics"]["readiness"]["reported"], "Ready",
        "{selected}"
    );
    assert!(selected["diagnostics"]["logs"]["observation_error"].is_string());
    let root_details: serde_json::Value =
        serde_json::from_str(&call(&["--json", "--canister", &root.to_text()])).unwrap();
    assert!(
        root_details["diagnostics"]["logs"]["reported"]["entries"].is_array(),
        "{root_details}"
    );
    pic.stop_canister(project, Some(root)).await.unwrap();
    let output = call(&["--json"]);
    let snapshot: serde_json::Value = serde_json::from_str(&output).unwrap();
    let canisters = snapshot["canisters"].as_array().unwrap();
    let target = |id: CanisterId| {
        canisters
            .iter()
            .find(|c| c["canister"] == id.to_text())
            .unwrap()
    };
    assert_eq!(target(project)["availability"], "stopped", "{output}");
    assert!(
        target(root)["availability"].get("running").is_some(),
        "{output}"
    );
    assert!(
        canisters
            .iter()
            .any(|c| c["installation"]["role"] == "project_ledger"
                && c["installation"]["parent"] == project.to_text())
    );
    let terminal = call(&[]);
    assert!(terminal.contains("project_ledger") && terminal.contains("stopped"));
    pic.stop_live().await;
}
