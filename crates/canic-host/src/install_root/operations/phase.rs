#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::install_root) struct InstallPhaseLabel(&'static str);

impl InstallPhaseLabel {
    pub(in crate::install_root) const BUILD_ARTIFACTS: Self = Self("build_artifacts");
    pub(in crate::install_root) const EMIT_MANIFEST: Self = Self("emit_manifest");
    pub(in crate::install_root) const EXECUTION_PREFLIGHT: Self = Self("execution_preflight");
    pub(in crate::install_root) const MATERIALIZE_ARTIFACTS: Self = Self("materialize_artifacts");

    #[must_use]
    pub(in crate::install_root) const fn as_str(self) -> &'static str {
        self.0
    }
}
