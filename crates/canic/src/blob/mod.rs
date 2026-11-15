//! Optional embedded Blob endpoint mounting. The application supplies a synchronous
//! Service handler, one Storage bootstrap and explicit lifecycle hooks. This module
//! owns no grant rules, catalog, service instance, timer or activation policy.
//!
//! Ingress handlers receive the actual caller unchanged. Only an application's
//! separately authorized local catalog operation may use its application authority.

/// The independent supplier; its DTOs and Service retain their original identities.
pub use ic_blob_storage as supplier;
/// Supplier-selected numeric bounds expressed in the adapter's Wire version.
/// The independent Blob service retains its own codec and persistence owner.
pub const RPC_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: supplier::protocol::RPC_LIMITS.bytes,
    allocation: supplier::protocol::RPC_LIMITS.allocation,
    nodes: supplier::protocol::RPC_LIMITS.nodes,
    depth: supplier::protocol::RPC_LIMITS.depth,
};
/// Wire identity for the captured-caller mutation endpoint.
pub const UPDATE: u64 = ic_wire::method_id("blob_update");
/// Wire identity for captured-caller inspection.
pub const QUERY: u64 = ic_wire::method_id("blob_query");
/// Replicated management observation, authorized by the supplier's configured manager.
pub const MANAGEMENT: u64 = ic_wire::method_id("blob_management");

#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub mod endpoint;

/// Mount the three Blob methods with synchronous application-supplied handlers.
///
/// Handlers receive `(captured_caller, command, captured_time)`,
/// `(captured_caller, inspection)` and `(captured_caller)` respectively. They call
/// the same local Service. No client DTO contains an authority principal.
/// Decode and encode failure trap; a synchronous handler's mutations therefore
/// share the IC message rollback boundary with its reply. No lifecycle is mounted.
///
/// ```ignore
/// canic::mount_blob_endpoints! {
///     execute = crate::execute_blob,
///     inspect = crate::inspect_blob,
///     management = crate::inspect_blob_management,
/// }
/// ```
#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! mount_blob_endpoints {
    (execute = $execute:path, inspect = $inspect:path, management = $management:path $(,)?) => {
        mod blob_update {
            use endpoint::{api, cdk};
            use $crate::blob::{
                UPDATE, endpoint,
                supplier::{
                    local::Error,
                    protocol::{Command, Outcome},
                },
            };
            #[cdk::update(
                crate = "cdk",
                guard = "endpoint::admit::<UPDATE>",
                decode_with = "endpoint::decode::<Command, UPDATE>",
                encode_with = "endpoint::encode::<Result<Outcome, Error>, UPDATE>"
            )]
            fn blob_update(input: Command) -> Result<Outcome, Error> {
                $execute(api::msg_caller(), input, api::time())
            }
        }
        mod blob_query {
            use endpoint::{api, cdk};
            use $crate::blob::{
                QUERY, endpoint,
                supplier::{
                    local::Error,
                    protocol::{Inspection, Observation},
                },
            };
            #[cdk::query(
                crate = "cdk",
                guard = "endpoint::admit::<QUERY>",
                decode_with = "endpoint::decode::<Inspection, QUERY>",
                encode_with = "endpoint::encode::<Result<Observation, Error>, QUERY>"
            )]
            fn blob_query(input: Inspection) -> Result<Observation, Error> {
                $inspect(api::msg_caller(), input)
            }
        }
        mod blob_management {
            use endpoint::{api, cdk};
            use $crate::blob::{
                MANAGEMENT, endpoint,
                supplier::{local::Error, protocol::ManagementView},
            };
            #[cdk::update(
                crate = "cdk",
                guard = "endpoint::admit::<MANAGEMENT>",
                decode_with = "endpoint::decode::<(), MANAGEMENT>",
                encode_with = "endpoint::encode::<Result<ManagementView, Error>, MANAGEMENT>"
            )]
            fn blob_management() -> Result<ManagementView, Error> {
                $management(api::msg_caller())
            }
        }
    };
}
