//! Bounded caller-side proof acquisition and one-shot application dispatch.
use super::{Contract, Issue, Principal, Proof, wire};
use crate::method::{CallError, call};
use ic_platform_runtime::system;
use std::{cell::RefCell, marker::PhantomData};

/// Caller roles admitted by both ends of this application method.
///
/// A shared role method requires proof admission on its receiver.
///
/// ```compile_fail,E0283
/// #[canic::attested_method(roles = &["project"])]
/// fn announce() {}
/// type Shared = announce;
/// #[canic::update(contract = Shared, name = "announce")]
/// fn receive() {}
/// ```
pub trait RoleMethod: Contract {
    /// Accept a proof of any listed role. An empty slice admits no caller.
    const ROLES: &'static [&'static str];
}

/// Wire frame shared by the mounted receiver and its caller.
pub struct Attested<M>(PhantomData<M>);
// The preframed caller envelope also requires a verifying receiver.
impl<M: RoleMethod> crate::method::PlainMount<crate::method::AttestedMount> for Attested<M> {}

impl<M: RoleMethod> Contract for Attested<M> {
    const NAME: &'static str = M::NAME;
    const ID: u64 = M::ID;
    const REQUEST: wire::Limits = M::REQUEST;
    const REPLY: wire::Limits = M::REPLY;
    type Input = (Proof, M::Input);
    type Output = M::Output;
}

/// Identifies whether failure preceded the application attempt.
#[derive(Debug)]
pub enum RoleCallError {
    /// Root request failed before the application call.
    Renewal(CallError),
    /// Root refused the claim or it did not match the call.
    Proof(String),
    /// The application withdrew permission before dispatch.
    Authorization(String),
    /// Result of the single application attempt, including uncertain delivery.
    Application(CallError),
}

/// Retain one recently used audience in volatile memory. Reopen with the
/// installing Root after upgrade. Eviction or upgrade costs one renewal.
pub struct RoleCalls {
    root: Principal,
    last: RefCell<Option<Proof>>,
}

impl RoleCalls {
    /// Bind the outgoing calls to the Root recorded at install.
    #[must_use]
    pub fn new(root: Principal) -> Self {
        Self {
            root,
            last: RefCell::default(),
        }
    }

    fn matches<M: RoleMethod>(&self, proof: &Proof, receiver: Principal) -> bool {
        let claim = &proof.0;
        let now = system::time();
        claim.root == self.root
            && claim.subject == system::canister_self()
            && claim.receiver == receiver
            && M::ROLES.contains(&claim.role.as_str())
            && now >= claim.issued_at_ns
            && claim.expiry().is_ok_and(|expiry| now < expiry)
    }

    /// Obtain a cached proof for use in a query without issuing an update.
    #[must_use]
    pub fn cached<M: RoleMethod>(&self, receiver: Principal) -> Option<Proof> {
        self.last
            .borrow()
            .as_ref()
            .filter(|p| self.matches::<M>(p, receiver))
            .cloned()
    }

    /// Call using only an existing proof. A cold or expired cache returns before
    /// dispatch. An update can populate it for subsequent query calls.
    /// # Errors
    /// Reports a missing proof or the result of the one application attempt.
    pub async fn call_cached<M: RoleMethod>(
        &self,
        receiver: Principal,
        seconds: u32,
        input: M::Input,
    ) -> Result<M::Output, RoleCallError> {
        let proof = self
            .cached::<M>(receiver)
            .ok_or_else(|| RoleCallError::Proof("role renewal requires an update".into()))?;
        call::<Attested<M>>(receiver, seconds, &(proof, input))
            .await
            .map_err(RoleCallError::Application)
    }

    /// From an update, renew on demand and submit the application method once.
    /// Recheck application authority after any renewal await and before dispatch.
    /// # Errors
    /// Distinguishes renewal failures from application refusals or uncertain delivery.
    pub async fn call<M: RoleMethod>(
        &self,
        receiver: Principal,
        seconds: u32,
        input: M::Input,
        authorize: impl Fn() -> Result<(), String>,
    ) -> Result<M::Output, RoleCallError> {
        authorize().map_err(RoleCallError::Authorization)?;
        let proof = if let Some(proof) = self.cached::<M>(receiver) {
            proof
        } else {
            let proof = call::<Issue>(self.root, seconds, &receiver)
                .await
                .map_err(RoleCallError::Renewal)?
                .map_err(RoleCallError::Proof)?;
            if !self.matches::<M>(&proof, receiver) {
                return Err(RoleCallError::Proof(
                    "issuer returned a different or expired claim".into(),
                ));
            }
            self.last.replace(Some(proof.clone()));
            authorize().map_err(RoleCallError::Authorization)?;
            proof
        };
        call::<Attested<M>>(receiver, seconds, &(proof, input))
            .await
            .map_err(RoleCallError::Application)
    }
}
