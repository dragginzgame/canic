//! One explicitly approved Ed25519 issuer grant and direct authority pulls.
//! Root's controller boundary admits configuration; application handlers own
//! permission. Lost volatile authority or receiver state must remain closed.

#[cfg(all(test, feature = "auth-enrollment"))]
pub(super) mod tests;

#[cfg(feature = "auth-enrollment")]
mod authority;
#[cfg(feature = "auth-enrollment")]
mod receiver;
#[cfg(feature = "auth-enrollment")]
pub mod transport;

use super::protocol::model::{AudienceId, DelegatedRoleGrant};
#[cfg(feature = "auth-enrollment")]
pub use authority::Authority;
#[cfg(feature = "auth-enrollment")]
pub use ic_auth as supplier;
#[cfg(feature = "auth-enrollment")]
use ic_auth::{
    receiver_memory::EnrollmentMetadata,
    verification::{AuthorityGrants, Ed25519Key},
};
pub use ic_auth_protocol_types as types;
use ic_auth_protocol_types::{CanonicalId, Principal};
use ic_wire::{Direction, Encoding, Limits};
#[cfg(feature = "auth-enrollment")]
pub use receiver::{Authenticated, Challenge, Pull, Receiver, Start};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
#[cfg(feature = "auth-enrollment")]
use sha2::{Digest, Sha256};

/// Explicit Root grant approval method; no application login authority.
pub const CONFIGURE: u64 = 0x4341_5554_4801;
/// Terminal revocation of the selected key.
pub const REVOKE: u64 = 0x4341_5554_4802;
/// Actual receiver's direct snapshot pull.
pub const SNAPSHOT: u64 = 0x4341_5554_4803;
/// Controller asks a registered receiver to perform its own protected pull.
pub const TRIGGER_REFRESH: u64 = 0x4341_5554_4804;
/// Controller-only receiver endpoint. Takes unit and replies with a typed result.
pub const REFRESH: u64 = 0x4341_5554_4805;
/// Fixed receiver method used by the controller; never supplied by an operator.
pub const REFRESH_METHOD: &str = "refresh_auth";
/// Maximum receivers, including revoked slots, retained by a fresh Root.
pub const MAX_RECEIVERS: usize = 32;
/// Snapshot exported by an explicitly selected authority.
pub const SNAPSHOT_METHOD: &str = "auth_enrollment";
/// Bounded single-role exchange, including the metadata and complete scope list.
pub const LIMITS: Limits = Limits {
    bytes: 8192,
    allocation: 131072,
    nodes: 2048,
    depth: 12,
};
/// Original local receipt window, including entropy acquisition and dispatch.
pub const RECEIPT_NS: u64 = 30_000_000_000;
/// Maximum snapshot freshness, independently bounded by the grant's approval.
pub const FRESHNESS_NS: u64 = 60_000_000_000;

/// Encode the unit request shared by independently controlled and Root-controlled receivers.
/// # Errors
/// Returns an encoding refusal if the request cannot fit the enrollment budget.
pub fn encode_refresh_request() -> Result<Vec<u8>, Error> {
    encode_request(REFRESH, &())
}

/// Decode the receiver's typed result. A malformed frame is an outer error.
/// # Errors
/// Refuses a reply with the wrong identity, direction, encoding or bounds.
pub fn decode_refresh_response(bytes: &[u8]) -> Result<Result<(), Error>, Error> {
    decode_response(REFRESH, bytes)
}

fn encode_request(method: u64, value: &impl Serialize) -> Result<Vec<u8>, Error> {
    ic_wire::encode(
        value,
        Encoding::Frame {
            method,
            direction: Direction::Request,
        },
        LIMITS,
    )
    .map_err(|_| Error::Invalid)
}

fn decode_response<T: DeserializeOwned>(method: u64, bytes: &[u8]) -> Result<T, Error> {
    ic_wire::decode(
        bytes,
        Encoding::Frame {
            method,
            direction: Direction::Response,
        },
        LIMITS,
    )
    .map_err(|_| Error::Invalid)
}

/// Operator-approved immutable grant. A canister's role or controller relationship
/// alone cannot construct this approval at the mounted Root boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grant {
    /// Exact receiving canister; no wildcard or fleet-membership inference.
    pub receiver: Principal,
    /// Explicit network/application identity, not their display names.
    pub audience: AudienceId,
    /// Issuer qualifying the application subject.
    pub issuer: Principal,
    /// Protected verification key; request credentials cannot replace it.
    pub key: [u8; 32],
    /// One independently approved role and its canonically ordered scopes.
    pub grant: DelegatedRoleGrant,
    /// Inclusive original authority start.
    pub not_before_ns: u64,
    /// Exclusive original approval deadline; refresh never changes it.
    pub expires_at_ns: u64,
}
impl Grant {
    /// Project passive audience bytes into the supplier's identity type.
    #[must_use]
    pub fn audience(&self) -> types::AudienceId {
        types::AudienceId {
            canonical_network_id: CanonicalId::from_bytes(self.audience.canonical_network_id),
            fleet_id: CanonicalId::from_bytes(self.audience.fleet_id),
        }
    }

    /// Validate role/scope grammar and canonical order at the owning supplier.
    #[cfg(feature = "auth-enrollment")]
    pub fn grants(&self) -> Result<AuthorityGrants, Error> {
        let grant = types::DelegatedRoleGrant {
            target: self
                .grant
                .target
                .as_str()
                .parse()
                .map_err(|_| Error::Invalid)?,
            scopes: self
                .grant
                .scopes
                .as_slice()
                .iter()
                .map(|s| s.as_str().parse().map_err(|_| Error::Invalid))
                .collect::<Result<_, _>>()?,
        };
        AuthorityGrants::new(vec![grant]).map_err(|_| Error::Invalid)
    }

    #[cfg(feature = "auth-enrollment")]
    fn validate(&self) -> Result<(), Error> {
        if [self.receiver, self.issuer]
            .iter()
            .any(|p| !valid_principal(*p))
            || self.not_before_ns >= self.expires_at_ns
        {
            return Err(Error::Invalid);
        }
        self.grants()?;
        Ed25519Key::new(self.key).map_err(|_| Error::Invalid)?;
        Ok(())
    }
}

#[cfg(feature = "auth-enrollment")]
fn valid_principal(p: Principal) -> bool {
    p != Principal::anonymous() && p != Principal::management_canister()
}

/// No claimed receiver or authority principal is accepted in a pull request.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// Fresh receiver-host entropy, echoed by the authenticated authority.
    pub challenge: [u8; 32],
    /// Original absolute receipt deadline, never renewed on retries.
    pub receipt_deadline_ns: u64,
}
/// Complete single-grant snapshot. Transport data alone authenticates nothing.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Complete protected grant, including audience and receiver.
    pub grant: Grant,
    /// Exact pull request, including its original deadline.
    pub request: Request,
    /// Monotonic snapshot revision, including freshness-only responses.
    pub revision: u64,
    /// Monotonic content generation; revocation changes it.
    pub generation: u64,
    /// Terminal key retirement.
    pub revoked: bool,
    /// Authority's actual clock at response construction.
    pub issued_at_ns: u64,
    /// Exclusive snapshot freshness, no later than the approval deadline.
    pub expires_at_ns: u64,
}
#[cfg(feature = "auth-enrollment")]
impl Snapshot {
    fn metadata(&self) -> Result<EnrollmentMetadata, Error> {
        self.grant.validate()?;
        if self.revision == 0 || self.generation == 0 {
            return Err(Error::Invalid);
        }
        Ok(EnrollmentMetadata {
            receiver: self.grant.receiver,
            audience: self.grant.audience(),
            challenge: self.request.challenge,
            revision: self.revision,
            generation: self.generation,
            snapshot_digest: digest(&("canic-auth-snapshot-v1", self))?,
            authority_digest: digest(&("canic-auth-authority-v1", &self.grant, self.revoked))?,
            issued_at_ns: self.issued_at_ns,
            expires_at_ns: self.expires_at_ns,
        })
    }
}

#[cfg(feature = "auth-enrollment")]
fn digest(value: &impl Serialize) -> Result<[u8; 32], Error> {
    let bytes = ic_wire::encode(value, Encoding::Payload, LIMITS).map_err(|_| Error::Invalid)?;
    Ok(Sha256::digest(bytes).into())
}
/// Enrollment refusal; none authorizes recreation of lost state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Error {
    /// Original volatile owner is absent or not configured.
    Closed,
    /// A pending original attempt has not expired.
    Busy,
    /// Wrong actual caller, authority, receiver or audience.
    Binding,
    /// Original deadline or current authority freshness elapsed.
    Expired,
    /// Invalid bounds, canonical content, key or commitment.
    Invalid,
    /// Conflicting approval, response, revocation or superseded callback.
    Conflict,
    /// Entropy or direct authority call did not return an admissible reply.
    Unavailable,
    /// Supplier refused enrollment metadata; no host entries were installed.
    Refused,
    /// All receiver slots are occupied. Revocation does not recycle a slot.
    Capacity,
}

/// Mount the optional controller-triggered `refresh_auth` update. The application's
/// async handler owns its existing receiver, reservation, pull and installation.
/// Mounting does not refresh, create state or authorize application sessions.
///
/// ```ignore
/// canic::mount_auth_refresh_endpoint!(crate::auth::pull_enrollment);
/// ```
#[cfg(feature = "auth-enrollment")]
// Rustfmt 1.100 repeatedly reindents multiline attributes in this macro body.
#[rustfmt::skip]
#[macro_export]
macro_rules! mount_auth_refresh_endpoint {
    ($pull:path) => {
        mod refresh_auth {
            use $crate::auth::{
                endpoint::{fail, runtime},
                enrollment::{Error, REFRESH, transport},
            };

            #[runtime::update(
                runtime = runtime,
                guard = transport::admit::<REFRESH>,
                decode = transport::decode::<(), REFRESH>,
                max_bytes = 8192,
                on_error = fail,
                name = "refresh_auth",
                encode = transport::encode::<Result<(), Error>, REFRESH>,
            )]
            async fn refresh_auth((): ()) -> Result<(), Error> {
                transport::authorize_refresh()?;
                $pull().await
            }
        }
    };
}
