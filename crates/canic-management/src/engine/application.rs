//! Application arguments share the existing creation journal, artifact lease and timers.
use super::{Artifact, STATE, delivery, snapshot};
use canic::topology::directory::ServiceInstall;
use canic_management_protocol::{
    Approval, Refusal, Status, application, children::ChildId, stored,
};
use ic_platform_runtime::system as api;

pub(super) fn lifecycle(slot: ChildId) -> Option<application::Lifecycle> {
    STATE.with_borrow(
        |state| match &state.as_ref().expect("open engine").artifact.0[&slot] {
            Artifact::Application(_, value) => Some(value.clone()),
            _ => None,
        },
    )
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
            value.creation.into_vec()
        }
    })
}

pub(super) fn require_legacy() {
    assert!(
        ChildId::ALL
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
pub fn stage(slot: ChildId, input: application::Stage<Approval>) -> Result<Status, Refusal> {
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
    if let Ok(binding) = canic::topology::Install::decode(input.arguments.as_slice()) {
        binding
            .check_root(api::canister_self())
            .map_err(|_| Refusal::Binding)?;
        // A declared parent receives authority over this child. Check Root's
        // own installation evidence before reserving capacity or cycles.
        let parent = binding.parent.to_text();
        let owns_parent = STATE.with_borrow(|state| {
            state
                .as_ref()
                .expect("open engine")
                .journal
                .0
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
    if let Ok(named) = ServiceInstall::decode(input.arguments.as_slice())
        && ChildId::ALL
            .into_iter()
            .filter_map(super::directory::named_service)
            .any(|other| other.name == named.name)
    {
        return Err(Refusal::Binding);
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
pub async fn reconcile(slot: ChildId, input: application::Stage<Approval>) -> Status {
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
pub fn custody() -> application::Custody {
    application::Custody {
        root: super::stored_api::root_history_complete(),
        applications: ChildId::ALL.map(lifecycle),
    }
}

#[cfg(feature = "blob")]
pub(super) fn upgrade_matches(slot: ChildId, input: &application::Upgrade) -> bool {
    STATE.with_borrow(|state| {
        let state = state.as_ref().expect("open engine");
        let journal = &state.journal.0[&slot];
        let Artifact::Application(sources, lifecycle) = &state.artifact.0[&slot] else {
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
pub fn stage_upgrade(
    slot: ChildId,
    input: application::Upgrade,
) -> Result<canic_management_protocol::upgrade::Status, Refusal> {
    super::upgrade::stage_input(slot, super::upgrade::Input::Application(input))
}

/// Reconcile only this complete maintenance operation; never activate implicitly.
/// # Panics
/// Traps for unauthorized or mismatched input.
#[cfg(feature = "blob")]
pub async fn reconcile_upgrade(
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
