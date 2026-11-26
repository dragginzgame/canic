//! Optional embedded Blob endpoint mounting. The application supplies a synchronous
//! Service handler, one Storage bootstrap and explicit lifecycle hooks. This module
//! owns no grant rules, catalog, service instance, timer or activation policy.
//!
//! Ingress handlers receive the actual caller unchanged. Only an application's
//! separately authorized local catalog operation may use its application authority.

#[doc(hidden)]
pub mod endpoint;

/// The independent supplier; its DTOs and Service retain their original identities.
pub use ic_blob_storage as supplier;

use ic_platform_types::Principal;
use ic_storage::{Memory, RuntimeMemory};
use supplier::{
    local::persistence::{OpenError, Service},
    protocol::Installation,
};

impl crate::metrics::store::Source for supplier::metrics::MetricsStorage {
    type Storage = Self;
}

/// Attach a service to the application-granted lease with this canister’s selected diagnostics.
/// Supply installation settings only on fresh install. Reopen uses the existing record.
/// The application owns the service and its maintenance admission.
///
/// # Errors
/// Propagates the supplier's create or reopen refusal.
/// # Panics
/// Backing memory failures trap the lifecycle message.
pub fn open_service<M: Memory>(
    memory: RuntimeMemory<M>,
    installation: Principal,
    fresh: Option<Installation>,
) -> Result<Service<M>, OpenError> {
    let metrics = crate::metrics::store::open::<supplier::metrics::MetricsStorage>();
    let service = match fresh {
        Some(input) => Service::create_with_metrics(memory, installation, input, metrics),
        None => Service::open_with_metrics(memory, installation, metrics),
    }?;
    Ok(service.with_logging(crate::logging::record_blob))
}

/// Request and reply budgets defined by the Blob protocol.
pub use supplier::protocol::RPC_LIMITS;

/// Wire identity for the captured-caller mutation endpoint.
pub const UPDATE: u64 = ic_wire::method_id("blob_update");
/// Wire identity for captured-caller inspection.
pub const QUERY: u64 = ic_wire::method_id("blob_query");
/// Replicated management observation, authorized by the supplier's configured manager.
pub const MANAGEMENT: u64 = ic_wire::method_id("blob_management");

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
// Rustfmt 1.100 repeatedly reindents multiline attributes in this macro body.
#[rustfmt::skip]
#[macro_export]
macro_rules! mount_blob_endpoints {
    (execute = $execute:path, inspect = $inspect:path, management = $management:path $(,)?) => {
        $crate::__canic_report_mount!("canic:module:blob");
        const _: () = {
            use endpoint::api;
            use $crate::__private::runtime;
            use $crate::blob::{
                UPDATE, endpoint,
                supplier::{
                    local::Error,
                    protocol::{Command, Outcome},
                },
            };

            $crate::__canic_builtin! {
                wire, UPDATE, ($crate::blob::RPC_LIMITS, $crate::blob::RPC_LIMITS), runtime::update, "blob_update", (
                    runtime = runtime,
                    guard = endpoint::admit::<UPDATE>,
                    decode = endpoint::decode::<Command, UPDATE>,
                    max_bytes = 65536,
                    on_error = endpoint::fail,
                    encode = endpoint::encode::<Result<Outcome, Error>, UPDATE>,
                ),
                [] fn __canic_blob_update(input: Command) -> Result<Outcome, Error> {
                    $execute(api::caller(), input, api::time())
                }
            }
        };
        const _: () = {
            use endpoint::api;
            use $crate::__private::runtime;
            use $crate::blob::{
                QUERY, endpoint,
                supplier::{
                    local::Error,
                    protocol::{Inspection, Observation},
                },
            };

            $crate::__canic_builtin! {
                wire, QUERY, ($crate::blob::RPC_LIMITS, $crate::blob::RPC_LIMITS), runtime::query, "blob_query", (
                    runtime = runtime,
                    guard = endpoint::admit::<QUERY>,
                    decode = endpoint::decode::<Inspection, QUERY>,
                    max_bytes = 65536,
                    on_error = endpoint::fail,
                    encode = endpoint::encode::<Result<Observation, Error>, QUERY>,
                ),
                [] fn __canic_blob_query(input: Inspection) -> Result<Observation, Error> {
                    $inspect(api::caller(), input)
                }
            }
        };
        const _: () = {
            use endpoint::api;
            use $crate::__private::runtime;
            use $crate::blob::{
                MANAGEMENT, endpoint,
                supplier::{local::Error, protocol::ManagementView},
            };

            $crate::__canic_builtin! {
                wire, MANAGEMENT, ($crate::blob::RPC_LIMITS, $crate::blob::RPC_LIMITS), runtime::update, "blob_management", (
                    runtime = runtime,
                    guard = endpoint::admit::<MANAGEMENT>,
                    decode = endpoint::decode::<(), MANAGEMENT>,
                    max_bytes = 65536,
                    on_error = endpoint::fail,
                    encode = endpoint::encode::<Result<ManagementView, Error>, MANAGEMENT>,
                ),
                [] fn __canic_blob_management((): ()) -> Result<ManagementView, Error> {
                    $management(api::caller())
                }
            }
        };
    };
}
