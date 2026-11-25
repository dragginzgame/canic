//! Controller-admitted Store publication and observation; Store owns release custody.
use ic_wasm_store_protocol as store;
use serde::{Deserialize, Serialize};

/// Execute one bounded Store operation; never admits deployment or cache work.
pub const EXECUTE: u64 = ic_wire::method_id("canic.root.publish_store_artifact.v1");
/// Last-dispatch window, independent of any child deployment approval.
pub const MAX_WINDOW_NS: u64 = 300_000_000_000;
/// One transient chunk plus target/deadline metadata; no durable Root payload.
pub const REQUEST_LIMITS: ic_wire::Limits = crate::stored::STORE_CHUNK_LIMITS;
/// Store metadata with an explicit transport-uncertainty result.
pub const RESPONSE_LIMITS: ic_wire::Limits = crate::stored::STORE_LIMITS;

/// Fixed publication operations, not arbitrary methods or management forwarding.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Command {
    /// Reserve an immutable manifest; exact replay returns existing progress.
    Prepare(store::Manifest),
    /// Supply one declared chunk; exact replay does not add another copy.
    Put(store::Put),
    /// Verify and publish all retained bytes; does not install anything.
    Publish(store::Release),
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
    /// Root has retained exclusive Store maintenance; no publication/admission mutation issued.
    Maintenance,
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

/// Observe a release and every pin on its artifact through the actual Store controller.
pub const CUSTODY: u64 = 48;
/// Bounded preservation evidence, without granting maintenance or pin authority.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CustodyRequest {
    /// Canonical Store principal; Store independently checks Root control.
    pub store: String,
    /// Last dispatch time, at most five minutes from Root admission.
    pub expires_at_ns: u64,
    /// Exact existing release, including an incomplete upload.
    pub release: store::Release,
}
/// One replicated metadata observation, not a certificate of installed code or full catalog.
pub type CustodyReply = Result<store::Custody, Error>;

/// Read-only Root proxy for Store metadata and release custody.
pub const OBSERVATION: u64 = ic_wire::method_id("canic.root.observe_store.v1");
/// Select one Store and its original dispatch deadline.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ObservationRequest {
    /// Inspect initialized Store quotas and usage.
    Info(InfoRequest),
    /// Inspect the named release and all pins on its artifact.
    Custody(CustodyRequest),
    /// Read publication progress for the named release.
    ReleaseStatus(CustodyRequest),
    /// Read the latest Store admission attempt.
    Admission(InfoRequest),
}
/// Evidence returned by the selected read.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Observation {
    /// Initialized Store report.
    Info(store::Info),
    /// Release status and retention obligations.
    Custody(store::Custody),
    /// Publication progress bound to the original release.
    ReleaseStatus(store::Status),
    /// Latest Store admission, absent until the first closure.
    Admission(Option<store::Admission>),
}
/// Successful typed evidence or a bounded Store failure.
pub type ObservationReply = Result<Observation, Error>;

/// Explicit Store-local admission transition through its controller; never installs code.
pub const ADMISSION: u64 = ic_wire::method_id("canic.root.change_store_admission.v1");

/// Explicit transitions of the Store mutation gate.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AdmissionChange {
    /// Close under this exact attempt.
    Close(store::MaintenanceAttempt),
    /// Reopen under this exact attempt.
    Activate(store::MaintenanceAttempt),
}
/// Root forwards the controller’s selected admission transition.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdmissionRequest {
    /// Canonical Store principal.
    pub store: String,
    /// Last dispatch time, at most five minutes from Root admission.
    pub expires_at_ns: u64,
    /// Exact Store transition and attempt.
    pub command: AdmissionChange,
}
/// Store's retained admission phase; no quiescence or installation receipt.
pub type AdmissionReply = Result<Option<store::Admission>, Error>;

/// One exact removal through Root; no retry, restoration or pin-release authority.
pub const REMOVE: u64 = ic_wire::method_id("canic.root.remove_store_artifact.v1");
/// One full manifest plus target/deadline; no transient Wasm bytes.
pub const REMOVAL_LIMITS: ic_wire::Limits = RESPONSE_LIMITS;
/// Published deletion and incomplete-upload cancellation have different supplier contracts.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Removal {
    /// Delete a published, unpinned release, retaining its immutable tombstone.
    Delete,
    /// Cancel an incomplete, unpinned upload, retaining its immutable tombstone.
    Cancel,
}
/// The caller must retain one exact signed attempt, including this original deadline.
/// Before restoration, settle every removal and let its dispatch authority expire.
/// A catalog observation is not settlement, and a fresh ingress is not a retry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RemovalRequest {
    /// Canonical Store identity, independently checked by Store against actual Root control.
    pub store: String,
    /// Original last-dispatch time; never renewed to replay removal after restoration.
    pub expires_at_ns: u64,
    /// Exact supplier action, never inferred from current upload progress.
    pub action: Removal,
    /// Full immutable identity, including size and ordered chunk commitments.
    pub manifest: store::Manifest,
}
/// Success acknowledges this completed remote removal. Missing/invalid replies remain uncertain.
/// This does not prove other attempts settled or allow restoration before their deadlines expire.
pub type RemovalReply = Result<store::Manifest, Error>;
