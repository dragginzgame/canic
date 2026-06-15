use std::path::PathBuf;

pub(super) const ROOT_ROLE: &str = "root";
pub(super) const WASM_STORE_ROLE: &str = "wasm_store";
pub(super) const LOCAL_ARTIFACT_ROOT_RELATIVE: &str = ".icp/local/canisters";
pub(super) const WASM_TARGET: &str = "wasm32-unknown-unknown";

///
/// CanisterArtifactBuildOutput
///

#[derive(Clone, Debug)]
pub struct CanisterArtifactBuildOutput {
    pub artifact_root: PathBuf,
    pub wasm_path: PathBuf,
    pub wasm_gz_path: PathBuf,
    pub did_path: PathBuf,
    pub manifest_path: Option<PathBuf>,
}
