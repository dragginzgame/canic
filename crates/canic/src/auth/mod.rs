//! Optional Auth receiver mounts. The application supplies synchronous handlers
//! backed by its actual Auth receiver and protected enrollment. Mounting grants
//! no authority, opens no readiness, allocates no storage and schedules no work.
//!
//! Caller and time are captured at the IC boundary. Credential fields cannot
//! supply either. Auth owns proof, session, replay and logout semantics; the
//! application owns current scope/resource permission and lifecycle composition.

/// Explicit, volatile single-grant enrollment. Does not mount or activate itself.
pub mod enrollment;

pub mod install;

/// Canic-owned issuer transport; application handlers own authentication and keys.
pub mod issuer;

#[doc(hidden)]
pub mod endpoint;

/// Stable enrolled session adapter with application-selected limits and role.
#[cfg(feature = "auth-enrollment")]
pub mod session;

/// Independent supplier contracts; numeric identities and directional budgets stay unchanged.
pub use ic_auth_wire as protocol;

/// Mount admission, session read and logout under the supplier's export names.
///
/// Handlers receive `(caller, input, time_ns)`, `(caller, time_ns)` and
/// `(caller, time_ns)`. They run synchronously in the same IC transaction as
/// reply encoding. Decoding/encoding failure traps. Ordinary Auth refusals
/// remain ordinary replies, preserving applicable failed-attempt charges.
///
/// This mounts no credential/issuer endpoint, enrollment or lifecycle hook.
/// The application must refuse a missing receiver checkpoint on upgrade;
/// fresh enrollment cannot reconstruct its replay or revocation evidence.
///
/// ```ignore
/// canic::mount_auth_endpoints! {
///     admit = crate::admit,
///     read = crate::read,
///     logout = crate::logout,
/// }
/// ```
// Rustfmt 1.100 repeatedly reindents multiline attributes in this macro body.
#[rustfmt::skip]
#[macro_export]
macro_rules! mount_auth_endpoints {
    (admit = $admit:path, read = $read:path, logout = $logout:path $(,)?) => {
        $crate::__canic_report_mount!("canic:module:auth");

        const _: () = {
            use endpoint::api;
            use $crate::__private::runtime;
            use $crate::auth::{
                endpoint,
                protocol::{EstablishSession, SessionInput, SessionView, model::AdmissionRefusal},
            };
            const ID: u64 = EstablishSession::ID;

            $crate::__canic_builtin! {
                auth, ID, (EstablishSession::REQUEST_LIMITS, EstablishSession::RESPONSE_LIMITS), runtime::update, "canic_establish_portable_session", (
                    runtime = runtime,
                    guard = endpoint::admit::<ID>,
                    decode = endpoint::decode::<SessionInput, ID>,
                    max_bytes = 2097152,
                    on_error = endpoint::fail,
                    encode = endpoint::encode::<Result<SessionView, AdmissionRefusal>, ID>,
                ),
                [] fn __canic_establish_portable_session(
                    input: SessionInput,
                ) -> Result<SessionView, AdmissionRefusal> {
                    $admit(api::caller(), input, api::time())
                }
            }
        };
        const _: () = {
            use endpoint::api;
            use $crate::__private::runtime;
            use $crate::auth::{
                endpoint,
                protocol::{ReadSession, SessionView, model::SessionReadRefusal},
            };
            const ID: u64 = ReadSession::ID;

            $crate::__canic_builtin! {
                auth, ID, (ReadSession::REQUEST_LIMITS, ReadSession::RESPONSE_LIMITS), runtime::query, "canic_portable_session_read", (
                    runtime = runtime,
                    guard = endpoint::admit::<ID>,
                    decode = endpoint::decode::<(), ID>,
                    max_bytes = 2097152,
                    on_error = endpoint::fail,
                    encode = endpoint::encode::<Result<SessionView, SessionReadRefusal>, ID>,
                ),
                [] fn __canic_portable_session_read((): ()) -> Result<SessionView, SessionReadRefusal> {
                    $read(api::caller(), api::time())
                }
            }
        };
        const _: () = {
            use endpoint::api;
            use $crate::__private::runtime;
            use $crate::auth::{
                endpoint,
                protocol::{Logout, model::LogoutResult},
            };
            const ID: u64 = Logout::ID;

            $crate::__canic_builtin! {
                auth, ID, (Logout::REQUEST_LIMITS, Logout::RESPONSE_LIMITS), runtime::update, "canic_logout_portable_session", (
                    runtime = runtime,
                    guard = endpoint::admit::<ID>,
                    decode = endpoint::decode::<(), ID>,
                    max_bytes = 2097152,
                    on_error = endpoint::fail,
                    encode = endpoint::encode::<LogoutResult, ID>,
                ),
                [] fn __canic_logout_portable_session((): ()) -> LogoutResult {
                    $logout(api::caller(), api::time())
                }
            }
        };
    };
}
