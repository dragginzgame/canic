use super::{
    Error, RECEIPT_NS, Request, Snapshot,
    types::{AudienceId, Principal},
};
use ic_auth::receiver_memory::{EnrollmentBinding, MemoryReceiver};

/// Receiver-owned volatile enrollment projection. The application owns the one
/// real Auth receiver and publishes both in a single IC callback transaction.
pub struct Receiver {
    binding: EnrollmentBinding,
    audience: AudienceId,
    sequence: u64,
    pending: Option<(u64, u64)>,
    current: Option<Snapshot>,
    observed_at_ns: u64,
}
/// Original admission before entropy acquisition; fields cannot be supplied by ingress.
#[derive(Debug, Eq, PartialEq)]
#[cfg_attr(test, derive(Clone))]
pub struct Start {
    sequence: u64,
    deadline_ns: u64,
}
/// A management-generated challenge for one original admission.
pub struct Challenge {
    pub(super) start: Start,
    pub(super) bytes: [u8; 32],
}
/// One direct call ticket. It borrows no application state across an await.
pub struct Pull {
    pub(super) authority: Principal,
    pub(super) start: Start,
    pub(super) request: Request,
}
/// Reply provenance can only be produced by the ticket's direct runtime call.
/// No public decoder/constructor accepts a relayed or caller-provided snapshot.
pub struct Authenticated {
    pub(super) pull: Pull,
    pub(super) snapshot: Snapshot,
}
impl Receiver {
    /// Start closed on a genuinely fresh receiver with protected installed bindings.
    /// This creates no Auth engine and cannot recover lost replay/revocation state.
    pub fn new(
        binding: EnrollmentBinding,
        audience: AudienceId,
        now_ns: u64,
    ) -> Result<Self, Error> {
        if !super::valid_principal(binding.receiver) || !super::valid_principal(binding.authority) {
            return Err(Error::Binding);
        }
        Ok(Self {
            binding,
            audience,
            sequence: 0,
            pending: None,
            current: None,
            observed_at_ns: now_ns,
        })
    }
    /// Reserve the original receipt window before awaiting entropy. An unexpired
    /// attempt stays busy, including after a transport failure or callback trap.
    pub fn begin(&mut self, now_ns: u64) -> Result<Start, Error> {
        self.observe(now_ns)?;
        if self.pending.is_some_and(|(_, deadline)| now_ns < deadline) {
            return Err(Error::Busy);
        }
        let start = Start {
            sequence: self.sequence.checked_add(1).ok_or(Error::Closed)?,
            deadline_ns: now_ns.checked_add(RECEIPT_NS).ok_or(Error::Closed)?,
        };
        self.sequence = start.sequence;
        self.pending = Some((start.sequence, start.deadline_ns));
        Ok(start)
    }
    /// Bind real entropy to the pending admission and the supplier's exact window.
    pub fn prepare(
        &mut self,
        engine: &mut MemoryReceiver,
        challenge: Challenge,
        now_ns: u64,
    ) -> Result<Pull, Error> {
        self.check(&challenge.start, now_ns)?;
        engine
            .request_enrollment(now_ns, challenge.bytes, challenge.start.deadline_ns)
            .map_err(|_| Error::Refused)?;
        Ok(Pull {
            authority: self.binding.authority,
            request: Request {
                challenge: challenge.bytes,
                receipt_deadline_ns: challenge.start.deadline_ns,
            },
            start: challenge.start,
        })
    }
    /// Validate the complete authenticated response before supplier publication.
    /// No fallible host work follows successful metadata installation. A later
    /// application persistence/encoding failure must trap the whole IC callback.
    pub fn finish(
        &mut self,
        engine: &mut MemoryReceiver,
        response: Authenticated,
        now_ns: u64,
    ) -> Result<(), Error> {
        // A duplicate committed response is harmless only within its original
        // window, without a newer pending request or any authority substitution.
        if self.pending.is_none()
            && self.sequence == response.pull.start.sequence
            && response.pull.authority == self.binding.authority
            && self.current.as_ref() == Some(&response.snapshot)
        {
            self.observe(now_ns)?;
            if now_ns >= response.pull.start.deadline_ns {
                return Err(Error::Expired);
            }
            return Ok(());
        }
        self.check(&response.pull.start, now_ns)?;
        let snapshot = response.snapshot;
        if response.pull.authority != self.binding.authority
            || snapshot.grant.receiver != self.binding.receiver
            || snapshot.grant.audience() != self.audience
        {
            return Err(Error::Binding);
        }
        if snapshot.request != response.pull.request {
            return Err(Error::Conflict);
        }
        if snapshot.issued_at_ns < snapshot.grant.not_before_ns
            || snapshot.expires_at_ns > snapshot.grant.expires_at_ns
            || snapshot.expires_at_ns.saturating_sub(snapshot.issued_at_ns) > super::FRESHNESS_NS
        {
            return Err(Error::Invalid);
        }
        if self
            .current
            .as_ref()
            .is_some_and(|old| old.grant != snapshot.grant || (old.revoked && !snapshot.revoked))
        {
            return Err(Error::Conflict);
        }
        let metadata = snapshot.metadata()?;
        engine
            .install_enrollment(now_ns, response.pull.authority, metadata)
            .map_err(|_| Error::Refused)?;
        self.current = Some(snapshot);
        self.pending = None;
        Ok(())
    }
    /// Borrow current protected content without extending freshness or any session.
    pub fn current(&self, now_ns: u64) -> Result<&Snapshot, Error> {
        if now_ns < self.observed_at_ns {
            return Err(Error::Invalid);
        }
        let snapshot = self.current.as_ref().ok_or(Error::Closed)?;
        if now_ns >= snapshot.expires_at_ns || now_ns >= snapshot.grant.expires_at_ns {
            return Err(Error::Expired);
        }
        Ok(snapshot)
    }
    fn check(&mut self, start: &Start, now_ns: u64) -> Result<(), Error> {
        self.observe(now_ns)?;
        if self.pending != Some((start.sequence, start.deadline_ns)) {
            return Err(Error::Conflict);
        }
        if now_ns >= start.deadline_ns {
            return Err(Error::Expired);
        }
        Ok(())
    }
    fn observe(&mut self, now_ns: u64) -> Result<(), Error> {
        if now_ns < self.observed_at_ns {
            return Err(Error::Invalid);
        }
        self.observed_at_ns = now_ns;
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
impl Start {
    /// Obtain fresh IC entropy without renewing the admitted deadline.
    pub async fn entropy(self) -> Result<Challenge, Error> {
        use ic_platform_runtime::{
            call::{Call, Outcome, Wait},
            system as api,
        };
        if api::time() >= self.deadline_ns {
            return Err(Error::Expired);
        }
        let response = Call::new(
            Principal::management_canister(),
            ic_platform_protocol::raw_rand::METHOD,
            Wait::Bounded { seconds: 300 },
        )
        .args(ic_platform_protocol::raw_rand::encode(6).map_err(|_| Error::Unavailable)?)
        .execute()
        .await
        .map_err(|_| Error::Unavailable)?;
        let response = match response.into_parts().0 {
            Outcome::Reply(bytes) => bytes,
            _ => return Err(Error::Unavailable),
        };
        if api::time() >= self.deadline_ns {
            return Err(Error::Expired);
        }
        let bytes = ic_platform_protocol::raw_rand::decode(response.as_ref(), 42)
            .map_err(|_| Error::Unavailable)?;
        if bytes == [0; 32] {
            return Err(Error::Unavailable);
        }
        Ok(Challenge { start: self, bytes })
    }
}
#[cfg(target_arch = "wasm32")]
impl Pull {
    /// Authenticate by the actual call destination; never trust a response's
    /// claimed principal. This function owns one attempt, with no fallback/retry.
    pub async fn fetch(self) -> Result<Authenticated, Error> {
        use ic_platform_runtime::{
            call::{Call, Outcome, Wait},
            system as api,
        };
        if api::time() >= self.start.deadline_ns {
            return Err(Error::Expired);
        }
        let bytes = super::transport::encode_request(super::SNAPSHOT, &self.request)?;
        let response = Call::new(
            self.authority,
            super::SNAPSHOT_METHOD,
            Wait::Bounded { seconds: 300 },
        )
        .args(&bytes)
        .execute()
        .await
        .map_err(|_| Error::Unavailable)?;
        let response = match response.into_parts().0 {
            Outcome::Reply(bytes) => bytes,
            _ => return Err(Error::Unavailable),
        };
        if api::time() >= self.start.deadline_ns {
            return Err(Error::Expired);
        }
        let snapshot: Result<Snapshot, Error> =
            super::transport::decode_response(super::SNAPSHOT, response.as_ref())?;
        Ok(Authenticated {
            pull: self,
            snapshot: snapshot?,
        })
    }
}
