//! Bounded account provisioning through the installed hub and its managed shards.

// These host fixtures reopen an isolated VectorMemory. On Wasm, Database uses IC stable memory.
#[cfg(all(test, feature = "provisioning", not(target_arch = "wasm32")))]
mod tests;

#[cfg(feature = "provisioning")]
mod actor;
#[cfg(feature = "provisioning")]
mod rows;

use crate::method::Contract;
#[cfg(feature = "provisioning")]
pub use actor::{Assignment, place};
use ic_platform_types::Principal;
#[cfg(feature = "provisioning")]
pub use rows::{account_row, session_row};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// Durable account selector derived from the authenticated issuer and subject.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
pub struct AuthSubject {
    /// Admitted issuer.
    pub issuer: Principal,
    /// Stable subject within this issuer.
    pub subject: Principal,
}

#[cfg(feature = "provisioning")]
impl AuthSubject {
    fn valid(self) -> bool {
        self.issuer != Principal::anonymous() && self.subject != Principal::anonymous()
    }
}

#[cfg(feature = "provisioning")]
fn matches_target(
    observation: &super::Placement,
    allocation: &super::Allocation,
    shard: Principal,
) -> bool {
    observation.assigned_target().ok().flatten() == Some((allocation, shard))
}

#[cfg(feature = "provisioning")]
fn placement_key<R: Resource>(account: AuthSubject) -> super::Key {
    use sha2::{Digest, Sha256};
    let mut digest = Sha256::new().chain_update(R::DOMAIN);
    for principal in [account.issuer, account.subject] {
        let bytes = principal.as_slice();
        digest.update([u8::try_from(bytes.len()).expect("principal length")]);
        digest.update(bytes);
    }
    digest.finalize().into()
}

/// The app selects its wire ID, initial payload, method name and key namespace.
pub trait Resource {
    /// App-owned stable ID in its Wire representation.
    type Id: Serialize + DeserializeOwned + Clone + Eq;
    /// Initial data retained by the hub until the original operation is confirmed.
    type Data: Serialize + DeserializeOwned;
    /// Application refusal acknowledged by the shard.
    type Refusal: Serialize + DeserializeOwned;
    /// Published parent-only update export.
    const METHOD: &'static str;
    /// Domain separator for the permanent placement key.
    const DOMAIN: &'static [u8];
    /// Directional frame budget for this small request and acknowledgement.
    const LIMITS: ic_wire::Limits = super::LIMITS;
}

/// Exact identity acknowledged by the originally called shard.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
pub struct Binding<I> {
    /// Application-supplied ID.
    pub id: I,
    /// Issuer-qualified identity owning this ID.
    pub account: AuthSubject,
    /// Admitted target principal.
    pub shard: Principal,
}

impl<I: Eq> Binding<I> {
    /// Compare all immutable fields before acknowledging or committing a reservation.
    /// # Panics
    /// Traps if the ID, issuer, subject or shard changed.
    pub fn require_same(&self, retained: &Self) {
        if self != retained {
            crate::method::fail("changed provisioning binding");
        }
    }
}

/// Exact provisioning request. Retries use the retained payload.
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
pub struct Request<I, D> {
    /// Immutable assignment.
    pub binding: Binding<I>,
    /// Application-specific initial data.
    pub data: D,
}

/// Typed parent-to-shard protocol.
pub struct Provision<R>(std::marker::PhantomData<R>);
impl<R: Resource> Contract for Provision<R> {
    const NAME: &'static str = R::METHOD;
    const ID: u64 = ic_wire::method_id(R::METHOD);
    const REQUEST: ic_wire::Limits = R::LIMITS;
    const REPLY: ic_wire::Limits = R::LIMITS;
    type Input = Request<R::Id, R::Data>;
    type Output = Result<Binding<R::Id>, R::Refusal>;
}

/// Outcome of an unsuccessful hub dispatch.
#[derive(Debug, Eq, PartialEq)]
pub enum SendError<E> {
    /// Transport or placement could not establish an exact reply.
    Progress(Progress),
    /// The assigned shard acknowledged an application-defined refusal.
    Refused(E),
}

/// Result of one bounded signup attempt, in the existing Toko Wire order.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
pub enum Progress {
    /// The hub holds an exact confirmation.
    Ready,
    /// An earlier creation or capacity claim is pending.
    PlacementPending,
    /// The retained request can be reconciled on retry.
    ProvisionPending,
    /// The approved pool cannot accept another key.
    PoolFull,
    /// The pool cannot verify the original target.
    PlacementUnavailable,
}
