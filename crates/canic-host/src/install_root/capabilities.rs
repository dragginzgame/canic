use crate::deployment_truth::DeploymentExecutorCapabilityV1;

pub(super) const CURRENT_INSTALL_REQUIRED_CAPABILITIES: &[DeploymentExecutorCapabilityV1] = &[
    DeploymentExecutorCapabilityV1::CreateCanister,
    DeploymentExecutorCapabilityV1::CanisterStatus,
    DeploymentExecutorCapabilityV1::InstallCode,
    DeploymentExecutorCapabilityV1::Query,
];
