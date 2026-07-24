#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::install_root) struct InstallPhaseLabel(&'static str);

impl InstallPhaseLabel {
    pub(in crate::install_root) const BUILD_ARTIFACTS: Self = Self("build_artifacts");
    pub(in crate::install_root) const EMIT_MANIFEST: Self = Self("emit_manifest");
    pub(in crate::install_root) const EXECUTION_PREFLIGHT: Self = Self("execution_preflight");
    pub(in crate::install_root) const FUND_ROOT_POST_READY: Self = Self("fund_root_post_ready");
    pub(in crate::install_root) const FUND_ROOT_PRE_BOOTSTRAP: Self =
        Self("fund_root_pre_bootstrap");
    pub(in crate::install_root) const INSTALL_ROOT: Self = Self("install_root");
    pub(in crate::install_root) const MATERIALIZE_ARTIFACTS: Self = Self("materialize_artifacts");
    pub(in crate::install_root) const PROMOTED_PLAN_INSTALL: Self = Self("promoted_plan_install");
    pub(in crate::install_root) const RESOLVE_ROOT_CANISTER: Self = Self("resolve_root_canister");
    pub(in crate::install_root) const RESUME_BOOTSTRAP: Self = Self("resume_bootstrap");
    pub(in crate::install_root) const STAGE_RELEASE_SET: Self = Self("stage_release_set");
    pub(in crate::install_root) const WAIT_READY: Self = Self("wait_ready");
    pub(in crate::install_root) const WRITE_INSTALL_STATE: Self = Self("write_install_state");

    #[must_use]
    pub(in crate::install_root) const fn as_str(self) -> &'static str {
        self.0
    }
}

pub(in crate::install_root) trait InstallPhaseOperation {
    fn phase(&self) -> InstallPhaseLabel;
    fn attempted_action(&self) -> &'static str;
    fn evidence(&self) -> Vec<String>;
    fn execute(&self) -> Result<(), Box<dyn std::error::Error>>;

    fn verified_evidence(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(self.evidence())
    }
}
