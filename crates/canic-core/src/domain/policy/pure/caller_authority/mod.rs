//! Module: domain::policy::pure::caller_authority
//!
//! Pure local managed-caller admission and retained-ticket decisions.
//!
//! Inputs are borrowed authoritative projections. No lookup or platform call occurs here.

use crate::{
    config::caller_authority::{CallerPermissionDirection, CallerScope, CompiledCallerPolicy},
    ids::{CallerInstallation, CallerReceiverAuthority},
    model::caller_authority::CallerAdmissionError,
    view::caller_authority::CallerAdmissionView,
};
use candid::Principal;

///
/// CallerAdmissionTicket
///
/// Retained authority for the next new effect after an await.
/// Fields are private so callers cannot expand or replace issued admission.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallerAdmissionTicket {
    receiver: CallerReceiverAuthority,
    source: CallerInstallation,
    generation: u64,
    permission: String,
}

///
/// CallerTargetTicket
///
/// Exact permitted destination; this ticket never authenticates an incoming caller.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallerTargetTicket(CallerAdmissionTicket);

impl CallerTargetTicket {
    /// Destination whose current installation was selected from protected authority.
    #[must_use]
    pub const fn target(&self) -> Principal {
        self.0.caller()
    }
}

impl CallerAdmissionTicket {
    /// The original transport caller, retained for effect attribution.
    #[must_use]
    pub const fn caller(&self) -> Principal {
        self.source.canister()
    }

    /// Named compiled operation class admitted by the endpoint.
    #[must_use]
    pub fn permission(&self) -> &str {
        &self.permission
    }
}

/// Admit the actual IC transport caller against one committed local projection.
pub fn admit(
    caller: Principal,
    permission: &str,
    policy: &CompiledCallerPolicy,
    view: &CallerAdmissionView<'_>,
) -> Result<CallerAdmissionTicket, CallerAdmissionError> {
    require_permission(caller, permission, policy, view)?;
    let source = view.source.ok_or(CallerAdmissionError::PermissionDenied)?;
    Ok(CallerAdmissionTicket {
        receiver: view.receiver.clone(),
        source: source.clone(),
        generation: view.generation,
        permission: permission.to_owned(),
    })
}

/// Evaluate ordinary endpoint admission using borrowed evidence only.
pub fn require_permission(
    caller: Principal,
    permission: &str,
    policy: &CompiledCallerPolicy,
    view: &CallerAdmissionView<'_>,
) -> Result<(), CallerAdmissionError> {
    validate_receiver(policy, view)?;
    require_direction(permission, CallerPermissionDirection::Caller, policy)?;
    require_peer(caller, permission, policy, view)
}

/// Select a destination independently of authentication of the proxy's incoming request.
pub fn select_target(
    target: Principal,
    permission: &str,
    policy: &CompiledCallerPolicy,
    view: &CallerAdmissionView<'_>,
) -> Result<CallerTargetTicket, CallerAdmissionError> {
    validate_receiver(policy, view)?;
    require_direction(permission, CallerPermissionDirection::Target, policy)?;
    require_peer(target, permission, policy, view)?;
    Ok(CallerTargetTicket(CallerAdmissionTicket {
        receiver: view.receiver.clone(),
        source: view
            .source
            .ok_or(CallerAdmissionError::PermissionDenied)?
            .clone(),
        generation: view.generation,
        permission: permission.to_owned(),
    }))
}

/// Recheck the selected destination immediately before a new proxy effect after an await.
pub fn revalidate_target(
    ticket: &CallerTargetTicket,
    policy: &CompiledCallerPolicy,
    view: &CallerAdmissionView<'_>,
) -> Result<(), CallerAdmissionError> {
    require_ticket(&ticket.0, view)?;
    require_direction(
        &ticket.0.permission,
        CallerPermissionDirection::Target,
        policy,
    )?;
    require_peer(ticket.target(), &ticket.0.permission, policy, view)
}

fn require_direction(
    permission: &str,
    expected: CallerPermissionDirection,
    policy: &CompiledCallerPolicy,
) -> Result<(), CallerAdmissionError> {
    if policy
        .configuration
        .as_ref()
        .and_then(|config| config.permissions.get(permission))
        .is_some_and(|permission| permission.direction == expected)
    {
        Ok(())
    } else {
        Err(CallerAdmissionError::PermissionDenied)
    }
}

fn require_peer(
    caller: Principal,
    permission: &str,
    policy: &CompiledCallerPolicy,
    view: &CallerAdmissionView<'_>,
) -> Result<(), CallerAdmissionError> {
    validate_receiver(policy, view)?;
    let source = view.source.ok_or(CallerAdmissionError::PermissionDenied)?;
    if source.canister() != caller || caller == Principal::anonymous() {
        return Err(CallerAdmissionError::PermissionDenied);
    }
    if !view.source_open || view.component_fenced {
        return Err(CallerAdmissionError::Fenced);
    }
    if source.install_id == [0; 32]
        || !matches_permission(policy, permission, &view.receiver.receiver, source)
    {
        return Err(CallerAdmissionError::PermissionDenied);
    }
    Ok(())
}

/// Recheck before a new protected effect; reconciliation keeps the original paid authority.
pub fn revalidate(
    ticket: &CallerAdmissionTicket,
    policy: &CompiledCallerPolicy,
    view: &CallerAdmissionView<'_>,
) -> Result<(), CallerAdmissionError> {
    require_ticket(ticket, view)?;
    require_permission(ticket.caller(), ticket.permission(), policy, view)
}

fn require_ticket(
    ticket: &CallerAdmissionTicket,
    view: &CallerAdmissionView<'_>,
) -> Result<(), CallerAdmissionError> {
    if ticket.receiver != *view.receiver
        || ticket.generation != view.generation
        || view.source != Some(&ticket.source)
    {
        return Err(CallerAdmissionError::TicketExpired);
    }
    Ok(())
}

/// Resolve exact pairs and scope without treating source claims as authority.
#[must_use]
pub fn matches_permission(
    policy: &CompiledCallerPolicy,
    permission: &str,
    receiver: &CallerInstallation,
    source: &CallerInstallation,
) -> bool {
    let Some(permission) = policy
        .configuration
        .as_ref()
        .and_then(|configuration| configuration.permissions.get(permission))
    else {
        return false;
    };
    let receiver_component = receiver.component();
    let source_component = source.component();
    let same_root = receiver_component.authority == source_component.authority
        && receiver_component.fleet_subnet_root == source_component.fleet_subnet_root
        && receiver_component.placement_subnet == source_component.placement_subnet;
    let scope_matches = match permission.scope {
        CallerScope::SameComponent => {
            same_root
                && receiver_component == source_component
                && receiver.component_install_id == source.component_install_id
        }
        CallerScope::SameRoot => same_root,
    };
    scope_matches
        && permission.sources.iter().any(|selector| {
            selector.component_spec == source_component.component_spec
                && selector.role == *source.role()
        })
}

fn validate_receiver(
    policy: &CompiledCallerPolicy,
    view: &CallerAdmissionView<'_>,
) -> Result<(), CallerAdmissionError> {
    let receiver = &view.receiver.receiver;
    let issuer = &view.receiver.issuer;
    let binding_matches = receiver.component().authority == issuer.registry
        && receiver.component().fleet_subnet_root == issuer.root;
    let installation_present = receiver.install_id != [0; 32]
        && issuer.install_id != [0; 32]
        && receiver.component_install_id != [0; 32];
    let policy_matches =
        policy.digest == view.receiver.policy_digest && policy.role == *receiver.role();
    if !binding_matches || !installation_present || !policy_matches {
        return Err(CallerAdmissionError::AuthorityConflict);
    }
    if !view.receiver_open || view.generation == 0 {
        return Err(CallerAdmissionError::Fenced);
    }
    Ok(())
}
