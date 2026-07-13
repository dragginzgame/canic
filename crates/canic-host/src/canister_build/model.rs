use std::path::PathBuf;

pub(super) const ROOT_ROLE: &str = "root";
pub(super) const WASM_STORE_ROLE: &str = "wasm_store";
pub(super) const LOCAL_ARTIFACT_ROOT_RELATIVE: &str = ".icp/local/canisters";
pub(super) const WASM_TARGET: &str = "wasm32-unknown-unknown";

/// Exact package and output paths admitted before one role build starts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanisterArtifactBuildSpec {
    pub(crate) role: String,
    pub(crate) package_name: String,
    pub(crate) package_manifest_path: PathBuf,
    pub(crate) artifact_root: PathBuf,
    pub(crate) wasm_path: PathBuf,
    pub(crate) wasm_gz_path: PathBuf,
    pub(crate) did_path: PathBuf,
}

///
/// CanisterArtifactBuildOutput
///

#[derive(Clone, Debug)]
pub struct CanisterArtifactBuildOutput {
    pub artifact_root: PathBuf,
    pub wasm_path: PathBuf,
    pub wasm_gz_path: PathBuf,
    pub did_path: PathBuf,
}

/// One successful role output from the current complete-build invocation.
#[derive(Clone, Debug)]
pub struct CurrentCanisterArtifactBuildOutput {
    pub(crate) role: String,
    pub(crate) output: CanisterArtifactBuildOutput,
}
