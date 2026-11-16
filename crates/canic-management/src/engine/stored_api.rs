//! Explicit Store-bound endpoints; legacy contracts never silently omit supplier custody.
use super::{Artifact, STATE, delivery, snapshot, status, upgrade};
use canic_management_protocol::{
    Approval, Refusal, Status,
    children::{self, Slot},
    stored,
};
use ic_platform_runtime::cdk::api;

/// Admit creation from a published Store artifact without retaining Wasm bytes.
/// # Errors
/// Refuses changed bindings, invalid metadata, time or capacity.
/// # Panics
/// Traps for noncontrollers or failed durable writes.
pub fn stage(slot: Slot, input: stored::Stage<Approval>) -> Result<Status, Refusal> {
    assert!(
        api::is_controller(&api::msg_caller()),
        "controller required"
    );
    if super::store_maintenance::active() {
        return Err(Refusal::Occupied);
    }
    if let Some(previous) = snapshot(slot).approval {
        return if previous == input.approval
            && delivery::source(slot, false)
                .is_some_and(|source| source.reference == input.reference)
        {
            Ok(status(slot))
        } else {
            Err(Refusal::Occupied)
        };
    }
    input
        .approval
        .validate(&api::canister_self().to_text(), api::time())?;
    delivery::admit(&input.reference, &input.approval.module_hash)?;
    let creation = delivery::retained(slot, stored::STAGE, &input);
    super::admit(
        slot,
        input.approval,
        Artifact::Stored(stored::Sources {
            creation,
            upgrades: vec![],
        }),
    )
}
/// Admit a Store-backed successor under the existing bounded upgrade journal.
/// # Errors
/// Refuses changed bindings, incomplete prior work, time or capacity.
/// # Panics
/// Traps for noncontrollers or failed durable writes.
pub fn stage_upgrade(
    slot: Slot,
    input: stored::Stage<canic_management_protocol::upgrade::Approval>,
) -> Result<canic_management_protocol::upgrade::Status, Refusal> {
    upgrade::stage_input(slot, upgrade::Input::Stored(input))
}
pub(super) fn upgrade_source(
    slot: Slot,
    approval: &canic_management_protocol::upgrade::Approval,
) -> Option<stored::Delivery> {
    STATE.with_borrow(|state| {
        let state = state.as_ref().expect("open engine");
        let journal = &state.journal.get().0[&slot];
        let index = journal
            .2
            .iter()
            .chain(std::iter::once(&journal.1))
            .position(|record| record.approval.as_ref() == Some(approval))?;
        state.artifact.get().0[&slot]
            .sources()?
            .upgrades
            .get(index)
            .cloned()
    })
}
/// Reconcile only the exact admitted Store-backed creation.
/// # Panics
/// Traps for noncontrollers or mismatched complete authority.
pub async fn reconcile(slot: Slot, input: stored::Stage<Approval>) -> Status {
    assert!(
        api::is_controller(&api::msg_caller()),
        "controller required"
    );
    assert_eq!(
        delivery::source(slot, false)
            .expect("Store source")
            .reference,
        input.reference,
        "Store binding mismatch"
    );
    super::reconcile_bound(slot, input.approval).await
}
/// Reconcile only the exact admitted Store-backed upgrade, including history lookup.
/// # Panics
/// Traps for noncontrollers or mismatched complete authority.
pub async fn reconcile_upgrade(
    slot: Slot,
    input: stored::Stage<canic_management_protocol::upgrade::Approval>,
) -> canic_management_protocol::upgrade::Status {
    assert!(
        api::is_controller(&api::msg_caller()),
        "controller required"
    );
    assert_eq!(
        upgrade_source(slot, &input.approval)
            .expect("Store upgrade source")
            .reference,
        input.reference,
        "Store binding mismatch"
    );
    upgrade::reconcile_bound(slot, input.approval).await
}
/// Observe all journals and Store sources without scheduling work.
/// # Panics
/// Traps if lifecycle has not opened storage.
#[must_use]
pub fn custody() -> stored::Custody {
    assert!(
        super::store_maintenance::observe().is_none(),
        "Store maintenance custody required"
    );
    stored_journals()
}
/// Observe both child slots, all artifact references and the surviving Store operation.
/// # Panics
/// Traps if lifecycle has not opened storage.
#[must_use]
pub fn root_custody() -> canic_management_protocol::custody::Root {
    assert!(
        STATE.with_borrow(|state| state
            .as_ref()
            .expect("open engine")
            .journal
            .get()
            .2
            .is_empty()),
        "complete Store history required"
    );
    current_root_custody()
}
/// Read all child custody and every retained Store maintenance attempt synchronously.
/// # Panics
/// Traps if lifecycle has not opened storage.
#[must_use]
pub fn root_history() -> canic_management_protocol::custody::RootHistory {
    canic_management_protocol::custody::RootHistory {
        current: current_root_custody(),
        store_history: STATE
            .with_borrow(|state| state.as_ref().expect("open engine").journal.get().2.clone()),
    }
}
fn current_root_custody() -> canic_management_protocol::custody::Root {
    canic_management_protocol::custody::Root {
        stored: stored_journals(),
        store_operation: super::store_maintenance::observe(),
    }
}
fn stored_journals() -> stored::Custody {
    let children: children::Custody = super::journals();
    let sources = STATE.with_borrow(|state| {
        let artifacts = &state.as_ref().expect("open engine").artifact.get().0;
        Slot::ALL.map(|slot| artifacts[&slot].sources().cloned())
    });
    stored::Custody { children, sources }
}
