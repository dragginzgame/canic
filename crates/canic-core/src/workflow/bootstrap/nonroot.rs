//! Non-root bootstrap workflows.

use crate::{log, log::Topic};

///
/// Bootstrap workflow for non-root canisters during init.
///
/// Environment and directory state are assumed to be initialized by
/// lifecycle adapters before this function is called.
///
/// Must be safe to retry if scheduling or execution is repeated.
///
/// `_args` are optional opaque bootstrap arguments forwarded from init.
/// Currently unused.
///
#[allow(clippy::unused_async)]
pub async fn bootstrap_init_nonroot_canister(_args: Option<Vec<u8>>) {
    log!(Topic::Init, Info, "non-root bootstrap: init start");

    // TODO:
    // - register with root if required
    // - start background tasks
    // - reconcile local state
    // - emit readiness signals

    log!(Topic::Init, Info, "non-root bootstrap: init complete");
}

///
/// Bootstrap workflow for non-root canisters after upgrade.
///
/// Must be safe to run multiple times.
///
#[allow(clippy::unused_async)]
pub async fn bootstrap_post_upgrade_nonroot_canister() {
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
