//! Release-set discovery, artifact validation, and manifest emission.

mod application;
mod artifact;
mod config;
mod infrastructure;
mod manifest;
mod paths;

pub(crate) use application::APPLICATION_ARTIFACT_UNION_FILE;
pub use application::{
    ApplicationArtifactBuildOutput, ApplicationArtifactBuildTarget, ApplicationArtifactEntry,
    ApplicationArtifactFileBuildOutput, ApplicationArtifactUnion,
    ApplicationArtifactUnionPersistenceError, ApplicationReleaseSetEntryKind,
    ApplicationReleaseSetError, FleetSubnetRootReleaseSetEntry, FleetSubnetRootReleaseSetManifest,
    PersistedApplicationArtifactUnion, compile_and_persist_application_artifact_union,
    load_persisted_application_artifact_union,
};
use artifact::build_release_set_entry;
pub(crate) use artifact::resolve_release_artifact_path;
pub(crate) use artifact::validate_release_artifact_relative_path;
pub(crate) use config::configured_release_roles_from_config;
pub use config::{
    AppConfigDeclaration, AppConfigError, AppConfigIoOperation, AppConfigMutationConflict,
    AppConfigNameField, AppConfigNameIssue, AppConfigOperation, AppConfigPackageIssue,
    AppConfigSnapshot, AppConfigTomlOperation, AttachedAppRole, ConfiguredPoolExpectation,
    ConfiguredRoleLifecycle, DeclaredAppRole, RenamedAppRole, attach_app_role, declare_app_role,
    plan_attach_app_role, plan_declare_app_role, plan_rename_app_role, read_app_config_identity,
    rename_app_role,
};
pub(crate) use infrastructure::INFRASTRUCTURE_ARTIFACT_MANIFEST_FILE;
pub use infrastructure::{
    CanicInfrastructureArtifactBuildOutput, CanicInfrastructureArtifactEntry,
    CanicInfrastructureArtifactInput, CanicInfrastructureArtifactManifest,
    CanicInfrastructureArtifactManifestError, CanicInfrastructureArtifactPersistenceError,
    CanicInfrastructureRole, PersistedCanicInfrastructureArtifactManifest,
    compile_and_persist_canic_infrastructure_artifact_manifest,
    load_persisted_canic_infrastructure_artifact_manifest,
};
pub use manifest::{ReleaseSetEntry, RootReleaseSetManifest, load_root_release_set_manifest};
pub(crate) use manifest::{
    RootReleaseSetBuildSnapshot, RootReleaseSetBuildTarget,
    emit_root_release_set_manifest_from_build, validate_root_release_set_manifest,
};
pub use paths::{
    ArtifactRootError, WorkspaceDiscoveryError, app_sources_root, artifact_root_path, config_path,
    display_workspace_path, icp_root, load_root_package_version, load_workspace_package_version,
    resolve_artifact_root, resolve_artifact_root_path, root_release_set_manifest_path,
    workspace_manifest_path, workspace_root,
};

pub(super) const APP_SOURCES_ROOT_RELATIVE: &str = "apps";
pub(super) const ROOT_CONFIG_FILE: &str = "canic.toml";
pub(super) const WORKSPACE_MANIFEST_RELATIVE: &str = "Cargo.toml";
pub(crate) const ROOT_RELEASE_SET_MANIFEST_FILE: &str = "root.release-set.json";
pub(super) const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];
pub(super) const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];

pub(super) fn valid_package_name(package: &str) -> bool {
    const MAX_PACKAGE_BYTES: usize = 128;
    let bytes = package.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= MAX_PACKAGE_BYTES
        && bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[cfg(test)]
mod tests;
