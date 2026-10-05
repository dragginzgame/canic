//! Module: workflow::component_registry::caller_authority
//!
//! Caller publication driven by the original Component membership operation.
//!
//! Admission stays receiver-local; this workflow only runs at membership boundaries.

use super::{
    ComponentRegistryOps, ConfigOps, FleetActivationWorkflow, InternalError, root_authority,
};
use crate::{
    ops::component_registry::caller_authority::{CallerReceiverPlan, RootCallerOps},
    view::component_registry::caller_authority::{
        CallerJournalPhase, CallerLifecycleScope, CallerRecipientKind,
    },
};
use candid::{CandidType, Principal};
use canic_core::{
    control_plane_support::{
        model::caller_authority::CallerReceiptPhase,
        ops::{caller_authority::CallerAuthorityOps, ic::call::CallOps},
        policy::caller_authority::matches_permission,
    },
    dto::{
        caller_authority::{
            CallerAuthorityChange, CallerAuthorityCommand, CallerAuthorityPublication,
            CallerAuthorityReceipt, CallerAuthorityStatus,
        },
        error::Error,
        role::{OperationReceipt, OperationStatusRequest},
    },
    ids::{
        CallerInstallation, CallerReceiverAuthority, CallerRootAuthority, ManagedCanisterBinding,
    },
    protocol,
};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(CandidType)]
enum Command {
    CallerAuthority(CallerAuthorityCommand),
    ReleaseApplicationStartup(CallerAuthorityPublication),
}
#[derive(CandidType, Deserialize)]
enum CommandResponse {
    CallerAuthority(Box<CallerAuthorityReceipt>),
    OperationAccepted(OperationReceipt),
}
#[derive(CandidType)]
enum StatusRequest {
    CallerAuthority(OperationStatusRequest),
}
#[derive(CandidType, Deserialize)]
enum StatusResponse {
    CallerAuthority(CallerAuthorityStatus),
}

pub fn restore() -> Result<(), InternalError> {
    RootCallerOps::restore(&issuer()?)
}

fn issuer() -> Result<CallerRootAuthority, InternalError> {
    let (root, _) = root_authority()?;
    Ok(CallerRootAuthority {
        registry: root.binding.authority,
        root: root.binding.fleet_subnet_root,
        install_id: FleetActivationWorkflow::status()?.identity.operation_id,
    })
}

/// Query protected receiver authority and framework readiness without waiting for application hooks.
pub(super) async fn framework_ready(
    binding: &ManagedCanisterBinding,
    install_id: [u8; 32],
    component_install_id: [u8; 32],
) -> Result<(), InternalError> {
    let receiver = CallerInstallation {
        binding: binding.clone(),
        install_id,
        component_install_id,
    };
    let expected = receiver_authority(receiver, issuer()?)?;
    let observed = status(expected.receiver.canister(), [0; 32]).await?;
    if observed.authority != expected {
        return Err(InternalError::conflict());
    }
    if observed.readiness
        == canic_core::dto::caller_authority::CallerAuthorityReadiness::FrameworkPending
    {
        return Err(InternalError::unavailable());
    }
    Ok(())
}

/// Freeze all initial descendants together; a child of a Prepared Component waits for this release.
pub(super) async fn prepare_activation(
    source: CallerInstallation,
    top_level: bool,
) -> Result<(), InternalError> {
    let operation = source.install_id;
    let scope = if top_level {
        CallerLifecycleScope::ActivateComponent(source.clone())
    } else {
        CallerLifecycleScope::ActivateChild(source.clone())
    };
    let issuer = issuer()?;
    if let Some(original) = RootCallerOps::operation_view(operation) {
        if original.scope != scope || original.issuer != issuer {
            return Err(InternalError::conflict());
        }
    } else {
        crate::workflow::root_admission::require_catalog_mutation_for(Some(operation))?;
        let current = ComponentRegistryOps::caller_sources(
            top_level.then_some(source.component().component),
        )?;
        let new_sources: Vec<_> = if top_level {
            current
                .iter()
                .filter(|entry| entry.component().component == source.component().component)
                .cloned()
                .collect()
        } else {
            vec![source]
        };
        let plans = activation_plans(&issuer, &new_sources, &current)?;
        RootCallerOps::begin_publication(operation, issuer, scope, plans)?;
    }
    drive(operation, false).await
}

fn receiver_authority(
    receiver: CallerInstallation,
    issuer: CallerRootAuthority,
) -> Result<CallerReceiverAuthority, InternalError> {
    let policy = ConfigOps::caller_policy(receiver.role())?;
    Ok(CallerReceiverAuthority {
        receiver,
        issuer,
        policy_digest: policy.digest,
    })
}

fn accepts(
    receiver: &CallerReceiverAuthority,
    source: &CallerInstallation,
) -> Result<bool, InternalError> {
    let policy = ConfigOps::caller_policy(receiver.receiver.role())?;
    Ok(policy.configuration.as_ref().is_some_and(|config| {
        config
            .permissions
            .keys()
            .any(|name| matches_permission(&policy, name, &receiver.receiver, source))
    }))
}

fn activation_plans(
    issuer: &CallerRootAuthority,
    sources: &[CallerInstallation],
    current: &[CallerInstallation],
) -> Result<Vec<CallerReceiverPlan>, InternalError> {
    let mut plans = BTreeMap::new();
    for source in sources {
        let authority = receiver_authority(source.clone(), issuer.clone())?;
        plans.insert(
            source.canister(),
            CallerReceiverPlan {
                authority,
                base_generation: 0,
                enroll: true,
                retire: false,
                before: Vec::new(),
                after: Vec::new(),
            },
        );
    }
    for source in sources {
        for role in ConfigOps::caller_receiver_roles(source)? {
            for receiver in RootCallerOps::receiver_views(&role)? {
                if accepts(&receiver.authority, source)? {
                    plans
                        .entry(receiver.authority.receiver.canister())
                        .or_insert(CallerReceiverPlan {
                            authority: receiver.authority,
                            base_generation: receiver.generation,
                            enroll: false,
                            retire: false,
                            before: Vec::new(),
                            after: Vec::new(),
                        });
                }
            }
        }
    }
    for plan in plans.values_mut() {
        let mut grants = BTreeMap::new();
        for source in sources.iter().chain(current.iter().filter(|_| plan.enroll)) {
            if accepts(&plan.authority, source)? {
                grants.insert(source.canister(), source.clone());
            }
        }
        for source in grants.into_values() {
            plan.before
                .push(CallerAuthorityChange::StageSource(source.clone()));
            plan.after.push(CallerAuthorityChange::Grant(source));
        }
        if plan.enroll {
            plan.after.push(CallerAuthorityChange::OpenReceiver);
        }
    }
    Ok(plans.into_values().collect())
}

/// Open staged source rows only after the canonical Registry commit returned successfully.
pub(super) async fn publish_activation(operation: [u8; 32]) -> Result<(), InternalError> {
    RootCallerOps::mark_membership_committed(operation)?;
    drive(operation, true).await
}

/// Release every original enrollment only after the entire immutable census is published.
pub(super) async fn release_startup(operation: [u8; 32]) -> Result<(), InternalError> {
    let current = RootCallerOps::operation_view(operation).ok_or_else(InternalError::invariant)?;
    if current.phase == CallerJournalPhase::Compacted {
        return Ok(());
    }
    if current.phase == CallerJournalPhase::Compacting {
        return compact(operation);
    }
    if current.phase == CallerJournalPhase::Complete {
        require_applications_ready(operation).await?;
        return compact(operation);
    }
    if current.phase != CallerJournalPhase::Published {
        return Err(InternalError::unavailable());
    }
    for ordinal in 0..current.recipient_count {
        let recipient = RootCallerOps::recipient_view(operation, ordinal)?;
        if recipient.startup_released {
            continue;
        }
        let release = RootCallerOps::release_receipt(operation, ordinal)?
            .ok_or_else(InternalError::invariant)?;
        let release_id = release.operation_id;
        let target = recipient.authority.receiver.canister();
        match command(
            target,
            Command::ReleaseApplicationStartup(CallerAuthorityOps::publication_to_dto(release)),
        )
        .await?
        {
            CommandResponse::OperationAccepted(receipt) if receipt.operation_id == release_id => {}
            _ => return Err(InternalError::conflict()),
        }
        RootCallerOps::mark_startup_released(operation, ordinal)?;
    }
    RootCallerOps::complete(operation)?;
    require_applications_ready(operation).await?;
    compact(operation)
}

async fn require_applications_ready(operation: [u8; 32]) -> Result<(), InternalError> {
    let current = RootCallerOps::operation_view(operation).ok_or_else(InternalError::invariant)?;
    for ordinal in 0..current.recipient_count {
        let recipient = RootCallerOps::recipient_view(operation, ordinal)?;
        if recipient.kind != CallerRecipientKind::Enrollment {
            continue;
        }
        let observed = status(recipient.authority.receiver.canister(), operation).await?;
        if observed.authority != recipient.authority {
            return Err(InternalError::conflict());
        }
        if observed.readiness
            != canic_core::dto::caller_authority::CallerAuthorityReadiness::ApplicationReady
        {
            let release = RootCallerOps::release_receipt(operation, ordinal)?
                .ok_or_else(InternalError::invariant)?;
            let release_id = release.operation_id;
            match command(
                recipient.authority.receiver.canister(),
                Command::ReleaseApplicationStartup(CallerAuthorityOps::publication_to_dto(release)),
            )
            .await?
            {
                CommandResponse::OperationAccepted(receipt)
                    if receipt.operation_id == release_id => {}
                _ => return Err(InternalError::conflict()),
            }
            return Err(InternalError::unavailable());
        }
    }
    Ok(())
}

async fn drive(operation: [u8; 32], publish: bool) -> Result<(), InternalError> {
    for _ in 0..48 {
        let current = RootCallerOps::next_phase_view(operation)?;
        match current.phase {
            CallerJournalPhase::Complete
            | CallerJournalPhase::Compacting
            | CallerJournalPhase::Compacted
            | CallerJournalPhase::Published => return Ok(()),
            CallerJournalPhase::Prepared if !publish => return Ok(()),
            CallerJournalPhase::Prepared => return Err(InternalError::conflict()),
            CallerJournalPhase::Reserving => {
                let recipient = RootCallerOps::recipient_view(operation, current.cursor)?;
                let policy = ConfigOps::caller_policy(recipient.authority.receiver.role())?;
                let (entries, bytes) = policy.configuration.map_or((8, 65_536), |config| {
                    (config.maximum_entries, config.maximum_bytes)
                });
                let observed = status(recipient.authority.receiver.canister(), operation).await?;
                RootCallerOps::reserve(operation, &observed, entries, bytes)?;
            }
            CallerJournalPhase::Preparing | CallerJournalPhase::Publishing => {
                let recipient = RootCallerOps::recipient_view(operation, current.cursor)?;
                let through = if current.phase == CallerJournalPhase::Preparing {
                    recipient.before_count
                } else {
                    recipient.step_count
                };
                if recipient.completed_steps == through {
                    RootCallerOps::advance_recipient(operation)?;
                    continue;
                }
                let step =
                    RootCallerOps::step_view(operation, current.cursor, recipient.completed_steps)?;
                let publication = CallerAuthorityOps::publication_to_dto(step.publication);
                let request = match step.phase {
                    None => CallerAuthorityCommand::Prepare(publication),
                    Some(CallerReceiptPhase::Prepared) => {
                        CallerAuthorityCommand::Commit(publication)
                    }
                    Some(CallerReceiptPhase::Committed) => {
                        CallerAuthorityCommand::Complete(publication)
                    }
                    Some(CallerReceiptPhase::Complete) => return Err(InternalError::invariant()),
                };
                let CommandResponse::CallerAuthority(receipt) = command(
                    recipient.authority.receiver.canister(),
                    Command::CallerAuthority(request),
                )
                .await?
                else {
                    return Err(InternalError::conflict());
                };
                RootCallerOps::acknowledge(
                    operation,
                    current.cursor,
                    recipient.completed_steps,
                    *receipt,
                )?;
            }
        }
    }
    Err(InternalError::unavailable())
}

async fn status(
    target: Principal,
    operation_id: [u8; 32],
) -> Result<CallerAuthorityStatus, InternalError> {
    let response = CallOps::bounded_wait(target, protocol::CANIC_CONTROL_STATUS)
        .with_arg(StatusRequest::CallerAuthority(OperationStatusRequest {
            operation_id,
        }))?
        .execute()
        .await
        .map_err(|_| InternalError::unavailable())?;
    let result: Result<StatusResponse, Error> =
        response.candid().map_err(|_| InternalError::invariant())?;
    let StatusResponse::CallerAuthority(status) = result.map_err(InternalError::observed_public)?;
    Ok(status)
}

async fn command(target: Principal, command: Command) -> Result<CommandResponse, InternalError> {
    let response = CallOps::bounded_wait(target, protocol::CANIC_COMMAND)
        .with_arg(command)?
        .execute()
        .await
        .map_err(|_| InternalError::unavailable())?;
    let result: Result<CommandResponse, Error> =
        response.candid().map_err(|_| InternalError::invariant())?;
    result.map_err(InternalError::observed_public)
}

/// Publish exact group or subtree denial while Registry membership still remains active.
pub(super) async fn prepare_denial(
    operation: [u8; 32],
    source: CallerInstallation,
    whole_component: bool,
) -> Result<(), InternalError> {
    let scope = if whole_component {
        CallerLifecycleScope::DenyComponent(source.clone())
    } else {
        CallerLifecycleScope::DenySubtree(source.clone())
    };
    let issuer = issuer()?;
    if let Some(original) = RootCallerOps::operation_view(operation) {
        if original.scope != scope || original.issuer != issuer {
            return Err(InternalError::conflict());
        }
    } else {
        RootCallerOps::require_mutation_allowed(Some(operation))?;
        let current = ComponentRegistryOps::caller_sources(Some(source.component().component))?;
        let sources = select_denied_sources(&source, whole_component, current)?;
        let plans = denial_plans(&source, whole_component, &sources)?;
        RootCallerOps::begin_publication(operation, issuer, scope, plans)?;
    }
    drive(operation, false).await
}

fn select_denied_sources(
    source: &CallerInstallation,
    whole: bool,
    current: Vec<CallerInstallation>,
) -> Result<Vec<CallerInstallation>, InternalError> {
    let current: Vec<_> = current
        .into_iter()
        .filter(|entry| entry.component() == source.component())
        .collect();
    if whole {
        return Ok(current);
    }
    let mut selected = std::collections::BTreeSet::from([source.canister()]);
    loop {
        let before = selected.len();
        for entry in &current {
            if let ManagedCanisterBinding::ComponentChild(child) = &entry.binding
                && selected.contains(&child.parent_canister_id)
            {
                selected.insert(entry.canister());
            }
        }
        if selected.len() == before {
            break;
        }
    }
    let selected: Vec<_> = current
        .into_iter()
        .filter(|entry| selected.contains(&entry.canister()))
        .collect();
    if !selected.contains(source) {
        return Err(InternalError::conflict());
    }
    Ok(selected)
}

fn denial_plans(
    root_source: &CallerInstallation,
    whole: bool,
    sources: &[CallerInstallation],
) -> Result<Vec<CallerReceiverPlan>, InternalError> {
    let mut plans = BTreeMap::new();
    for source in sources {
        let receiver = RootCallerOps::receiver_view(source.canister())
            .ok_or_else(InternalError::unavailable)?;
        if receiver.retired {
            continue;
        }
        plans.insert(
            source.canister(),
            CallerReceiverPlan {
                authority: receiver.authority,
                base_generation: receiver.generation,
                enroll: false,
                retire: true,
                before: Vec::new(),
                after: Vec::new(),
            },
        );
    }
    for source in sources {
        for receiver in RootCallerOps::source_receiver_views(source)? {
            plans
                .entry(receiver.authority.receiver.canister())
                .or_insert(CallerReceiverPlan {
                    authority: receiver.authority,
                    base_generation: receiver.generation,
                    enroll: false,
                    retire: false,
                    before: Vec::new(),
                    after: Vec::new(),
                });
        }
    }
    for plan in plans.values_mut() {
        if whole {
            plan.before.push(CallerAuthorityChange::DenyComponent(
                canic_core::ids::CallerComponentInstallation {
                    binding: root_source.component().clone(),
                    install_id: root_source.component_install_id,
                },
            ));
        } else {
            for source in sources {
                if accepts(&plan.authority, source)? {
                    plan.before
                        .push(CallerAuthorityChange::DenySource(source.clone()));
                }
            }
        }
        if plan.retire {
            plan.before.push(CallerAuthorityChange::RetireReceiver);
        }
    }
    Ok(plans.into_values().collect())
}

/// The enclosing Registry commit is the only authority to complete a denial journal.
pub(super) async fn finish_denial(operation: [u8; 32]) -> Result<(), InternalError> {
    RootCallerOps::mark_membership_committed(operation)?;
    drive(operation, true).await?;
    RootCallerOps::complete(operation)?;
    compact(operation)
}

/// Derived subtree work retains the exact enclosing Component denial instead of issuing a second fence.
pub(super) async fn finish_removal(
    component: canic_core::ids::ComponentInstanceId,
    operation: [u8; 32],
) -> Result<(), InternalError> {
    if let Some(journal) = RootCallerOps::operation_view(operation) {
        if !matches!(journal.scope, CallerLifecycleScope::DenySubtree(source) if source.component().component == component)
        {
            return Err(InternalError::conflict());
        }
        return finish_denial(operation).await;
    }
    let draining = ComponentRegistryOps::component_draining(component)?
        .ok_or_else(InternalError::unavailable)?;
    let journal = RootCallerOps::operation_view(draining.operation_id)
        .ok_or_else(InternalError::unavailable)?;
    if !matches!(journal.scope, CallerLifecycleScope::DenyComponent(source) if source.component().component == component)
    {
        return Err(InternalError::conflict());
    }
    finish_denial(draining.operation_id).await
}

fn compact(operation: [u8; 32]) -> Result<(), InternalError> {
    if RootCallerOps::compact(operation)? {
        Ok(())
    } else {
        Err(InternalError::unavailable())
    }
}
