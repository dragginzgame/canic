use super::*;
use crate::{canister_build::CanisterArtifactBuilder, test_support::temp_dir};

#[test]
fn explicit_prepared_tools_reach_the_builder_with_their_admitted_identities() {
    let tools_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.tools/ic/bin");
    let ic_wasm = tools_root.join("ic-wasm");
    let wasm_opt = tools_root.join("wasm-opt");
    let tools = BuildToolchain::resolve_with_paths(
        CanisterBuildProfile::Release,
        Some(&ic_wasm),
        Some(&wasm_opt),
    )
    .unwrap();
    assert_eq!(tools.ic_wasm().path(), ic_wasm.canonicalize().unwrap());
    let optimizer = tools.binaryen().unwrap();
    assert_eq!(optimizer.path(), wasm_opt.canonicalize().unwrap());
    assert_eq!(
        optimizer.sha256(),
        current_binaryen_authority().unwrap().executable_sha256()
    );
    std::assert_matches!(
        tools.require_profile(CanisterBuildProfile::Fast),
        Err(BuildToolchainError::ProfileMismatch { .. })
    );
    let diagnostics = tools.diagnostic_lines();
    let builder = CanisterArtifactBuilder::with_toolchain(tools);
    assert_eq!(builder.diagnostic_lines(), diagnostics);
}

#[test]
fn explicit_tools_refuse_missing_paths_and_wrong_optimizer_bytes() {
    let root = temp_dir("explicit-build-tools");
    std::fs::create_dir_all(&root).unwrap();
    let missing = root.join("missing-ic-wasm");
    std::assert_matches!(
        BuildToolchain::resolve_with_paths(CanisterBuildProfile::Release, Some(&missing), None),
        Err(BuildToolchainError::IcWasm(
            IcWasmToolError::RequestedExecutableMissing { .. }
        ))
    );
    let ic_wasm = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.tools/ic/bin/ic-wasm");
    std::assert_matches!(
        BuildToolchain::resolve_with_paths(
            CanisterBuildProfile::Release,
            Some(&ic_wasm),
            Some(&missing),
        ),
        Err(BuildToolchainError::Binaryen(
            BinaryenToolError::NotExecutable { .. }
        ))
    );
    // A valid executable of the wrong identity must be refused, never executed as an optimizer.
    std::assert_matches!(
        BuildToolchain::resolve_with_paths(
            CanisterBuildProfile::Release,
            Some(&ic_wasm),
            Some(&ic_wasm),
        ),
        Err(BuildToolchainError::Binaryen(
            BinaryenToolError::ExecutableHashMismatch { .. }
        ))
    );
    for profile in [CanisterBuildProfile::Debug, CanisterBuildProfile::Fast] {
        let tools =
            BuildToolchain::resolve_with_paths(profile, Some(&ic_wasm), Some(&missing)).unwrap();
        assert!(tools.binaryen().is_none());
        tools.require_profile(profile).unwrap();
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn root_home_installation_warning_names_the_actual_directory() {
    let warning = root_home_warning_for(Some(OsStr::new("/")), Path::new("/.local/bin"))
        .expect("root HOME must warn");

    assert!(warning.contains("HOME resolves to `/`"));
    assert!(warning.contains("/.local/bin"));
    assert!(root_home_warning_for(Some(OsStr::new("/home/operator")), Path::new("/bin")).is_none());
}
