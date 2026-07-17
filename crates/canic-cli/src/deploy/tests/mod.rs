mod authority;
mod catalog;
mod compare;
mod deploy_check;
mod external_builders;
mod external_commands;
mod fixtures;
mod inspect;
mod install;
mod plan;
mod promote;
mod register;
mod root;
mod truth;

use super::*;

#[test]
fn deploy_preserves_icp_root_resolution_causes() {
    let error = DeployCommandError::from(IcpConfigError::NoIcpRoot {
        start: PathBuf::from("/project"),
    });

    assert_eq!(error.exit_code(), 2);
    std::assert_matches!(
        error,
        DeployCommandError::IcpRoot(IcpConfigError::NoIcpRoot { .. })
    );
}
