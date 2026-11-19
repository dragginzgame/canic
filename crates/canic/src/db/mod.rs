//! Optional typed database mounts. The application owns its protocol, permissions,
//! schema, query construction and explicit storage/lifecycle hooks.
//! No database executor, registry or durable transport representation lives here.

/// A retained database in one application-owned named allocation.
#[cfg(feature = "db-store")]
pub mod store;

/// Auth-enrolled stateful hub and shard lifecycle.
#[cfg(all(feature = "stateful-role", target_arch = "wasm32"))]
pub mod role;

#[cfg(feature = "db")]
#[doc(hidden)]
pub mod endpoint;
/// Passive bounded inspection contract, usable independently of the database engine.
pub mod inspection;
/// Optional projection of compiled DB metadata; application authorization stays external.
#[cfg(feature = "db-schema")]
pub mod schema;

/// Fixed transport envelope, independent of the application's narrower work bounds.
pub const RPC_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 8 * 1024 * 1024,
    nodes: 65_536,
    depth: 32,
};
/// Surface presented to the application's synchronous pre-decode authorization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    /// Application-selected schema/status information.
    Inspect,
    /// Application-defined bounded reads.
    Query,
    /// Application-defined atomic writes.
    Write,
}

/// Mount typed `db_inspect`, `db_query` and `db_write` methods.
///
/// `protocol` supplies passive `INSPECT`, `QUERY`, `WRITE` Wire IDs and corresponding
/// `InspectRequest/Reply`, `QueryRequest/Reply`, `WriteRequest/Reply` types. Those
/// contracts belong to the application; no arbitrary database query is exposed.
/// `authorize(caller, Operation)` returns `Result<(), String>` before payload copy.
/// Inspect/query handlers receive `(caller, request)`; write receives
/// `(caller, request, time)`. All handlers are synchronous; reply failure traps.
/// An allowed operation may still need row/tenant checks inside its typed handler.
/// Mounting performs no initialization, grants no authority and exports no lifecycle.
#[cfg(all(feature = "db", target_arch = "wasm32"))]
// Rustfmt 1.100 repeatedly reindents multiline attributes in this macro body.
#[rustfmt::skip]
#[macro_export]
macro_rules! mount_db_endpoints {
    (protocol = $protocol:path, authorize = $authorize:path,
     inspect = $inspect:path, query = $query:path, write = $write:path $(,)?) => {
        mod db_inspect {
            use endpoint::{api, runtime};
            use protocol::{INSPECT, InspectReply, InspectRequest};
            use $crate::db::{Operation, endpoint};
            use $protocol as protocol;

            /// Check application permission for this operation before copying its request.
            fn admit() -> Result<(), String> {
                $authorize(api::caller(), Operation::Inspect)?;
                endpoint::admit::<INSPECT>()
            }

            #[runtime::query(
                runtime = runtime,
                guard = admit,
                decode = endpoint::decode::<InspectRequest, INSPECT>,
                max_bytes = 65536,
                on_error = endpoint::fail,
                name = "db_inspect",
                encode = endpoint::encode::<InspectReply, INSPECT>,
            )]
            fn db_inspect(request: InspectRequest) -> InspectReply {
                $inspect(api::caller(), request)
            }
        }
        mod db_query {
            use endpoint::{api, runtime};
            use protocol::{QUERY, QueryReply, QueryRequest};
            use $crate::db::{Operation, endpoint};
            use $protocol as protocol;

            /// Check application permission for this operation before copying its request.
            fn admit() -> Result<(), String> {
                $authorize(api::caller(), Operation::Query)?;
                endpoint::admit::<QUERY>()
            }

            #[runtime::query(
                runtime = runtime,
                guard = admit,
                decode = endpoint::decode::<QueryRequest, QUERY>,
                max_bytes = 65536,
                on_error = endpoint::fail,
                name = "db_query",
                encode = endpoint::encode::<QueryReply, QUERY>,
            )]
            fn db_query(request: QueryRequest) -> QueryReply {
                $query(api::caller(), request)
            }
        }
        mod db_write {
            use endpoint::{api, runtime};
            use protocol::{WRITE, WriteReply, WriteRequest};
            use $crate::db::{Operation, endpoint};
            use $protocol as protocol;

            /// Check application permission for this operation before copying its request.
            fn admit() -> Result<(), String> {
                $authorize(api::caller(), Operation::Write)?;
                endpoint::admit::<WRITE>()
            }

            #[runtime::update(
                runtime = runtime,
                guard = admit,
                decode = endpoint::decode::<WriteRequest, WRITE>,
                max_bytes = 65536,
                on_error = endpoint::fail,
                name = "db_write",
                encode = endpoint::encode::<WriteReply, WRITE>,
            )]
            fn db_write(request: WriteRequest) -> WriteReply {
                $write(api::caller(), request, api::time())
            }
        }
    };
}
