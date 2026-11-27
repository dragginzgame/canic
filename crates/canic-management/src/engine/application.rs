//! Application arguments share the existing creation journal, artifact lease and timers.
use super::{Artifact, STATE, delivery, snapshot};
use canic::topology::directory::ServiceInstall;
use canic_management_protocol::{
    Approval, Refusal, Status, application, children::ChildId, stored,
};
use ic_platform_runtime::system as api;

pub(super) fn lifecycle(slot: ChildId) -> Option<application::Lifecycle> {
    STATE.with_borrow(|state| {
        match state
            .as_ref()
            .expect("open engine")
            .artifact
            .rows()
            .get(&slot)?
        {
            Artifact::Application(_, value) => Some(value.clone()),
            _ => None,
        }
    })
}

pub(super) fn arguments(slot: ChildId, upgrade: bool) -> Vec<u8> {
    lifecycle(slot).map_or_else(Vec::new, |value| {
        if upgrade {
            value
                .upgrades
                .last()
                .expect("retained upgrade arguments")
                .arguments
                .as_slice()
                .to_vec()
        } else {
            let bytes = value.creation.into_vec();
            #[cfg(feature = "sharding")]
            if let Some(generation) = super::sharding::worker_generation(slot)
                && let Ok(mut install) = canic::topology::Install::decode(&bytes)
                && (install.application == canic::topology::WORKER_GENERATION
                    || canic::topology::worker::minimum_reuse_cycles(&install.application)
                        .is_some())
            {
                install.application = generation.to_vec();
                return install.encode().expect("approved worker generation");
            }
            bytes
        }
    })
}

pub(super) fn require_legacy() {
    #[cfg(feature = "role-issuer")]
    super::role_issuer::require_unprepared();
    assert!(
        super::service_slots()
            .into_iter()
            .all(|slot| lifecycle(slot).is_none()),
        "application custody required"
    );
}

/// Retain exact initialization bytes under the existing Store creation authority.
/// # Errors
/// Refuses changed arguments, invalid authority or exhausted custody.
/// # Panics
/// Traps for noncontrollers or failed durable writes.
fn stage(slot: ChildId, input: application::Stage<Approval>) -> Result<Status, Refusal> {
    assert!(api::is_controller(&api::caller()), "controller required");
    if super::store_maintenance::active() {
        return Err(Refusal::Occupied);
    }
    if let Some(previous) = snapshot(slot).approval {
        return if previous == input.source.approval
            && lifecycle(slot).is_some_and(|l| l.creation == input.arguments)
            && delivery::source(slot, false).is_some_and(|s| s.reference == input.source.reference)
        {
            Ok(super::status(slot))
        } else {
            Err(Refusal::Occupied)
        };
    }
    if input.arguments.as_slice().is_empty() {
        return Err(Refusal::Binding);
    }
    if let Some(binding) = canic::topology::Install::approved(input.arguments.as_slice())
        .map_err(|_| Refusal::Binding)?
    {
        binding
            .check_root(api::canister_self())
            .map_err(|_| Refusal::Binding)?;
        // A declared parent receives authority over this child. Check Root's
        // own installation evidence before reserving capacity or cycles.
        #[cfg(feature = "sharding")]
        if STATE.with_borrow(|s| {
            s.as_ref()
                .expect("open engine")
                .pools
                .singleton_owns(&binding)
        }) {
            return Err(Refusal::Binding);
        }
        let parent = binding.parent.to_text();
        let owns_parent = STATE.with_borrow(|state| {
            state
                .as_ref()
                .expect("open engine")
                .journal
                .rows()
                .values()
                .any(|journal| {
                    journal.0.phase == canic_management_protocol::Phase::Ready
                        && journal.0.worker.as_ref() == Some(&parent)
                })
        });
        if !owns_parent {
            return Err(Refusal::Binding);
        }
    }
    if let Some(bindings) = canic::topology::directory::DbService::from_receiver(
        input.arguments.as_slice(),
        api::canister_self(),
    )
    .map_err(|_| Refusal::Binding)?
    {
        #[cfg(feature = "role-issuer")]
        let trusted = super::role_issuer::retained_public()
            .is_some_and(|pin| pin.as_slice() == bindings.issuer_key);
        #[cfg(not(feature = "role-issuer"))]
        let trusted = {
            let _ = bindings;
            false
        };
        if !trusted {
            return Err(Refusal::Binding);
        }
    }
    if let Ok(named) = ServiceInstall::approved(input.arguments.as_slice(), api::canister_self()) {
        let mut count = 0;
        for other in super::service_slots()
            .into_iter()
            .filter_map(super::directory::named_service)
        {
            if other.name == named.name {
                return Err(Refusal::Binding);
            }
            count += 1;
        }
        if count >= canic::topology::directory::MAX_SERVICES {
            return Err(Refusal::Binding);
        }
    }
    input
        .source
        .approval
        .validate(&api::canister_self().to_text(), api::time())?;
    delivery::admit(&input.source.reference, &input.source.approval.module_hash)?;
    admit_creation(super::Origin::Permanent, slot, input)
}

/// Caller has already admitted the effect owner, source and exact nonempty bytes.
pub(super) fn admit_creation(
    origin: super::Origin,
    slot: ChildId,
    input: application::Stage<Approval>,
) -> Result<Status, Refusal> {
    let operation = input.pin(application::STAGE, slot).operation;
    let source = stored::Sources {
        creation: stored::Delivery {
            reference: input.source.reference,
            operation,
            state: stored::PinState::Pending,
            cache_deadline: None,
        },
        upgrades: vec![],
    };
    super::admit(
        origin,
        slot,
        input.source.approval,
        Artifact::Application(
            source,
            application::Lifecycle {
                creation: input.arguments,
                upgrades: vec![],
            },
        ),
    )
}

/// Reconcile the original complete creation without renewing authority.
/// # Panics
/// Traps for noncontrollers or changed bindings.
async fn reconcile(slot: ChildId, input: application::Stage<Approval>) -> Status {
    assert!(api::is_controller(&api::caller()), "controller required");
    assert_eq!(
        lifecycle(slot).expect("application custody").creation,
        input.arguments
    );
    assert_eq!(
        delivery::source(slot, false)
            .expect("Store source")
            .reference,
        input.source.reference
    );
    super::reconcile_bound(slot, input.source.approval).await
}

/// Complete Root and application custody; this read never schedules effects.
/// # Panics
/// Traps if storage has not opened.
#[must_use]
pub fn custody(selected: Option<application::DeploymentSelector>) -> application::Custody {
    #[cfg(feature = "role-issuer")]
    super::role_issuer::require_unprepared();
    child_evidence(selected)
}

/// Observe child deployments. This endpoint grants no Root maintenance authority.
/// # Panics
/// Requires open Root storage.
#[must_use]
pub fn child_evidence(selected: Option<application::DeploymentSelector>) -> application::Custody {
    STATE.with_borrow(|state| {
        let state = state.as_ref().expect("open engine");
        if let Some(request) = selected {
            let slot = request.slot;
            assert!(slot.service_index().is_some(), "permanent service required");
            let mut children = Vec::new();
            if let Some(j) = state.journal.rows().get(&slot) {
                let a = &state.artifact.rows()[&slot];
                let pinned = request.value.as_ref().and_then(|approval| {
                    j.2.iter()
                        .chain(std::iter::once(&j.1))
                        .enumerate()
                        .find(|(_, u)| u.approval.as_ref() == Some(approval))
                });
                let sources = a
                    .sources()
                    .map(|s| canic_management_protocol::stored::Sources {
                        creation: s.creation.clone(),
                        upgrades: pinned
                            .map(|(i, _)| s.upgrades[i].clone())
                            .into_iter()
                            .collect(),
                    });
                let lifecycle = if let Artifact::Application(_, l) = a {
                    Some(application::Lifecycle {
                        creation: l.creation.clone(),
                        upgrades: pinned
                            .map(|(i, _)| l.upgrades[i].clone())
                            .into_iter()
                            .collect(),
                    })
                } else {
                    None
                };
                children.push(application::ChildCustody {
                    slot,
                    journal: canic_management_protocol::custody::Custody {
                        creation: j.0.clone(),
                        upgrades: pinned.map(|(_, u)| u.clone()).into_iter().collect(),
                    },
                    sources,
                    lifecycle,
                });
            } else {
                children.push(application::ChildCustody {
                    slot,
                    journal: canic_management_protocol::custody::Custody {
                        creation: Status::default(),
                        upgrades: vec![],
                    },
                    sources: None,
                    lifecycle: None,
                });
            }
            return application::Custody {
                children,
                store_operation: state.metadata.maintenance.clone(),
                store_history: vec![],
            };
        }
        let children = state
            .journal
            .rows()
            .iter()
            .filter(|(id, _)| id.service_index().is_some())
            .map(|(&slot, journal)| application::ChildCustody {
                slot,
                journal: journal.custody(),
                sources: state.artifact.rows()[&slot].sources().cloned(),
                lifecycle: lifecycle(slot),
            })
            .collect();
        application::Custody {
            children,
            store_operation: state.metadata.maintenance.clone(),
            store_history: state.metadata.history.clone(),
        }
    })
}

#[cfg(feature = "blob")]
pub(super) fn upgrade_matches(slot: ChildId, input: &application::Upgrade) -> bool {
    STATE.with_borrow(|state| {
        let state = state.as_ref().expect("open engine");
        let journal = &state.journal.rows()[&slot];
        let Artifact::Application(sources, lifecycle) = &state.artifact.rows()[&slot] else {
            return false;
        };
        journal
            .2
            .iter()
            .chain(std::iter::once(&journal.1))
            .position(|s| s.approval.as_ref() == Some(&input.source.source.approval))
            .is_some_and(|i| {
                lifecycle.upgrades.get(i).is_some_and(|m| {
                    m.attempt == input.attempt && m.arguments == input.source.arguments
                }) && sources.upgrades.get(i).is_some_and(|s| {
                    s.reference == input.source.source.reference && s.pin() == input.pin(slot)
                })
            })
    })
}

/// Admit the supplier's exact maintenance attempt under the existing upgrade owner.
/// # Errors
/// Refuses incompatible application or changed authority.
/// # Panics
/// Traps for noncontrollers and failed durable writes.
#[cfg(feature = "blob")]
fn stage_upgrade(
    slot: ChildId,
    input: application::Upgrade,
) -> Result<canic_management_protocol::upgrade::Status, Refusal> {
    super::upgrade::stage_input(slot, super::upgrade::Input::Application(input))
}

/// Reconcile only this complete maintenance operation; never activate implicitly.
/// # Panics
/// Traps for unauthorized or mismatched input.
#[cfg(feature = "blob")]
async fn reconcile_upgrade(
    slot: ChildId,
    input: application::Upgrade,
) -> canic_management_protocol::upgrade::Status {
    assert!(api::is_controller(&api::caller()), "controller required");
    assert!(
        upgrade_matches(slot, &input),
        "complete application upgrade binding"
    );
    super::upgrade::reconcile_bound(slot, input.source.source.approval).await
}

/// Stage the selected creation under its original custody owner.
/// # Errors
/// Staging refuses a changed or inadmissible operation.
/// # Panics
/// Requires an admitted controller and the configured custody owner.
pub fn stage_child_creation(
    slot: ChildId,
    input: application::Creation,
) -> Result<canic_management_protocol::Status, canic_management_protocol::Refusal> {
    match input {
        application::Deployment::Stored(value) => super::stored_api::stage(slot, value),
        application::Deployment::Application(value) => stage(slot, value),
    }
}

/// Reconcile the selected creation under its original custody owner.
/// # Panics
/// Requires an admitted controller and the configured custody owner.
pub async fn reconcile_child_creation(
    slot: ChildId,
    input: application::Creation,
) -> canic_management_protocol::Status {
    match input {
        application::Deployment::Stored(value) => super::stored_api::reconcile(slot, value).await,
        application::Deployment::Application(value) => reconcile(slot, value).await,
    }
}

/// Stage the selected replacement under its original custody owner.
/// # Errors
/// Staging refuses a changed or inadmissible operation.
/// # Panics
/// Requires an admitted controller and the configured custody owner.
pub fn stage_child_upgrade(
    slot: ChildId,
    input: application::Replacement,
) -> Result<canic_management_protocol::upgrade::Status, canic_management_protocol::Refusal> {
    match input {
        application::Deployment::Stored(value) => super::stored_api::stage_upgrade(slot, value),
        #[cfg(feature = "blob")]
        application::Deployment::Application(value) => {
            #[cfg(feature = "sharding")]
            assert!(!super::sharding::occupied(), "shard pool custody required");
            stage_upgrade(slot, value)
        }
        #[cfg(not(feature = "blob"))]
        application::Deployment::Application(_) => Err(Refusal::Binding),
    }
}

/// Reconcile the selected replacement under its original custody owner.
/// # Panics
/// Requires an admitted controller and the configured custody owner.
pub async fn reconcile_child_upgrade(
    slot: ChildId,
    input: application::Replacement,
) -> canic_management_protocol::upgrade::Status {
    match input {
        application::Deployment::Stored(value) => {
            super::stored_api::reconcile_upgrade(slot, value).await
        }
        #[cfg(feature = "blob")]
        application::Deployment::Application(value) => {
            #[cfg(feature = "sharding")]
            assert!(!super::sharding::occupied(), "shard pool custody required");
            reconcile_upgrade(slot, value).await
        }
        #[cfg(not(feature = "blob"))]
        application::Deployment::Application(_) => {
            ic_platform_runtime::system::trap("application upgrade requires Blob")
        }
    }
}

/// Observe a bounded Root-maintenance page. The revision changes on custody writes and Root upgrade.
/// # Panics
/// Requires an opened Root with admitted maintenance custody.
#[must_use]
pub fn custody_page(key: application::CustodyKey) -> application::CustodyPage {
    use application::{CustodyItem as Item, CustodyKey as Key};
    #[cfg(feature = "role-issuer")]
    super::role_issuer::require_unprepared();
    STATE.with_borrow(|s| {
        let state = s.as_ref().expect("open Root");
        let item = (|| match key {
            Key::Inventory => Some(Item::Inventory {
                children: state.service_count() as u64,
                store_attempts: (state.metadata.history.len()
                    + usize::from(state.metadata.maintenance.is_some()))
                    as u64,
            }),
            Key::Store(n) => {
                let i = usize::try_from(n).ok()?;
                let s = if i == state.metadata.history.len() {
                    state.metadata.maintenance.as_ref()
                } else {
                    state.metadata.history.get(i)
                }?;
                Some(Item::Store(Box::new(s.clone())))
            }
            Key::Child(slot) | Key::Upgrade(slot, _) => {
                slot.service_index()?;
                let j = state.journal.rows().get(&slot)?;
                let a = state.artifact.rows().get(&slot)?;
                let l = if let Artifact::Application(_, l) = a {
                    Some(l)
                } else {
                    None
                };
                if let Key::Upgrade(_, n) = key {
                    let i = usize::try_from(n).ok()?;
                    let status = if i == j.2.len() && j.1.approval.is_some() {
                        &j.1
                    } else {
                        j.2.get(i)?
                    };
                    Some(Item::Upgrade {
                        status: status.clone(),
                        source: a.sources().and_then(|s| s.upgrades.get(i)).cloned(),
                        lifecycle: l.and_then(|l| l.upgrades.get(i)).cloned(),
                    })
                } else {
                    Some(Item::Child {
                        creation: j.0.clone(),
                        source: a.sources().map(|s| s.creation.clone()),
                        arguments: l.map(|l| l.creation.clone()),
                        upgrades: (j.2.len() + usize::from(j.1.approval.is_some())) as u64,
                    })
                }
            }
        })();
        application::CustodyPage {
            revision: (state.open_version, crate::storage::revision()),
            key,
            item,
        }
    })
}
