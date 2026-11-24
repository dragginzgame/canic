//! Direct role proof issued for one caller, receiver and placement generation.
use crate::method::{Contract, wire};
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};

/// Maximum lifetime and default retirement waiting period, in nanoseconds.
pub const LIFETIME_NS: u64 = 7_200_000_000_000;

/// Signed allocation identity under its issuing Root.
#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
pub struct Claim {
    /// Root responsible for this allocation.
    pub root: Principal,
    /// Ready child that requested the proof.
    pub subject: Principal,
    /// Only receiver that may accept it.
    pub receiver: Principal,
    /// Role in the approved topology install frame.
    pub role: String,
    /// Original creation operation digest.
    pub generation: wire::Bytes<32>,
    /// IC time at issuance.
    pub issued_at_ns: u64,
}

impl Claim {
    /// Exclusive expiration, fixed at issuance.
    /// # Errors
    /// Refuses an unrepresentable deadline.
    pub fn expiry(&self) -> Result<u64, &'static str> {
        self.issued_at_ns
            .checked_add(LIFETIME_NS)
            .ok_or("role proof deadline overflow")
    }

    #[cfg(feature = "role-verification")]
    fn check_time(&self) -> Result<(), &'static str> {
        let now = ic_platform_runtime::system::time();
        if now < self.issued_at_ns || now >= self.expiry()? {
            return Err("role refused");
        }
        Ok(())
    }

    /// Canonical, domain-separated Ed25519 signing input.
    /// # Errors
    /// Refuses an oversized claim or a malformed generation.
    pub fn signing_bytes(&self) -> Result<Vec<u8>, wire::Error> {
        if self.generation.as_slice().len() != 32 {
            return Err(wire::Error::Limit(wire::LimitKind::Field));
        }
        wire::encode(
            self,
            wire::Encoding::Frame {
                method: wire::method_id("canic.role-proof.v1"),
                direction: wire::Direction::Response,
            },
            ic_wire::Limits {
                bytes: 512,
                allocation: 16_384,
                nodes: 128,
                depth: 8,
            },
        )
    }
}

/// Claim and detached Ed25519 signature.
pub type Proof = (Claim, wire::Bytes<64>);

///
/// CloseRoleIssuance
/// Controller approval to end renewal for one empty, drained allocation.
///

pub struct CloseRoleIssuance;

impl Contract for CloseRoleIssuance {
    const NAME: &'static str = "close_role_issuance";
    const ID: u64 = wire::method_id(Self::NAME);
    type Input = (Principal, wire::Bytes<32>);
    /// Original proof expiry ceiling in IC nanoseconds.
    type Output = Result<u64, String>;
}

/// Recover the installing Root's stable Ed25519 public key.
pub struct PrepareRoleKey;
impl Contract for PrepareRoleKey {
    const NAME: &'static str = "prepare_role_key";
    const ID: u64 = wire::method_id(Self::NAME);
    type Input = ();
    type Output = Result<wire::Bytes<32>, String>;
}

/// One replicated request by the actual shard for a proof addressed to a receiver.
pub struct Issue;
impl Contract for Issue {
    const NAME: &'static str = "issue_role";
    const ID: u64 = wire::method_id(Self::NAME);
    type Input = Principal;
    type Output = Result<Proof, String>;
}

/// A receiver's trusted installing Root and its approved Ed25519 verifying key.
#[cfg(feature = "role-verification")]
pub struct RoleVerifier {
    root: Principal,
    key: ic_platform::ed25519::VerifyingKey,
}

#[cfg(feature = "role-verification")]
impl RoleVerifier {
    /// Load the receiver's retained install pin. The caller supplies the Root
    /// from its trusted install context, never from the presented proof.
    /// # Errors
    /// Refuses a non-canister Root or an invalid or weak Ed25519 key.
    pub fn new(root: Principal, public_key: &[u8]) -> Result<Self, &'static str> {
        if root.as_slice().last() != Some(&1) {
            return Err("expected canister Root");
        }
        let key = ic_platform::ed25519::VerifyingKey::from_bytes(
            public_key.try_into().map_err(|_| "invalid role key")?,
        )
        .map_err(|_| "invalid role key")?;
        if key.is_weak() {
            return Err("weak role key");
        }
        Ok(Self { root, key })
    }

    /// Admit a role-bound request at ingress, before performing protected work.
    /// The receiver checks the IC caller, its own identity and IC time locally.
    /// # Errors
    /// Refuses a mismatch, malformed signature or proof outside its validity window.
    pub fn require_role(
        &self,
        roles: &[&str],
        (claim, signature): &Proof,
    ) -> Result<(), &'static str> {
        use ic_platform_runtime::system;
        if claim.root != self.root
            || claim.subject != system::caller()
            || claim.receiver != system::canister_self()
            || !roles.contains(&claim.role.as_str())
        {
            return Err("role refused");
        }
        claim.check_time()?;
        let signature = ic_platform::ed25519::Signature::from_slice(signature.as_slice())
            .map_err(|_| "signature")?;
        self.key
            .verify_strict(&claim.signing_bytes().map_err(|_| "claim")?, &signature)
            .map_err(|_| "signature")
    }
}

mod client;
pub use client::{Attested, RoleCallError, RoleCalls, RoleMethod};

#[cfg(feature = "role-verification")]
thread_local! {
    static TRUST: std::cell::OnceCell<RoleVerifier> = const { std::cell::OnceCell::new() };
}

#[cfg(feature = "role-verification")]
impl RoleVerifier {
    /// Attach this receiver's reopened pin to its role-protected endpoints.
    /// # Errors
    /// Refuses replacement during the current installation or upgrade.
    pub fn attach(self) -> Result<(), &'static str> {
        TRUST.with(|slot| slot.set(self).map_err(|_| "role verifier already attached"))
    }
}

/// Admission for the original role-bound call. The install pin and signed role are
/// immutable during a call; its time window can expire while the handler awaits.
#[cfg(feature = "role-verification")]
pub struct RoleAdmission<'a>(&'a Claim);

#[cfg(feature = "role-verification")]
impl RoleAdmission<'_> {
    /// Check the admitted proof immediately before resuming protected work.
    /// # Errors
    /// Refuses outside the original proof's validity window.
    pub fn reauthorize(&self) -> Result<(), &'static str> {
        self.0.check_time()
    }
}

#[cfg(feature = "role-verification")]
pub(crate) fn verify_roles<'a>(
    roles: &[&str],
    proof: &'a Proof,
) -> Result<RoleAdmission<'a>, &'static str> {
    TRUST.with(|slot| {
        slot.get()
            .ok_or("missing role verifier")?
            .require_role(roles, proof)?;
        Ok(RoleAdmission(&proof.0))
    })
}
