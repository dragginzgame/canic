//! Store owns mutation admission; an external owner still owns installation and quiescence.
use super::{Error, MemoryBacking, State};
use canic_wasm_store_protocol::{Admission, AdmissionCommand};

impl<M: MemoryBacking> State<M> {
    pub fn admission(
        &mut self,
        caller: &[u8],
        command: &AdmissionCommand,
    ) -> Result<Option<Admission>, Error> {
        let mut settings = self.settings.get().clone();
        let attempt = match command {
            AdmissionCommand::Observe => return Ok(settings.admission),
            AdmissionCommand::Close(attempt) | AdmissionCommand::Activate(attempt) => attempt,
        };
        if caller.is_empty()
            || caller.len() > 29
            || caller == [4]
            || attempt.sequence == 0
            || attempt.operation == [0; 32]
        {
            return Err(Error::Invalid);
        }
        if let Some(old) = &settings.admission
            && old.attempt == *attempt
            && old.owner == caller
        {
            // A delayed Close after activation observes Open; it cannot re-close this attempt.
            if matches!(command, AdmissionCommand::Close(_)) || !old.closed {
                return Ok(settings.admission);
            }
        } else if matches!(command, AdmissionCommand::Activate(_)) {
            return Err(Error::Conflict);
        }
        if matches!(command, AdmissionCommand::Close(_)) {
            let previous = settings.admission.as_ref();
            if previous.is_some_and(|old| old.closed)
                || previous
                    .map_or(0, |old| old.attempt.sequence)
                    .checked_add(1)
                    != Some(attempt.sequence)
            {
                return Err(Error::Conflict);
            }
        }
        settings.admission = Some(Admission {
            attempt: attempt.clone(),
            owner: caller.to_vec(),
            closed: matches!(command, AdmissionCommand::Close(_)),
        });
        let result = settings.admission.clone();
        self.settings.set(settings);
        Ok(result)
    }

    /// A cache continuation must retain this epoch, even if admission reopens before its reply.
    pub fn admission_epoch(&self) -> Result<u64, Error> {
        match &self.settings.get().admission {
            Some(gate) if gate.closed => Err(Error::Closed),
            Some(gate) => Ok(gate.attempt.sequence),
            None => Ok(0),
        }
    }
    pub fn cache_fence(&self, admitted: u64) -> Result<(), Error> {
        if self.admission_epoch()? != admitted {
            return Err(Error::Conflict);
        }
        Ok(())
    }
}
