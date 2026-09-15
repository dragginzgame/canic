use canic_core::ids::ReleaseBuildId;
use canic_host::canister_build::{
    CanisterBuildProfile, WorkspaceBuildContext, build_workspace_canister_artifact,
    copy_icp_wasm_output, print_workspace_build_context_once,
};
use canic_host::icp_config::resolve_icp_build_network_from_root;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let (
        Some(canister_name),
        Some(profile),
        Some(workspace_root),
        Some(icp_root),
        Some(config_path),
    ) = (
        args.next(),
        args.next(),
        args.next(),
        args.next(),
        args.next(),
    )
    else {
        return Err(
            "usage: cargo run -p canic-host --example build_artifact -- <canister-name> <debug|fast|release> <workspace-root> <icp-root> <config-path> [--environment <name>] [--refresh-canonical-did] [--release-build-id <id>]"
                .into(),
        );
    };
    let mut refresh_canonical_infrastructure_did = false;
    let mut release_build_id = None;
    let mut explicit_environment = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--environment" => {
                if explicit_environment.is_some() {
                    return Err("--environment may be supplied only once".into());
                }
                explicit_environment = Some(args.next().ok_or("--environment requires a name")?);
            }
            "--refresh-canonical-did"
                if matches!(canister_name.as_str(), "fleet_coordinator" | "wasm_store") =>
            {
                refresh_canonical_infrastructure_did = true;
            }
            "--refresh-canonical-did" => {
                return Err(
                    "--refresh-canonical-did requires fleet_coordinator or wasm_store".into(),
                );
            }
            "--release-build-id" => {
                if release_build_id.is_some() {
                    return Err("--release-build-id may be supplied only once".into());
                }
                let value = args
                    .next()
                    .ok_or("--release-build-id requires a canonical release-build ID")?;
                release_build_id = Some(value.parse::<ReleaseBuildId>()?);
            }
            _ => return Err("unknown build_artifact argument".into()),
        }
    }
    let profile = profile.parse::<CanisterBuildProfile>()?;

    let workspace_root = PathBuf::from(workspace_root).canonicalize()?;
    let icp_root = PathBuf::from(icp_root).canonicalize()?;
    let config_path = resolve_path(&workspace_root, &config_path).canonicalize()?;
    let environment = resolve_environment(
        explicit_environment.as_deref(),
        environment_variable("ICP_CLI_ENVIRONMENT")?.as_deref(),
        environment_variable("ICP_ENVIRONMENT")?.as_deref(),
    )?;
    let build_network = resolve_icp_build_network_from_root(&icp_root, &environment)?;
    let context = WorkspaceBuildContext {
        role: canister_name.clone(),
        profile,
        environment,
        build_network,
        config_path,
        workspace_root,
        icp_root,
        local_replica: None,
        refresh_canonical_infrastructure_did,
        release_build_id,
    };
    print_workspace_build_context_once(&context)?;
    let output = build_workspace_canister_artifact(&context)?;
    copy_icp_wasm_output(&canister_name, &output)?;
    println!("{}", output.wasm_gz_path.display());
    Ok(())
}

/// Reject ambiguous explicit authority; an inherited CLI default is not authority.
#[derive(Debug, thiserror::Error)]
enum BuildEnvironmentError {
    #[error("build environment must be a nonempty name without surrounding whitespace")]
    Invalid,
    #[error(
        "explicit build environment {explicit} conflicts with ICP-selected environment {selected}"
    )]
    Conflict { explicit: String, selected: String },
}

fn resolve_environment(
    explicit: Option<&str>,
    selected: Option<&str>,
    inherited_default: Option<&str>,
) -> Result<String, BuildEnvironmentError> {
    if let (Some(explicit), Some(selected)) = (explicit, selected)
        && explicit != selected
    {
        return Err(BuildEnvironmentError::Conflict {
            explicit: explicit.into(),
            selected: selected.into(),
        });
    }
    let environment = explicit
        .or(selected)
        .or(inherited_default)
        .unwrap_or("local");
    if environment.is_empty() || environment.trim() != environment {
        return Err(BuildEnvironmentError::Invalid);
    }
    Ok(environment.to_string())
}

fn environment_variable(name: &str) -> Result<Option<String>, std::env::VarError> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(error),
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

fn resolve_path(root: &Path, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icp_selected_environment_wins_over_inherited_default() {
        assert_eq!(
            resolve_environment(None, Some("proof"), None).unwrap(),
            "proof"
        );
        assert_eq!(
            resolve_environment(None, Some("proof"), Some("local")).unwrap(),
            "proof"
        );
        assert_eq!(
            resolve_environment(Some("proof"), Some("proof"), Some("local")).unwrap(),
            "proof"
        );
    }

    #[test]
    fn standalone_helper_accepts_explicit_or_cli_default_environment() {
        assert_eq!(
            resolve_environment(Some("proof"), None, Some("local")).unwrap(),
            "proof"
        );
        assert_eq!(resolve_environment(None, None, Some("ic")).unwrap(), "ic");
        assert_eq!(resolve_environment(None, None, None).unwrap(), "local");
    }

    #[test]
    fn rejects_conflicting_or_empty_selected_authority() {
        assert!(matches!(
            resolve_environment(Some("local"), Some("proof"), None),
            Err(BuildEnvironmentError::Conflict { .. })
        ));
        assert!(matches!(
            resolve_environment(None, Some(""), Some("local")),
            Err(BuildEnvironmentError::Invalid)
        ));
        assert!(matches!(
            resolve_environment(Some(" proof"), None, None),
            Err(BuildEnvironmentError::Invalid)
        ));
    }
}
