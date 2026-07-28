use super::*;

pub(in crate::deployment_truth::tests) const RELEASE_SET_USER_HUB_SHA256: &str =
    "771682dfb2e75fd9a27e7a55e7afed9cc0acaf8c1958e97828e0f1026cc3833e";

pub(in crate::deployment_truth::tests) fn assert_sha256_len(value: Option<&String>) {
    assert_eq!(value.map(String::len), Some(64));
}

pub(in crate::deployment_truth::tests) struct TempWorkspace {
    path: std::path::PathBuf,
}

impl TempWorkspace {
    pub(in crate::deployment_truth::tests) fn new(name: &str) -> Self {
        let path = temp_dir(name);
        fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }

    pub(in crate::deployment_truth::tests) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub(in crate::deployment_truth::tests) fn write_artifact(
    icp_root: &Path,
    role: &str,
    bytes: &[u8],
) {
    let path = icp_root
        .join(".icp")
        .join("local")
        .join("canisters")
        .join(role)
        .join(format!("{role}.wasm.gz"));
    fs::create_dir_all(path.parent().expect("artifact parent")).expect("create artifact dir");
    fs::write(path, bytes).expect("write artifact");
}

pub(in crate::deployment_truth::tests) fn write_release_set_manifest(icp_root: &Path) {
    let path = icp_root
        .join(".icp")
        .join("local")
        .join("canisters")
        .join("root")
        .join(ROOT_RELEASE_SET_MANIFEST_FILE);
    let manifest = serde_json::json!({
        "release_version": "0.41.1",
        "entries": [{
            "role": "user_hub",
            "template_id": "embedded:user_hub",
            "artifact_relative_path": ".icp/local/canisters/user_hub/user_hub.wasm.gz",
            "payload_size_bytes": 17,
            "payload_sha256_hex": RELEASE_SET_USER_HUB_SHA256,
            "chunk_size_bytes": 1_048_576,
            "chunk_sha256_hex": [RELEASE_SET_USER_HUB_SHA256]
        }]
    });
    fs::create_dir_all(path.parent().expect("manifest parent")).expect("create manifest dir");
    fs::write(
        path,
        serde_json::to_vec_pretty(&manifest).expect("encode manifest"),
    )
    .expect("write manifest");
}

pub(in crate::deployment_truth::tests) fn write_fleet_catalog_json(
    icp_root: &Path,
    environment: &str,
    fleet: FleetCatalogEntryV1,
) {
    let network = write_local_network_authority(icp_root, environment);
    let path = icp_root
        .join(".canic")
        .join("networks")
        .join(network.to_string())
        .join("fleets/catalog.json");
    fs::create_dir_all(path.parent().expect("catalog parent")).expect("create catalog dir");
    fs::write(
        path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 1,
            "canonical_network_id": network,
            "entries": [fleet],
        }))
        .expect("encode Fleet catalog"),
    )
    .expect("write Fleet catalog");
}

pub(in crate::deployment_truth::tests) fn sample_fleet_catalog_entry(
    fleet_name: &str,
    coordinator_principal: &str,
) -> FleetCatalogEntryV1 {
    FleetCatalogEntryV1 {
        canonical_network_id: test_local_network_id(),
        fleet_id: FleetId::from_generated_bytes([7; 32]),
        fleet_name: fleet_name.parse().expect("Fleet name"),
        app: AppId::from("demo"),
        environment: "local".to_string(),
        deployed_at_unix_secs: 1,
        coordinator_principal: coordinator_principal.to_string(),
    }
}

pub(in crate::deployment_truth::tests) fn write_local_network_authority(
    root: &Path,
    environment: &str,
) -> CanonicalNetworkId {
    crate::test_support::write_local_network_authority(root, environment)
}

fn test_local_network_id() -> CanonicalNetworkId {
    crate::test_support::local_network_id()
}
