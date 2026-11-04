//! Module: ops::auth::enrollment::receiver
//!
//! Responsibility: retain closed-on-restore readiness and authenticate direct Root pulls.
//! Boundary: lifecycle supplies fresh challenges; no relayed document installs authority.

use crate::{
    ops::{
        auth::{
            AuthOps,
            enrollment::{
                EnrollmentBinding, PortableEnrollmentError, decode, encode,
                validation::{validate_progress, validate_snapshot},
            },
        },
        ic::IcOps,
        runtime::env::EnvOps,
        storage::auth::{
            LocalApplicationAuthorizationStateOps,
            application_sessions::ApplicationSessionStateError,
        },
    },
    storage::stable::auth::{
        LocalApplicationAuthorizationState,
        enrollment::{PortableEnrollmentRecord, ReceiverPortableEnrollmentState},
    },
};
use canic_contracts::{
    dto::portable_enrollment::PortableEnrollmentSnapshot, protocol::portable_enrollment as wire,
};
use ic_platform::Principal;
use std::cell::Cell;

thread_local! { static BOOT_GENERATION: Cell<u64> = const { Cell::new(0) }; }

/// A receiver's heap-only readiness gate. Restored checkpoints never imply readiness.
///
/// Host lifecycle supplies a fresh CSPRNG boot challenge; no accepting endpoint
/// may take that challenge or receipt deadline from a browser request.
pub struct ReceiverPortableEnrollment {
    binding: EnrollmentBinding,
    challenge: [u8; 32],
    boot_generation: u64,
    current: Option<PortableEnrollmentSnapshot>,
}

/// Close retained gates before recovery or protected authority changes.
///
/// # Panics
/// Panics on exhaustion of the heap boot counter rather than wrapping it.
pub fn invalidate_receiver_boot() {
    BOOT_GENERATION.with(|slot| {
        slot.set(
            slot.get()
                .checked_add(1)
                .expect("receiver enrollment boot generation"),
        );
    });
}

impl ReceiverPortableEnrollment {
    /// Start closed after stable restoration. The host owns challenge acquisition.
    pub fn boot(challenge: [u8; 32]) -> Result<Self, PortableEnrollmentError> {
        start_boot(protected_binding()?, challenge, IcOps::now_nanos())
    }

    /// Prepare a pull without holding this owner across an asynchronous call.
    pub fn prepare_pull(
        &mut self,
        receipt_deadline_ns: u64,
    ) -> Result<PortableEnrollmentPull, PortableEnrollmentError> {
        self.prepare_at(
            protected_binding()?,
            IcOps::now_nanos(),
            receipt_deadline_ns,
        )
    }

    pub(super) fn prepare_at(
        &mut self,
        binding: EnrollmentBinding,
        now_ns: u64,
        receipt_deadline_ns: u64,
    ) -> Result<PortableEnrollmentPull, PortableEnrollmentError> {
        self.observe(now_ns)?;
        if self.binding != binding {
            self.current = None;
            return Err(PortableEnrollmentError::Binding);
        }
        if receipt_deadline_ns <= now_ns || receipt_deadline_ns - now_ns > wire::MAX_FRESHNESS_NS {
            return Err(PortableEnrollmentError::Invalid);
        }
        Ok(PortableEnrollmentPull {
            binding: self.binding,
            challenge: self.challenge,
            boot_generation: self.boot_generation,
            receipt_deadline_ns,
        })
    }

    /// Install only an opaque result from a direct Root pull; never a relayed document.
    pub fn finish_pull(
        &mut self,
        reply: AuthenticatedPortableEnrollment,
    ) -> Result<(), PortableEnrollmentError> {
        self.finish_at(protected_binding()?, reply, IcOps::now_nanos())
    }

    pub(super) fn finish_at(
        &mut self,
        binding: EnrollmentBinding,
        reply: AuthenticatedPortableEnrollment,
        now_ns: u64,
    ) -> Result<(), PortableEnrollmentError> {
        self.observe(now_ns)?;
        if self.binding != binding
            || reply.pull.binding != self.binding
            || reply.pull.challenge != self.challenge
        {
            self.current = None;
            return Err(PortableEnrollmentError::Binding);
        }
        if reply.pull.boot_generation != self.boot_generation {
            return Err(PortableEnrollmentError::Unavailable);
        }
        self.install_authenticated(reply.snapshot, now_ns, reply.pull.receipt_deadline_ns)
    }

    /// Return only the current receiver-bound snapshot, without renewing its lease.
    pub fn current(&self) -> Result<&PortableEnrollmentSnapshot, PortableEnrollmentError> {
        if self.binding != protected_binding()? {
            return Err(PortableEnrollmentError::Binding);
        }
        self.current_at(IcOps::now_nanos())
    }

    fn observe(&mut self, now_ns: u64) -> Result<(), PortableEnrollmentError> {
        if self.boot_generation != BOOT_GENERATION.with(Cell::get) {
            self.current = None;
            return Err(PortableEnrollmentError::Unavailable);
        }
        if let Err(error) = observe_time(now_ns) {
            self.current = None;
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn current_at(
        &self,
        now_ns: u64,
    ) -> Result<&PortableEnrollmentSnapshot, PortableEnrollmentError> {
        if self.boot_generation != BOOT_GENERATION.with(Cell::get) {
            return Err(PortableEnrollmentError::Unavailable);
        }
        if now_ns < LocalApplicationAuthorizationState::application_observed_at_ns() {
            return Err(PortableEnrollmentError::Clock);
        }
        let current = self
            .current
            .as_ref()
            .ok_or(PortableEnrollmentError::Unavailable)?;
        if now_ns >= current.expires_at_ns {
            return Err(PortableEnrollmentError::Expired);
        }
        Ok(current)
    }

    pub(super) fn install_authenticated(
        &mut self,
        snapshot: PortableEnrollmentSnapshot,
        now_ns: u64,
        receipt_deadline_ns: u64,
    ) -> Result<(), PortableEnrollmentError> {
        self.observe(now_ns)?;
        validate_snapshot(
            &snapshot,
            self.binding,
            self.challenge,
            now_ns,
            receipt_deadline_ns,
        )?;
        let previous = ReceiverPortableEnrollmentState::read();
        let bytes = encode(&snapshot)?;
        let mut generation = LocalApplicationAuthorizationState::application_authority_generation();
        if previous.bytes.is_empty() {
            generation = generation
                .checked_add(1)
                .ok_or(PortableEnrollmentError::Capacity)?;
        } else {
            let retained: PortableEnrollmentSnapshot = decode(&previous.bytes)?;
            validate_progress(&retained, &snapshot)?;
            if snapshot.revision == retained.revision {
                self.current = Some(snapshot);
                return Ok(());
            }
            if snapshot.generation != retained.generation {
                generation = generation
                    .checked_add(1)
                    .ok_or(PortableEnrollmentError::Capacity)?;
            }
        }
        ReceiverPortableEnrollmentState::commit(
            PortableEnrollmentRecord {
                bytes,
                observed_at_ns: now_ns,
            },
            generation,
        );
        self.current = Some(snapshot);
        Ok(())
    }
}

pub(super) fn start_boot(
    binding: EnrollmentBinding,
    challenge: [u8; 32],
    now_ns: u64,
) -> Result<ReceiverPortableEnrollment, PortableEnrollmentError> {
    let generation = BOOT_GENERATION.with(|slot| {
        let next = slot
            .get()
            .checked_add(1)
            .ok_or(PortableEnrollmentError::Capacity)?;
        slot.set(next);
        Ok(next)
    })?;
    observe_time(now_ns)?;
    if challenge == [0; 32]
        || binding.authority == Principal::anonymous()
        || binding.receiver == Principal::anonymous()
    {
        return Err(PortableEnrollmentError::Invalid);
    }
    let checkpoint = ReceiverPortableEnrollmentState::read();
    if checkpoint.observed_at_ns > now_ns {
        return Err(PortableEnrollmentError::Clock);
    }
    if !checkpoint.bytes.is_empty() {
        let retained: PortableEnrollmentSnapshot = decode(&checkpoint.bytes)?;
        if retained.boot_challenge == challenge {
            return Err(PortableEnrollmentError::Binding);
        }
    }
    Ok(ReceiverPortableEnrollment {
        binding,
        challenge,
        boot_generation: generation,
        current: None,
    })
}

fn protected_binding() -> Result<EnrollmentBinding, PortableEnrollmentError> {
    let authority = AuthOps::local_application_authorization_authority()
        .map_err(|_| PortableEnrollmentError::Unavailable)?
        .ok_or(PortableEnrollmentError::Unavailable)?;
    Ok(EnrollmentBinding {
        authority: EnvOps::root_pid()
            .map_err(|_| PortableEnrollmentError::Unavailable)?
            .into(),
        receiver: IcOps::canister_self().into(),
        fleet: authority.snapshot.fleet(),
    })
}

/// One immutable pull ticket. It borrows no receiver state while the IC call waits.
pub struct PortableEnrollmentPull {
    pub(super) binding: EnrollmentBinding,
    pub(super) challenge: [u8; 32],
    pub(super) boot_generation: u64,
    pub(super) receipt_deadline_ns: u64,
}

/// A direct IC response to the ticket's exact installed Root. No public constructor
/// or transport decoder can manufacture this marker from a relayed snapshot.
pub struct AuthenticatedPortableEnrollment {
    pub(super) pull: PortableEnrollmentPull,
    pub(super) snapshot: PortableEnrollmentSnapshot,
}

impl PortableEnrollmentPull {
    /// Fetch independently of the owner so cached authorization can continue.
    pub async fn fetch(self) -> Result<AuthenticatedPortableEnrollment, PortableEnrollmentError> {
        self.check_current(IcOps::now_nanos())?;
        let outcome =
            crate::ops::auth::enrollment::fetch_snapshot(self.binding.authority, self.challenge)
                .await;
        self.check_current(IcOps::now_nanos())?;
        let snapshot = outcome?;
        Ok(AuthenticatedPortableEnrollment {
            pull: self,
            snapshot,
        })
    }

    fn check_current(&self, now_ns: u64) -> Result<(), PortableEnrollmentError> {
        if self.boot_generation != BOOT_GENERATION.with(Cell::get) {
            return Err(PortableEnrollmentError::Unavailable);
        }
        if self.binding != protected_binding()? {
            return Err(PortableEnrollmentError::Binding);
        }
        if let Err(error) = observe_time(now_ns) {
            invalidate_receiver_boot();
            return Err(error);
        }
        if now_ns >= self.receipt_deadline_ns {
            return Err(PortableEnrollmentError::Expired);
        }
        Ok(())
    }
}

// Enrollment refreshes share the receiver's protected clock and physical accounting.
fn observe_time(now_ns: u64) -> Result<(), PortableEnrollmentError> {
    LocalApplicationAuthorizationStateOps::observe_application_time(now_ns).map_err(|error| {
        match error {
            ApplicationSessionStateError::ClockRollback => PortableEnrollmentError::Clock,
            _ => PortableEnrollmentError::Storage,
        }
    })
}
