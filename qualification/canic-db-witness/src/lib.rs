//! Application-owned passive protocol. No IC DB executor or runtime in host clients.
mod canister;

/// Typed protocol for this witness application, not a general dynamic DB frontend.
pub mod protocol {
    use serde::{Deserialize, Serialize};
    /// Listing Wire identity.
    pub const QUERY: u64 = ic_wire::method_id("canic-db-witness.query.v1");
    /// Atomic insert Wire identity.
    pub const WRITE: u64 = ic_wire::method_id("canic-db-witness.write.v1");
    /// Fixed application query: ID order, residual minimum score.
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct QueryRequest {
        /// Minimum score, inclusive.
        pub minimum: u64,
        /// Requested results (1..=3).
        pub limit: u8,
        /// Candidates per message (1..=4).
        pub work: u8,
        /// Untrusted engine continuation, validated against this query.
        pub after: Option<Vec<u8>>,
    }
    /// Application record projection.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Row {
        /// Stable application-supplied identifier.
        pub id: u64,
        /// Queryable score.
        pub score: u64,
    }
    /// One bounded live page. Empty rows do not imply completion.
    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Page {
        /// Obtained rows.
        pub rows: Vec<Row>,
        /// Next logical position; absence establishes completion.
        pub next: Option<Vec<u8>>,
        /// Candidates processed by this request.
        pub examined: u32,
    }
    /// Expected application-facing refusals, without private engine details.
    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Refusal {
        /// Request exceeds supported application bounds.
        Limit,
        /// Insert conflicts with a stored primary key.
        Exists,
        /// Batch addresses a key more than once.
        DuplicateTarget,
        /// Database ordinary access is frozen.
        Maintenance,
        /// Continuation does not match this request/database.
        Cursor,
    }
    /// One page or a refusal; no client filtering or hidden retry.
    pub type QueryReply = Result<Page, Refusal>;
    /// At most four inserts, committed in one synchronous engine batch.
    pub type WriteRequest = Vec<Row>;
    /// Inserted count. Failure to encode even a successful reply traps the message.
    pub type WriteReply = Result<Receipt, Refusal>;
    /// Application write receipt.
    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Receipt {
        /// Inserted rows.
        pub inserted: u32,
        /// Application note (empty in the ordinary witness).
        pub note: String,
    }
}
