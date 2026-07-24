use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct InstallTimingLabel(&'static str);

impl InstallTimingLabel {
    pub(super) const BUILD_ALL: Self = Self("build_all");
    pub(super) const CREATE_CANISTERS: Self = Self("create_canisters");
    pub(super) const EMIT_MANIFEST: Self = Self("emit_manifest");
    pub(super) const INSTALL_ROOT: Self = Self("install_root");
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
    pub(super) create_canisters: Duration,
    pub(super) build_all: Duration,
    pub(super) emit_manifest: Duration,
    pub(super) install_root: Duration,
}

impl InstallTimingSummary {
    pub(super) const fn record_activation(&mut self, activation: Self) {
        self.install_root = activation.install_root;
    }
}
