//! Host-owned continuation classification; recommendations grant no authority.
use super::{Phase, Status};

/// Recommended continuation of the original Store operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreContinuation {
    /// Observe an already-issued effect, including after expiry.
    RecoverIssued {
        /// The original effect window has expired.
        expired: bool,
    },
    /// Finish preservation verification without opening admission.
    VerifyCatalog {
        /// The original effect window has expired.
        expired: bool,
    },
    /// Retain completion evidence for the same operation.
    RetainCompletion,
    /// Close admission under the existing approval.
    Close,
    /// Finish the catalog scan and stop the Store.
    ScanAndStop,
    /// Explicitly install the approved replacement.
    Install,
    /// Continue the approved installation.
    ContinueInstallation,
    /// Start with admission closed.
    Start,
    /// Explicitly activate the verified attempt.
    Activate,
    /// Inspect the retained failure before proceeding.
    ReviewFailure,
    /// New effects are blocked by the original deadline.
    Expired,
    /// A missing host clock prevents a new-effect recommendation.
    ClockUnavailable,
}

pub(super) fn step(phase: Phase) -> StoreContinuation {
    match phase {
        Phase::Closing
        | Phase::StopIssued
        | Phase::UploadIssued
        | Phase::InstallIssued
        | Phase::StartIssued
        | Phase::ActivateIssued => StoreContinuation::RecoverIssued { expired: false },
        Phase::Verifying => StoreContinuation::VerifyCatalog { expired: false },
        Phase::Ready => StoreContinuation::RetainCompletion,
        Phase::Reserved => StoreContinuation::Close,
        Phase::Closed => StoreContinuation::ScanAndStop,
        Phase::Stopped => StoreContinuation::Install,
        Phase::Uploaded => StoreContinuation::ContinueInstallation,
        Phase::Installed => StoreContinuation::Start,
        Phase::Verified => StoreContinuation::Activate,
    }
}

/// Recommend a continuation from retained evidence, never new execution authority.
#[must_use]
pub fn store_continuation(status: &Status, now: Option<u128>) -> StoreContinuation {
    let next = step(status.phase);
    let expired = now.is_some_and(|now| now >= u128::from(status.approval.expires_at_ns));
    match next {
        StoreContinuation::RecoverIssued { .. } => StoreContinuation::RecoverIssued { expired },
        StoreContinuation::VerifyCatalog { .. } => StoreContinuation::VerifyCatalog { expired },
        StoreContinuation::RetainCompletion => next,
        _ if status.failure.is_some() => StoreContinuation::ReviewFailure,
        _ => match now {
            None => StoreContinuation::ClockUnavailable,
            Some(_) if expired => StoreContinuation::Expired,
            Some(_) => next,
        },
    }
}
