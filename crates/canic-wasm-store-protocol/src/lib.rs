//! Immutable raw-Wasm releases and resumable publication. No deployment authority.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Prepare an exact release, reserving its capacity before upload.
pub const PREPARE: u64 = 100;
/// Supply one declared content-addressed chunk.
pub const PUT: u64 = 101;
/// Verify the complete bytes and make a release readable.
pub const PUBLISH: u64 = 102;
/// Read upload progress and immutable identity; also used by replicated `observe`.
pub const STATUS: u64 = 103;
/// Read one published chunk.
pub const READ: u64 = 104;
/// Page through release history, including incomplete releases.
pub const LIST: u64 = 105;
/// Read quotas and counters.
pub const INFO: u64 = 106;
/// Change quotas without replacing stored data.
pub const CONFIGURE: u64 = 107;
/// Retain exact published content for one caller-owned operation.
pub const PIN: u64 = 108;
/// Replicated observation of the exact retained pin.
pub const OBSERVE_PIN: u64 = 109;
/// Populate missing management-cache chunks from one pinned artifact.
pub const PREPARE_CACHE: u64 = 110;
/// Replicated release progress and all pins on its shared artifact; no payload scan.
pub const CUSTODY: u64 = 111;
/// Close, inspect or explicitly reopen Store mutation admission for one exact attempt.
pub const ADMISSION: u64 = 112;

/// Monotonic maintenance identity; a new attempt uses the previous sequence plus one.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MaintenanceAttempt {
    /// Nonzero sequence; never wraps or resets after activation.
    pub sequence: u64,
    /// Nonzero operation commitment supplied by the current controller.
    pub operation: [u8; 32],
}
/// Latest retained admission transition. Closed does not mean in-flight calls have drained.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Admission {
    /// Original attempt, retained after activation to fence stale requests.
    pub attempt: MaintenanceAttempt,
    /// Actual controller that closed admission; activation requires that same caller.
    #[serde(with = "serde_bytes")]
    pub owner: Vec<u8>,
    /// All publication, pin, quota and fresh cache work is refused while true.
    pub closed: bool,
}
/// Explicit local admission operations; no installation or automatic activation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdmissionCommand {
    /// Read the latest attempt, or None before the first closure.
    Observe,
    /// Close only the next sequence; exact replay returns its retained phase.
    Close(MaintenanceAttempt),
    /// Reopen only the exact current attempt owned by the captured caller.
    Activate(MaintenanceAttempt),
}
/// Bounded retained pin records per artifact; exhaustion never evicts a pin.
pub const MAX_PINS: usize = 32;

/// One immutable retention obligation, scoped by caller and artifact hash.
/// An operation may pin several artifacts, but cannot relabel an existing pin.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Pin {
    /// Digest of the caller's retained operation; nonzero and never inferred from “latest”.
    pub operation: [u8; 32],
    /// Exact published release, content and chunk layout.
    pub manifest: Manifest,
}

/// Durable retention evidence, independent of cache availability and deployment approval.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PinReceipt {
    /// Authenticated caller that owns this obligation.
    #[serde(with = "serde_bytes")]
    pub owner: Vec<u8>,
    /// Exact retained pin.
    pub pin: Pin,
}

/// One retained obligation on the artifact identified by a custody observation.
/// Labels may differ from the requested release when several versions share bytes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Retention {
    /// Actual principal captured when the pin was admitted.
    #[serde(with = "serde_bytes")]
    pub owner: Vec<u8>,
    /// Caller-owned operation identity.
    pub operation: [u8; 32],
    /// Exact release selected by that operation.
    pub release: Release,
}

/// One bounded observation, not an atomic catalog snapshot or maintenance approval.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Custody {
    /// Requested release's progress, including staged and unpublished content.
    pub status: Status,
    /// All retained pins on its artifact, at most `MAX_PINS`, in admission order.
    /// The manifest is shared rather than repeated for every pin.
    pub pins: Vec<Retention>,
}

/// One explicitly bounded, idempotent cache preparation request; never installs code.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PrepareCache {
    /// Must already be retained for this caller.
    pub pin: Pin,
    /// Last dispatch time, no more than five minutes from admission.
    pub expires_at_ns: u64,
}

/// Cache preparation evidence; cache availability must still hold at installation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CacheReady {
    /// Exact caller-owned artifact.
    pub pin: Pin,
    /// Number of missing chunks uploaded by this invocation.
    pub uploaded: u32,
}

/// Cache failures preserve pins and never authorize another installation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CacheError {
    /// Exact caller-owned pin is absent or mismatched.
    Pin(Error),
    /// Out-of-range chunk or invalid dispatch window.
    Invalid,
    /// Original dispatch deadline has passed.
    Expired,
    /// The caller is no longer a controller after an await.
    Authority,
    /// Management returned a definite rejection code.
    Rejected(u32),
    /// Delivery or reply interpretation was uncertain; re-observe cache before repair.
    Uncertain,
}
/// Transfer unit, independent of artifact size.
pub const CHUNK_BYTES: usize = 262_144;
/// First qualified verification work bound: 8 MiB per artifact.
pub const MAX_CHUNKS: usize = 32;
/// Maximum releases returned by one metadata page.
pub const PAGE_SIZE: usize = 4;
/// Small metadata and pagination frames.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 262_144,
    // Covers Wire traversal of four full manifests, including fixed hash arrays.
    nodes: 32_768,
    depth: 16,
};
/// One raw chunk plus its bounded envelope; Wire charges allocation per blob byte.
pub const CHUNK_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: CHUNK_BYTES + 4096,
    allocation: CHUNK_BYTES * 9 + 65_536,
    nodes: 4096,
    depth: 16,
};

/// Logical release identity; labels have no implicit ordering or “latest” behavior.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Release {
    /// Application-owned artifact name, at most 64 ASCII label bytes.
    pub name: String,
    /// Explicit version, at most 64 ASCII label bytes.
    pub version: String,
}
impl Release {
    /// Validate unambiguous printable labels.
    #[must_use]
    pub fn valid(&self) -> bool {
        [&self.name, &self.version].into_iter().all(|s| {
            !s.is_empty()
                && s.len() <= 64
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        })
    }
}

/// Exact uncompressed module identity. Publication does not approve installation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Manifest {
    /// Immutable logical release label.
    pub release: Release,
    /// SHA-256 of the complete raw Wasm, also its installed module identity.
    pub hash: [u8; 32],
    /// Complete byte length, including the Wasm header.
    pub bytes: u64,
    /// Ordered SHA-256 values; all but the final chunk have `CHUNK_BYTES` bytes.
    pub chunks: Vec<[u8; 32]>,
}
impl Manifest {
    /// Check identity and bounded transfer geometry before reserving storage.
    #[must_use]
    pub fn valid(&self) -> bool {
        self.release.valid()
            && self.bytes >= 8
            && self.bytes <= (CHUNK_BYTES * MAX_CHUNKS) as u64
            && self.chunks.len() == (self.bytes as usize).div_ceil(CHUNK_BYTES)
    }
}

/// One chunk belonging to an already prepared release.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Put {
    /// Exact prepared release.
    pub release: Release,
    /// Zero-based position in its ordered manifest.
    pub index: u32,
    /// Bytes checked against the declared size and digest.
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
}
/// A chunk read from a published release.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Read {
    /// Exact published release.
    pub release: Release,
    /// Zero-based position.
    pub index: u32,
}
/// Metadata-only resume information, not an installation receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Status {
    /// Exact retained identity.
    pub manifest: Manifest,
    /// All bytes verified in a committed publication message.
    pub published: bool,
    /// Positions whose bytes have not been committed yet.
    pub missing: Vec<u32>,
}
/// Read response preserves the blob codec and avoids per-byte node admission.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Chunk(#[serde(with = "serde_bytes")] pub Vec<u8>);
/// First catalog limits are payload reservations and logical release count.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Quota {
    /// Sum of unique artifact lengths, including incomplete uploads; excludes metadata.
    pub reserved_bytes: u64,
    /// Count includes incomplete releases and every retained version.
    pub releases: u64,
}
impl Default for Quota {
    fn default() -> Self {
        Self {
            reserved_bytes: 128 * 1024 * 1024,
            releases: 1024,
        }
    }
}
impl Quota {
    /// First backing's finite operational ceilings; physical allocation has its own limit.
    #[must_use]
    pub fn valid(&self) -> bool {
        self.reserved_bytes > 0
            && self.reserved_bytes <= 256 * 1024 * 1024
            && self.releases > 0
            && self.releases <= 4096
    }
}
/// Current quota usage and executable witness.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Info {
    /// Configured admission ceilings.
    pub quota: Quota,
    /// Conservatively reserved payload bytes; chunk deduplication does not release reservations.
    pub reserved_bytes: u64,
    /// All retained release labels.
    pub releases: u64,
    /// Unique module identities.
    pub artifacts: u64,
    /// Committed distinct chunk payloads.
    pub chunks: u64,
    /// Distinct binary used only to qualify maintenance.
    pub maintenance_candidate: bool,
}
/// Typed refusal without partial mutation. Backing/codec failures trap instead.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Error {
    /// Malformed identity, geometry or unsupported raw Wasm framing.
    Invalid,
    /// Release or content identity already has different commitments.
    Conflict,
    /// No matching prepared release.
    NotFound,
    /// Upload is incomplete or not yet published.
    Incomplete,
    /// Chunk or full-payload digest mismatch.
    Hash,
    /// Finite quota cannot admit this request.
    Capacity,
    /// Store mutation admission is closed; reads and exact maintenance recovery remain available.
    Closed,
}
/// Canonical content hashing, shared by producer and Store.
#[must_use]
pub fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maximum_metadata_page_and_chunk_fit_independent_wire_budgets() {
        let manifest = Manifest {
            release: Release {
                name: "a".repeat(64),
                version: "v".repeat(64),
            },
            hash: [255; 32],
            bytes: (CHUNK_BYTES * MAX_CHUNKS) as u64,
            chunks: vec![[255; 32]; MAX_CHUNKS],
        };
        assert!(manifest.valid());
        let pin = Pin {
            operation: [255; 32],
            manifest: manifest.clone(),
        };
        let receipt = Ok::<_, Error>(PinReceipt {
            owner: vec![255; 29],
            pin: pin.clone(),
        });
        let encoded = ic_wire::native::encode_response(PIN, &receipt, LIMITS).unwrap();
        assert_eq!(
            ic_wire::native::decode_response::<Result<PinReceipt, Error>>(PIN, &encoded, LIMITS)
                .unwrap(),
            receipt
        );
        let request = PrepareCache {
            pin,
            expires_at_ns: u64::MAX,
        };
        let encoded = ic_wire::native::encode_request(PREPARE_CACHE, &request, LIMITS).unwrap();
        assert_eq!(
            ic_wire::native::decode_request::<PrepareCache>(PREPARE_CACHE, &encoded, LIMITS)
                .unwrap(),
            request
        );
        let status = Status {
            manifest: manifest.clone(),
            published: false,
            missing: (0..MAX_CHUNKS as u32).collect(),
        };
        let custody = Ok::<_, Error>(Custody {
            status: Status {
                published: true,
                missing: vec![],
                ..status.clone()
            },
            pins: (0..MAX_PINS)
                .map(|i| Retention {
                    owner: vec![255; 29],
                    operation: [i as u8 + 1; 32],
                    release: manifest.release.clone(),
                })
                .collect(),
        });
        let encoded = ic_wire::native::encode_response(CUSTODY, &custody, LIMITS).unwrap();
        assert_eq!(
            ic_wire::native::decode_response::<Result<Custody, Error>>(CUSTODY, &encoded, LIMITS)
                .unwrap(),
            custody
        );
        let page = Ok::<_, Error>(vec![status; PAGE_SIZE]);
        let encoded = ic_wire::native::encode_response(LIST, &page, LIMITS).unwrap();
        assert_eq!(
            ic_wire::native::decode_response::<Result<Vec<Status>, Error>>(LIST, &encoded, LIMITS)
                .unwrap(),
            page
        );
        let input = Put {
            release: manifest.release,
            index: 0,
            bytes: vec![255; CHUNK_BYTES],
        };
        let encoded = ic_wire::native::encode_request(PUT, &input, CHUNK_LIMITS).unwrap();
        assert_eq!(
            ic_wire::native::decode_request::<Put>(PUT, &encoded, CHUNK_LIMITS)
                .unwrap()
                .bytes
                .len(),
            CHUNK_BYTES
        );
        assert!(ic_wire::native::decode_request::<Put>(PUT, &encoded, LIMITS).is_err());
        let response = Ok::<_, Error>(Chunk(input.bytes));
        let encoded = ic_wire::native::encode_response(READ, &response, CHUNK_LIMITS).unwrap();
        assert_eq!(
            ic_wire::native::decode_response::<Result<Chunk, Error>>(READ, &encoded, CHUNK_LIMITS)
                .unwrap(),
            response
        );
    }
}
