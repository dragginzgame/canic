use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct InstallTimingLabel(&'static str);

impl InstallTimingLabel {
    pub(super) const BUILD_ALL: Self = Self("build_all");
    pub(super) const CREATE_CANISTERS: Self = Self("create_canisters");
    pub(super) const EMIT_MANIFEST: Self = Self("emit_manifest");
    pub(super) const FINALIZE_ROOT_FUNDING: Self = Self("finalize_root_funding");
    pub(super) const FUND_ROOT: Self = Self("fund_root");
    pub(super) const INSTALL_ROOT: Self = Self("install_root");
    pub(super) const RESUME_BOOTSTRAP: Self = Self("resume_bootstrap");
    pub(super) const STAGE_RELEASE_SET: Self = Self("stage_release_set");
    pub(super) const TOTAL: Self = Self("total");
    pub(super) const WAIT_READY: Self = Self("wait_ready");

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
    pub(super) create_canisters: Duration,
    pub(super) build_all: Duration,
    pub(super) emit_manifest: Duration,
    pub(super) install_root: Duration,
    pub(super) fund_root: Duration,
    pub(super) stage_release_set: Duration,
    pub(super) resume_bootstrap: Duration,
    pub(super) wait_ready: Duration,
    pub(super) finalize_root_funding: Duration,
}
