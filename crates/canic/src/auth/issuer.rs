//! Bounded issuer transport without an issuer engine, key store or lifecycle hooks.
//! Initial issuance/lookup captures the authenticated account caller. Renewal
//! and its lookup must authenticate the browser against the supplier's retained
//! login; a request's account is only a selector. Missing evidence is unresolved.
use super::protocol::model::{DelegatedRoleGrant, PortableCredential};
use ic_auth_protocol_types::Principal;
use ic_wire::{Limits, List};
use serde::{Deserialize, Serialize};

/// Account-authenticated initial issuance.
pub const ISSUE: u64 = 0x4341_5554_4810;
/// Browser-authenticated renewal of one retained login.
pub const RENEW: u64 = 0x4341_5554_4811;
/// Lookup only, under the original account or browser signer role.
pub const RECONCILE: u64 = 0x4341_5554_4812;
/// Browser-bound revocation of the original issuer login, distinct from receiver logout.
pub const LOGOUT: u64 = 0x4341_5554_4813;
/// Selected issuer input ceiling; larger supplier grant sets are not admitted here.
pub const REQUEST_LIMITS: Limits = Limits {
    bytes: 8192,
    allocation: 131072,
    nodes: 4096,
    depth: 12,
};
/// Complete original credential, without rewriting its canonical claims or proof.
pub const RESPONSE_LIMITS: Limits = Limits {
    bytes: 131072,
    allocation: 1048576,
    nodes: 16384,
    depth: 16,
};

/// Untrusted initial intent. No account, policy, issuer key or deadline is accepted.
#[cfg_attr(feature = "auth-browser", derive(ic_wire::typescript::Wire))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Issue {
    /// Original operation epoch.
    pub epoch: u64,
    /// Exact operation identity within the authenticated account.
    pub operation_id: [u8; 32],
    /// Original login epoch.
    pub login_epoch: u64,
    /// Exact login identity within the authenticated account.
    pub login_id: [u8; 32],
    /// Browser key to bind; these bytes do not prove possession.
    pub browser_public_key: [u8; 32],
    /// Requested grants, requiring current application permission.
    pub grants: List<DelegatedRoleGrant, 16>,
}
/// Browser-authenticated selection. Naming an account grants no account authority.
#[cfg_attr(feature = "auth-browser", derive(ic_wire::typescript::Wire))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Renew {
    /// Original account, verified through the retained browser/login binding.
    pub account: Principal,
    /// Original login epoch, never the current maintenance epoch.
    pub login_epoch: u64,
    /// Original login identity.
    pub login_id: [u8; 32],
    /// Exact next or retained renewal sequence.
    pub sequence: u64,
    /// Original grants; renewal must not widen them.
    pub grants: List<DelegatedRoleGrant, 16>,
}
/// Select the original login. Only the captured caller authenticates its browser.
#[cfg_attr(feature = "auth-browser", derive(ic_wire::typescript::Wire))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Logout {
    /// Original account; this field never authenticates an account call.
    pub account: Principal,
    /// Original login epoch.
    pub login_epoch: u64,
    /// Original login identity.
    pub login_id: [u8; 32],
}
/// Local issuer revocation only. Other receivers still need explicit cleanup.
#[cfg_attr(feature = "auth-browser", derive(ic_wire::typescript::Wire))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Revoked {
    /// Unchanged original deadline, not proof of expiry or logout at a receiver.
    pub login_deadline_ns: u64,
}
/// Select lookup under the original signer role; never fall back to issuance.
#[cfg_attr(feature = "auth-browser", derive(ic_wire::typescript::Wire))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Lookup {
    /// Authenticated account selects its exact initial operation.
    Issue {
        /// Original operation epoch.
        epoch: u64,
        /// Original operation identity.
        operation_id: [u8; 32],
    },
    /// Authenticated browser selects its exact retained renewal.
    Renew(Renew),
}
/// Retained evidence, not permission to issue or extend any lifetime.
#[cfg_attr(feature = "auth-browser", derive(ic_wire::typescript::Wire))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Status {
    /// Missing, pruned, pending or uncertain evidence; never proven no effect.
    Unresolved,
    /// Original canonical bytes and original exclusive admission deadline.
    Issued {
        /// Complete supplier credential, still subject to receiver verification.
        credential: PortableCredential,
        /// Original deadline; retrieval does not renew it.
        admission_deadline_ns: u64,
    },
}
/// Bounded transport refusals; ordinary refusals retain applicable work charges.
#[cfg_attr(feature = "auth-browser", derive(ic_wire::typescript::Wire))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Refusal {
    /// Malformed semantic input.
    Invalid,
    /// Account, browser, grants or current authority refused.
    Denied,
    /// Exact retained request or epoch conflicts.
    Conflict,
    /// Original finite window elapsed.
    Expired,
    /// Retained capacity exhausted.
    Capacity,
    /// Fresh work credit exhausted.
    Throttled,
    /// Closed owner, clock refusal or unavailable effect evidence.
    Unavailable,
}

/// Mount issuer calls without selecting an engine, key or application policy.
/// Issue/renew handlers are async; they must reserve before awaiting, recheck
/// permission/time on completion and never redispatch retained uncertainty.
/// Reconciliation is synchronous and lookup-only. Every handler receives the
/// actual caller and ingress time. Reply encoding traps its final transaction;
/// earlier async commits remain retained. Volatile loss must stay closed.
#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! mount_auth_issuer_endpoints {
    (issue = $issue:path, renew = $renew:path, reconcile = $reconcile:path,
     logout = $logout:path $(,)?) => {
        mod issue_portable_credential {
            use endpoint::{api, runtime};
            use $crate::auth::{
                endpoint,
                issuer::{ISSUE, Issue, Refusal, Status},
            };
            #[runtime::update(runtime = runtime, guard = endpoint::admit::<ISSUE>, decode = endpoint::decode::<Issue, ISSUE>, max_bytes = 8192, on_error = endpoint::fail, name = "issue_portable_credential", encode = endpoint::encode::<Result<Status, Refusal>, ISSUE>)]
            async fn issue_portable_credential(input: Issue) -> Result<Status, Refusal> {
                $issue(api::caller(), input, api::time()).await
            }
        }
        mod renew_portable_credential {
            use endpoint::{api, runtime};
            use $crate::auth::{
                endpoint,
                issuer::{RENEW, Refusal, Renew, Status},
            };
            #[runtime::update(runtime = runtime, guard = endpoint::admit::<RENEW>, decode = endpoint::decode::<Renew, RENEW>, max_bytes = 8192, on_error = endpoint::fail, name = "renew_portable_credential", encode = endpoint::encode::<Result<Status, Refusal>, RENEW>)]
            async fn renew_portable_credential(input: Renew) -> Result<Status, Refusal> {
                $renew(api::caller(), input, api::time()).await
            }
        }
        mod reconcile_portable_credential {
            use endpoint::{api, runtime};
            use $crate::auth::{
                endpoint,
                issuer::{Lookup, RECONCILE, Refusal, Status},
            };
            #[runtime::update(runtime = runtime, guard = endpoint::admit::<RECONCILE>, decode = endpoint::decode::<Lookup, RECONCILE>, max_bytes = 8192, on_error = endpoint::fail, name = "reconcile_portable_credential", encode = endpoint::encode::<Result<Status, Refusal>, RECONCILE>)]
            fn reconcile_portable_credential(input: Lookup) -> Result<Status, Refusal> {
                $reconcile(api::caller(), input, api::time())
            }
        }
        mod revoke_portable_login {
            use endpoint::{api, runtime};
            use $crate::auth::{endpoint, issuer::{LOGOUT, Logout, Refusal, Revoked}};
            #[runtime::update(runtime = runtime, guard = endpoint::admit::<LOGOUT>, decode = endpoint::decode::<Logout, LOGOUT>, max_bytes = 8192, on_error = endpoint::fail, name = "revoke_portable_login", encode = endpoint::encode::<Result<Revoked, Refusal>, LOGOUT>)]
            fn revoke_portable_login(input: Logout) -> Result<Revoked, Refusal> {
                $logout(api::caller(), input, api::time())
            }
        }
    };
}
