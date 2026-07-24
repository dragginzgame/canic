#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::install_root) struct InstallPhaseLabel(&'static str);

impl InstallPhaseLabel {
    pub(in crate::install_root) const BUILD_ARTIFACTS: Self = Self("build_artifacts");
    pub(in crate::install_root) const EMIT_MANIFEST: Self = Self("emit_manifest");
    pub(in crate::install_root) const EXECUTION_PREFLIGHT: Self = Self("execution_preflight");
    pub(in crate::install_root) const INSTALL_ROOT: Self = Self("install_root");
    pub(in crate::install_root) const MATERIALIZE_ARTIFACTS: Self = Self("materialize_artifacts");
    pub(in crate::install_root) const RESOLVE_ROOT_CANISTER: Self = Self("resolve_root_canister");

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

    fn execute_and_verify(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        self.execute()?;
        self.verified_evidence()
    }
}
