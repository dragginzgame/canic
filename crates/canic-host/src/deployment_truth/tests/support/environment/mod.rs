use super::*;

pub(in crate::deployment_truth::tests) const SAMPLE_CONFIG: &str = r#"
controllers = []
[services.fleet]
roles = []

[app]
name = "demo"
init_mode = "enabled"


[roles.root]
kind = "root"
package = "root"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"
[app.whitelist]

[subnets.default.canisters.root]
kind = "root"

[subnets.default.canisters.user_hub]
kind = "service"
"#;

pub(in crate::deployment_truth::tests) struct LimitedExecutor {
    pub(in crate::deployment_truth::tests) context: DeploymentExecutionContextV1,
}

impl DeploymentExecutor for LimitedExecutor {
    fn execution_context(&self) -> DeploymentExecutionContextV1 {
        self.context.clone()
    }
}

pub(in crate::deployment_truth::tests) struct FixtureRootSubnetEvidenceSource {
    pub(in crate::deployment_truth::tests) result: Result<RootSubnetEvidence, String>,
}

impl RootSubnetEvidenceSource for FixtureRootSubnetEvidenceSource {
    fn root_subnet_evidence(
        &self,
        _environment: &str,
        _icp_root: &std::path::Path,
        _canister_id: &str,
    ) -> Result<RootSubnetEvidence, String> {
        self.result.clone()
    }
}
