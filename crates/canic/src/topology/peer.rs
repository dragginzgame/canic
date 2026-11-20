//! Receiver-enforced, directional permissions for named application methods.
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};

/// One immutable direction and exported method under one fleet Root.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Binding {
    /// Installed calling hub.
    pub caller: Principal,
    /// Installed receiving hub.
    pub receiver: Principal,
    /// Export selected by the application.
    pub method: String,
}

impl Binding {
    /// Check that the binding addresses distinct canisters and a bounded method.
    /// # Errors
    /// Refuses invalid principals, a loop, or an invalid endpoint name.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.caller == self.receiver
            || [self.caller, self.receiver]
                .iter()
                .any(|p| p.as_slice().last() != Some(&1))
            || self.method.is_empty()
            || self.method.len() > 64
            || !self
                .method
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_')
        {
            return Err("invalid peer binding");
        }
        Ok(())
    }
}

/// An exact operator decision. Generations start at one and increase per binding.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Decision {
    /// Exact endpoints and method.
    pub binding: Binding,
    /// Per-binding generation.
    pub generation: u64,
    /// Whether this peer may currently call the method.
    pub allow: bool,
}

/// Evidence retained by Root for an individual decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Status {
    /// Root has committed the decision and lacks an exact receiver acknowledgement.
    Pending,
    /// The receiver acknowledged this exact decision.
    Applied,
}

/// Root-to-receiver update. The receiver echoes the exact committed decision.
#[cfg(feature = "method")]
pub struct ApplyPeer;

#[cfg(feature = "method")]
impl crate::method::Contract for ApplyPeer {
    const NAME: &'static str = "apply_peer";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    type Input = Decision;
    type Output = Result<Decision, String>;
}

#[cfg(feature = "storage")]
pub use retained::{Permissions, Ticket};

#[cfg(feature = "storage")]
mod retained {
    use super::{Binding, Decision, Principal, Status};
    use ic_storage::{MemoryBacking, MemoryRuntime, adapter::StableCell};

    const LIMITS: ic_wire::Limits = super::super::directory::DIRECTORY_LIMITS;
    type Rows = Vec<(Decision, Status)>;

    /// Local admission captured before an await.
    pub struct Ticket(Decision);

    /// Bounded receiver or Root ledger under a key in the owner's single bootstrap.
    pub struct Permissions<M: MemoryBacking> {
        cell: StableCell<Vec<u8>, M>,
        installer: Principal,
    }

    impl<M: MemoryBacking> Permissions<M> {
        /// Attach at fresh installation or strictly reopen the existing ledger.
        /// # Panics
        /// Traps on missing, unreadable or incompatible retained state.
        #[must_use]
        pub fn attach(runtime: &MemoryRuntime<M>, key: &str, fresh: bool) -> Self {
            let initial = fresh.then(|| {
                ic_wire::encode(
                    &(ic_platform_runtime::system::caller(), Rows::new()),
                    ic_wire::Encoding::Payload,
                    LIMITS,
                )
                .expect("empty permissions")
            });
            let cell = crate::storage::cell(runtime, key, LIMITS.bytes, initial);
            let (installer, rows): (Principal, Rows) =
                ic_wire::decode(cell.get(), ic_wire::Encoding::Payload, LIMITS)
                    .expect("retained permissions");
            assert!(rows.len() <= 16, "peer permission capacity");
            for (index, (decision, _)) in rows.iter().enumerate() {
                decision.binding.validate().expect("retained peer binding");
                assert!(
                    decision.generation > 0
                        && !rows[..index]
                            .iter()
                            .any(|(d, _)| d.binding == decision.binding),
                    "invalid retained peer decision"
                );
            }
            Self { cell, installer }
        }

        fn rows(&self) -> Rows {
            ic_wire::decode::<(Principal, Rows)>(
                self.cell.get(),
                ic_wire::Encoding::Payload,
                LIMITS,
            )
            .expect("retained permissions")
            .1
        }

        fn save(&mut self, rows: &Rows) {
            self.cell.set(
                ic_wire::encode(&(self.installer, rows), ic_wire::Encoding::Payload, LIMITS)
                    .expect("bounded permissions"),
            );
        }

        /// Whether this ledger has no decisions.
        #[must_use]
        pub fn is_empty(&self) -> bool {
            self.rows().is_empty()
        }

        /// Read the last decision and delivery evidence for this direction.
        #[must_use]
        pub fn latest(&self, binding: &Binding) -> Option<(Decision, Status)> {
            self.rows().into_iter().find(|(d, _)| d.binding == *binding)
        }

        /// Record an operator decision before dispatch, preserving exact retries.
        /// # Errors
        /// Refuses stale, conflicting or nonconsecutive decisions and capacity exhaustion.
        pub fn prepare(&mut self, decision: &Decision) -> Result<Status, &'static str> {
            decision.binding.validate()?;
            let mut rows = self.rows();
            if let Some((old, status)) =
                rows.iter_mut().find(|(d, _)| d.binding == decision.binding)
            {
                if old == decision {
                    return Ok(*status);
                }
                if old.generation.checked_add(1) != Some(decision.generation)
                    || old.allow == decision.allow
                    || (*status == Status::Pending && decision.allow)
                {
                    return Err("conflicting peer decision");
                }
                *old = decision.clone();
                *status = Status::Pending;
            } else {
                if decision.generation != 1 || !decision.allow {
                    return Err("expected initial peer grant");
                }
                if rows.len() >= 16 {
                    return Err("peer permission capacity");
                }
                rows.push((decision.clone(), Status::Pending));
            }
            self.save(&rows);
            Ok(Status::Pending)
        }

        /// Commit an acknowledgement only for the current exact decision.
        /// # Errors
        /// Refuses an acknowledgement superseded while the call was outstanding.
        pub fn acknowledge(&mut self, decision: &Decision) -> Result<(), &'static str> {
            let mut rows = self.rows();
            let (_, status) = rows
                .iter_mut()
                .find(|(d, _)| d == decision)
                .ok_or("superseded peer decision")?;
            if *status == Status::Pending {
                *status = Status::Applied;
                self.save(&rows);
            }
            Ok(())
        }

        /// Accept only the retained Root's delivery addressed to this receiver.
        /// # Errors
        /// Refuses wrong authority, target, malformed or stale decisions.
        pub fn receive(&mut self, decision: Decision) -> Result<Decision, &'static str> {
            super::super::require_sender(self.installer)?;
            let b = &decision.binding;
            b.validate()?;
            if b.receiver != ic_platform_runtime::system::canister_self()
                || decision.generation == 0
            {
                return Err("invalid peer decision");
            }
            let mut rows = self.rows();
            if let Some((old, status)) = rows.iter_mut().find(|(d, _)| d.binding == *b) {
                if old == &decision {
                    return Ok(decision);
                }
                if old.generation >= decision.generation {
                    return Err("stale peer decision");
                }
                *old = decision.clone();
                *status = Status::Applied;
            } else {
                if rows.len() >= 16 {
                    return Err("peer permission capacity");
                }
                rows.push((decision.clone(), Status::Applied));
            }
            self.save(&rows);
            Ok(decision)
        }

        /// Admit the actual caller for the method selected by the handler.
        /// # Errors
        /// Refuses missing or revoked permission.
        #[cfg(feature = "method")]
        pub fn admit<C: crate::method::Contract>(&self) -> Result<Ticket, &'static str> {
            let binding = Binding {
                caller: ic_platform_runtime::system::caller(),
                receiver: ic_platform_runtime::system::canister_self(),
                method: C::NAME.into(),
            };
            let (decision, Status::Applied) = self.latest(&binding).ok_or("peer denied")? else {
                return Err("peer denied");
            };
            if !decision.allow {
                return Err("peer denied");
            }
            Ok(Ticket(decision))
        }

        /// Fence effects after an await, including after revoke and regrant.
        /// # Errors
        /// Refuses a ticket from any earlier decision.
        pub fn revalidate(&self, ticket: &Ticket) -> Result<(), &'static str> {
            if self.latest(&ticket.0.binding) == Some((ticket.0.clone(), Status::Applied)) {
                Ok(())
            } else {
                Err("peer permission changed")
            }
        }
    }
}
