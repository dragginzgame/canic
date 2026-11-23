//! Passive database upgrade diagnostic for an application-authorized caller.
use serde::{Deserialize, Serialize};

/// Empty request for a single status observation.
pub type Request = ();

/// Wire method for the executable fingerprint and ordinary database admission.
pub const INSPECT: u64 = ic_wire::method_id("canic.db.inspect.v2");
/// Stored admission, separate from the executable schema identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Admission {
    /// Ordinary database open succeeded; this is its readable incarnation.
    Readable {
        /// Database incarnation, not a schema hash or row revision.
        incarnation: [u8; 16],
    },
    /// Ordinary reads are fenced by maintenance; no incarnation was read.
    Maintenance,
    /// The executable cannot admit this stored layout; bytes remain untouched.
    Incompatible,
    /// Missing, corrupt or unavailable state; no initialization or repair is implied.
    Unavailable,
}
/// One observation. Schema identity does not attest storage admission or integrity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Response {
    /// Executable compiled-schema identity.
    pub schema: [u8; 32],
    /// Independently obtained ordinary storage admission.
    pub admission: Admission,
}
