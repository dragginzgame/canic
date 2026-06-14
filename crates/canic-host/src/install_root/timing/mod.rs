use std::time::Duration;

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
