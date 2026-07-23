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

pub(in crate::deployment_truth::tests) fn write_deployment_state_json(
    icp_root: &Path,
    environment: &str,
    state: InstallState,
) {
    let path = icp_root
        .join(".canic")
        .join(environment)
        .join("deployments")
        .join(format!("{}.json", state.deployment_name));
    fs::create_dir_all(path.parent().expect("state parent")).expect("create state dir");
    fs::write(
        path,
        serde_json::to_vec_pretty(&state).expect("encode install state"),
    )
    .expect("write install state");
}

pub(in crate::deployment_truth::tests) fn sample_install_state(
    deployment_name: &str,
    root_canister_id: &str,
) -> InstallState {
    InstallState {
        schema_version: 1,
        deployment_name: deployment_name.to_string(),
        fleet_template: "demo".to_string(),
        created_at_unix_secs: 1,
        updated_at_unix_secs: 1,
        environment: "local".to_string(),
        root_target: "root".to_string(),
        root_canister_id: root_canister_id.to_string(),
        root_verification: RootVerificationStatus::Verified,
        root_build_target: "root".to_string(),
        workspace_root: "/workspace".to_string(),
        icp_root: "/workspace".to_string(),
        config_path: "fleets/canic.toml".to_string(),
        release_set_manifest_path: ".icp/local/canisters/root/release-set.json".to_string(),
    }
}
