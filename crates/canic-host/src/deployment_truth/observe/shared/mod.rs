use super::super::*;
use crate::icp::{IcpCanisterStatusReport, IcpCli};
use std::path::Path;

pub(super) fn read_live_canister_status(
    icp_root: &Path,
    network: &str,
    canister_id: &str,
) -> Result<IcpCanisterStatusReport, crate::icp::IcpCommandError> {
    IcpCli::new("icp", Some(network.to_string()))
        .with_cwd(icp_root)
        .canister_status_report(canister_id)
}

pub(super) fn normalize_module_hash(hash: &str) -> String {
    hash.strip_prefix("0x")
        .or_else(|| hash.strip_prefix("0X"))
        .unwrap_or(hash)
        .to_ascii_lowercase()
}

pub(super) fn observation_gap(
    key: impl Into<String>,
    description: impl Into<String>,
) -> DeploymentObservationGapV1 {
    DeploymentObservationGapV1 {
        key: key.into(),
        description: description.into(),
    }
}
