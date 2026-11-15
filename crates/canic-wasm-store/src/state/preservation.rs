//! Enumerate existing metadata only while its captured owner keeps the exact attempt closed.
use super::{Excluded, Key, MemoryBacking, State, Unbounded};
use canic_wasm_store_protocol::{
    Error,
    preservation::{Catalog, Page, Request},
};

impl<M: MemoryBacking> State<M> {
    pub fn preservation(&self, caller: &[u8], request: &Request) -> Result<Page, Error> {
        let settings = self.settings.get();
        if !settings.admission.as_ref().is_some_and(|gate| {
            gate.closed && gate.owner == caller && gate.attempt == request.attempt
        }) {
            return Err(Error::Conflict);
        }
        if request
            .after
            .as_ref()
            .is_some_and(|release| !release.valid())
        {
            return Err(Error::Invalid);
        }
        let after = request
            .after
            .as_ref()
            .map_or(Unbounded, |r| Excluded(Key::new(r)));
        let entry = self
            .releases
            .keys_range((after, Unbounded))
            .next()
            .map(|key| self.custody(&key.release()))
            .transpose()?;
        Ok(Page {
            catalog: Catalog {
                quota: settings.quota.clone(),
                reserved_bytes: settings.reserved_bytes,
                releases: self.releases.len(),
                artifacts: self.artifacts.len(),
                chunks: self.chunks.len(),
            },
            entry,
        })
    }
}
