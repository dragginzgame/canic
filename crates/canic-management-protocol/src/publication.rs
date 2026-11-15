//! Controller-admitted Store publication and observation; Store owns release custody.
use canic_wasm_store_protocol as store;
use serde::{Deserialize, Serialize};

/// Execute one bounded Store operation; never admits deployment or cache work.
pub const EXECUTE: u64 = 46;
/// Last-dispatch window, independent of any child deployment approval.
pub const MAX_WINDOW_NS: u64 = 300_000_000_000;
/// One transient chunk plus target/deadline metadata; no durable Root payload.
pub const REQUEST_LIMITS: ic_wire::Limits = store::CHUNK_LIMITS;
/// Store metadata with an explicit transport-uncertainty result.
pub const RESPONSE_LIMITS: ic_wire::Limits = store::LIMITS;

/// Fixed publication operations, not arbitrary methods or management forwarding.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Command {
    /// Reserve an immutable manifest; exact replay returns existing progress.
    Prepare(store::Manifest),
    /// Supply one declared chunk; exact replay does not add another copy.
    Put(store::Put),
    /// Verify and publish all retained bytes; does not install anything.
    Publish(store::Release),
    /// Replicated progress observation, including after a lost publication reply.
    Observe(store::Release),
}

/// The authenticated controller selects the destination and exact command.
/// No caller principal or installation authority can be claimed in this DTO.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Request {
    /// Canonical Store principal. The Store independently requires Root control.
    pub store: String,
    /// No dispatch at or after this time, at most five minutes from admission.
    pub expires_at_ns: u64,
    /// One operation; Root never schedules or retries it.
    pub command: Command,
}

/// Failure distinguishes pre-dispatch refusal from an unknown remote outcome.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Error {
    /// Target, dispatch window or bounded request encoding is invalid; no call issued.
    Invalid,
    /// Dispatch authority expired; no call issued.
    Expired,
    /// Store returned its own typed refusal.
    Store(store::Error),
    /// Call/reply failed or mismatched; Store may have committed. Observe it.
    Uncertain,
}

/// Retained Store progress, not a deployment receipt or Root journal entry.
pub type Reply = Result<store::Status, Error>;

/// Observe initialized Store state through its actual Root controller.
pub const INFO: u64 = 47;
/// Bounded read-only Store observation; grants no bootstrap or deployment authority.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InfoRequest {
    /// Canonical Store identity, independent of any application child slot.
    pub store: String,
    /// Last dispatch time, at most five minutes from Root admission.
    pub expires_at_ns: u64,
}
/// Replicated Store state at observation time, not a controller or module certificate.
pub type InfoReply = Result<store::Info, Error>;
