use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct InstallTimingLabel(&'static str);

impl InstallTimingLabel {
    pub(super) const ACTIVATE_FLEET: Self = Self("activate_fleet");
    pub(super) const BUILD_CONFIGURED: Self = Self("build_configured");
    pub(super) const BUILD_INFRASTRUCTURE: Self = Self("build_infrastructure");
    pub(super) const EMIT_MANIFEST: Self = Self("emit_manifest");
    pub(super) const MATERIALIZE_ARTIFACTS: Self = Self("materialize_artifacts");
    pub(super) const OTHER: Self = Self("planning_receipts_other");
    pub(super) const POST_BUILD_GATE: Self = Self("post_build_gate");
    pub(super) const PREFLIGHT: Self = Self("preflight");
    pub(super) const REUSE_ARTIFACTS: Self = Self("reuse_artifacts");
    pub(super) const TOTAL: Self = Self("total");

    #[must_use]
    pub(super) const fn as_str(self) -> &'static str {
        self.0
    }
}

///
/// InstallTimingSummary
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct InstallTimingSummary {
    pub(super) activate_fleet: Duration,
    pub(super) build_configured: Duration,
    pub(super) build_infrastructure: Duration,
    pub(super) emit_manifest: Duration,
    pub(super) materialize_artifacts: Duration,
    pub(super) post_build_gate: Duration,
    pub(super) preflight: Duration,
    pub(super) reuse_artifacts: Duration,
}
