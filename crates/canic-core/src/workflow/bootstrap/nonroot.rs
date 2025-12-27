//! Non-root bootstrap workflows.

use crate::{log, log::Topic};

/// Bootstrap workflow for non-root canisters during init.
///
/// Environment and directory state are assumed to be initialized by
/// lifecycle adapters before this function is called.
#[allow(clippy::unused_async)]
pub async fn nonroot_init(_args: Option<Vec<u8>>) {
    log!(Topic::Init, Info, "non-root bootstrap: init start");

    // TODO:
    // - register with root if required
    // - start background tasks
    // - reconcile local state
    // - emit readiness signals

    log!(Topic::Init, Info, "non-root bootstrap: init complete");
}

/// Bootstrap workflow for non-root canisters after upgrade.
///
/// Must be safe to run multiple times.
#[allow(clippy::unused_async)]
pub async fn nonroot_post_upgrade() {
    log!(Topic::Init, Info, "non-root bootstrap: post-upgrade start");

    // TODO:
    // - resume timers
    // - reconcile state
    // - restart background tasks

    log!(
        Topic::Init,
        Info,
        "non-root bootstrap: post-upgrade complete"
    );
}
