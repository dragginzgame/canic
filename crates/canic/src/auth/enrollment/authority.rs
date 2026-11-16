use super::{Error, FRESHNESS_NS, Grant, RECEIPT_NS, Request, Snapshot, types::Principal};

/// Volatile projection of one controller-approved grant. The containing Root
/// owns controller admission and must not recreate this owner after upgrade.
pub struct Authority {
    grant: Grant,
    revision: u64,
    revoked: bool,
    observed_at_ns: u64,
    last: Option<Snapshot>,
}
impl Authority {
    /// Admit a first explicit grant on a genuinely fresh authority only.
    pub fn new(grant: Grant, now_ns: u64) -> Result<Self, Error> {
        grant.validate()?;
        if now_ns < grant.not_before_ns || now_ns >= grant.expires_at_ns {
            return Err(Error::Expired);
        }
        Ok(Self {
            grant,
            revision: 0,
            revoked: false,
            observed_at_ns: now_ns,
            last: None,
        })
    }
    /// Exact configuration retry never replaces a grant or revives its key.
    pub fn matches(&self, grant: &Grant) -> Result<(), Error> {
        if self.grant == *grant {
            Ok(())
        } else {
            Err(Error::Conflict)
        }
    }
    /// Terminal retirement, authorized by the containing Root's real controller.
    pub fn revoke(&mut self, now_ns: u64) -> Result<(), Error> {
        self.observe(now_ns)?;
        self.revoked = true;
        Ok(())
    }
    /// Construct for the actual calling receiver. Exact retries retain their
    /// bytes and deadline unless revocation superseded the original authority.
    pub fn snapshot(
        &mut self,
        caller: Principal,
        request: Request,
        now_ns: u64,
    ) -> Result<Snapshot, Error> {
        self.observe(now_ns)?;
        if caller != self.grant.receiver {
            return Err(Error::Binding);
        }
        if request.challenge == [0; 32] {
            return Err(Error::Invalid);
        }
        let remaining = request
            .receipt_deadline_ns
            .checked_sub(now_ns)
            .ok_or(Error::Expired)?;
        if remaining == 0 {
            return Err(Error::Expired);
        }
        if remaining > RECEIPT_NS {
            return Err(Error::Invalid);
        }
        if now_ns >= self.grant.expires_at_ns {
            return Err(Error::Expired);
        }
        if let Some(old) = &self.last {
            if old.request == request {
                if old.revoked != self.revoked {
                    return Err(Error::Conflict);
                }
                if now_ns >= old.expires_at_ns {
                    return Err(Error::Expired);
                }
                return Ok(old.clone());
            }
            if old.request.challenge == request.challenge {
                return Err(Error::Conflict);
            }
        }
        let revision = self.revision.checked_add(1).ok_or(Error::Closed)?;
        let snapshot = Snapshot {
            grant: self.grant.clone(),
            request,
            revision,
            generation: if self.revoked { 2 } else { 1 },
            revoked: self.revoked,
            issued_at_ns: now_ns,
            expires_at_ns: now_ns
                .saturating_add(FRESHNESS_NS)
                .min(self.grant.expires_at_ns),
        };
        snapshot.metadata()?;
        self.last = Some(snapshot.clone());
        self.revision = revision;
        Ok(snapshot)
    }
    fn observe(&mut self, now_ns: u64) -> Result<(), Error> {
        if now_ns < self.observed_at_ns {
            return Err(Error::Invalid);
        }
        self.observed_at_ns = now_ns;
        Ok(())
    }
}
