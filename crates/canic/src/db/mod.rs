//! Optional typed database mounts. The application owns its protocol, permissions,
//! schema, query construction and explicit storage/lifecycle hooks.
//! No database executor, registry or durable transport representation lives here.

#[cfg(all(test, any(feature = "db-store", feature = "db-schema")))]
pub(crate) mod test_store {
    use ic_db::design::prelude::*;

    #[canister]
    pub(crate) struct Owner {}

    #[store(canister = Owner, key = "test.rows")]
    pub(crate) struct Rows {}
}

/// A retained database in one application-owned named allocation.
#[cfg(feature = "db-store")]
pub mod store;

#[cfg(feature = "db")]
#[doc(hidden)]
pub mod endpoint;
/// Passive bounded inspection contract, usable independently of the database engine.
#[cfg(feature = "db")]
pub mod inspection;
/// Optional projection of compiled DB metadata; application authorization stays external.
#[cfg(feature = "db-schema")]
pub mod schema;

/// Fixed transport envelope, independent of the application's narrower work bounds.
#[cfg(feature = "db")]
pub const RPC_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 8 * 1024 * 1024,
    nodes: 65_536,
    depth: 32,
};
/// Surface presented to the application's synchronous pre-decode authorization.
#[cfg(feature = "db")]
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
#[cfg(feature = "db")]
// Rustfmt 1.100 repeatedly reindents multiline attributes in this macro body.
#[rustfmt::skip]
#[macro_export]
macro_rules! mount_db_endpoints {
    (protocol = $protocol:path, authorize = $authorize:path,
     inspect = $inspect:path, query = $query:path, write = $write:path $(,)?) => {
        $crate::mount_db_endpoints!(@method $protocol, $authorize, $inspect, Inspect,
            query, "db_inspect", INSPECT, InspectRequest, InspectReply);
        $crate::mount_db_endpoints!(@method $protocol, $authorize, $query, Query,
            query, "db_query", QUERY, QueryRequest, QueryReply);
        $crate::mount_db_endpoints!(@method $protocol, $authorize, $write, Write,
            update, "db_write", WRITE, WriteRequest, WriteReply, time);
    };
    (@method $protocol:path, $authorize:path, $handle:path, $operation:ident,
     $mode:ident, $name:literal, $id:ident, $request:ident, $reply:ident $(, $time:ident)?) => {
        const _: () = {
            use $protocol as __canic_protocol;
            use __canic_protocol::{$id, $request, $reply};
            use $crate::db::{Operation, endpoint};
            use endpoint::api;
            use $crate::__private::runtime;

            fn __canic_admit() -> Result<(), String> {
                $authorize(api::caller(), Operation::$operation)?;
                endpoint::admit::<$id>()
            }

            $crate::__canic_builtin! {
                wire, $id, runtime::$mode, $name, (
                    runtime = runtime,
                    guard = __canic_admit,
                    decode = endpoint::decode::<$request, $id>,
                    max_bytes = 65536,
                    on_error = endpoint::fail,
                    encode = endpoint::encode::<$reply, $id>,
                ),
                [] fn __canic_handle(request: $request) -> $reply {
                    $handle(api::caller(), request $(, api::$time())?)
                }
            }
        };
    };
}
