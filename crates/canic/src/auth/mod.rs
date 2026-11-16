//! Optional Auth receiver mounts. The application supplies synchronous handlers
//! backed by its actual Auth receiver and protected enrollment. Mounting grants
//! no authority, opens no readiness, allocates no storage and schedules no work.
//!
//! Caller and time are captured at the IC boundary. Credential fields cannot
//! supply either. Auth owns proof, session, replay and logout semantics; the
//! application owns current scope/resource permission and lifecycle composition.

/// Independent supplier contracts; numeric identities and directional budgets stay unchanged.
pub use ic_auth_wire as protocol;

/// Explicit, volatile single-grant enrollment. Does not mount or activate itself.
pub mod enrollment;

#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub mod endpoint;

/// Mount admission, session read and logout under the supplier's export names.
///
/// Handlers receive `(caller, input, time_ns)`, `(caller, time_ns)` and
/// `(caller, time_ns)`. They run synchronously in the same IC transaction as
/// reply encoding. Decoding/encoding failure traps. Ordinary Auth refusals
/// remain ordinary replies, preserving applicable failed-attempt charges.
///
/// This mounts no credential/issuer endpoint, enrollment or lifecycle hook.
/// The application must keep a lost volatile receiver closed after upgrade;
/// fresh enrollment cannot reconstruct its replay or revocation evidence.
///
/// ```ignore
/// canic::mount_auth_endpoints! {
///     admit = crate::admit,
///     read = crate::read,
///     logout = crate::logout,
/// }
/// ```
#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! mount_auth_endpoints {
    (admit = $admit:path, read = $read:path, logout = $logout:path $(,)?) => {
        mod establish_portable_session {
            use endpoint::{api, cdk};
            use $crate::auth::{
                endpoint,
                protocol::{EstablishSession, SessionInput, SessionView, model::AdmissionRefusal},
            };
            const ID: u64 = EstablishSession::ID;
            #[cdk::update(
                crate = "cdk",
                guard = "endpoint::admit::<ID>",
                decode_with = "endpoint::decode::<SessionInput, ID>",
                encode_with = "endpoint::encode::<Result<SessionView, AdmissionRefusal>, ID>"
            )]
            fn establish_portable_session(
                input: SessionInput,
            ) -> Result<SessionView, AdmissionRefusal> {
                $admit(api::msg_caller(), input, api::time())
            }
        }
        mod portable_session_read {
            use endpoint::{api, cdk};
            use $crate::auth::{
                endpoint,
                protocol::{ReadSession, SessionView, model::SessionReadRefusal},
            };
            const ID: u64 = ReadSession::ID;
            #[cdk::query(
                crate = "cdk",
                guard = "endpoint::admit::<ID>",
                decode_with = "endpoint::decode::<(), ID>",
                encode_with = "endpoint::encode::<Result<SessionView, SessionReadRefusal>, ID>"
            )]
            fn portable_session_read() -> Result<SessionView, SessionReadRefusal> {
                $read(api::msg_caller(), api::time())
            }
        }
        mod logout_portable_session {
            use endpoint::{api, cdk};
            use $crate::auth::{
                endpoint,
                protocol::{Logout, model::LogoutResult},
            };
            const ID: u64 = Logout::ID;
            #[cdk::update(
                crate = "cdk",
                guard = "endpoint::admit::<ID>",
                decode_with = "endpoint::decode::<(), ID>",
                encode_with = "endpoint::encode::<LogoutResult, ID>"
            )]
            fn logout_portable_session() -> LogoutResult {
                $logout(api::msg_caller(), api::time())
            }
        }
    };
}
