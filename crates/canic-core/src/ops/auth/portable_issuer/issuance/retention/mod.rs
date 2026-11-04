//! Bounded issuer history reclamation and independent immutable login windows.
//!
//! Closing the current operation epoch and deleting its history share one commit.
//! No clock or caller input can reopen an absent login from a closed epoch.

use crate::model::auth::portable_issuer::{
    PortableIssuancePolicy, PortableIssuanceRecord, PortableIssuerJournalRecord,
    PortableIssuerLoginBinding, PortableIssuerLoginRecord,
};
use canic_contracts::dto::portable_issuance::{
    IssuePortableCredential, PortableCredentialIssuanceRefusal as E, PortableIssuanceMaintenance,
};
use ic_platform::Principal;

pub const MAX_LOGINS: usize = 64;

pub(super) const fn binding(
    caller: Principal,
    request: &IssuePortableCredential,
) -> PortableIssuerLoginBinding {
    PortableIssuerLoginBinding {
        caller,
        epoch: request.login_epoch,
        login_id: request.login_id,
    }
}

pub(super) fn references(row: &PortableIssuanceRecord, login: &PortableIssuerLoginRecord) -> bool {
    binding(row.caller, &row.request) == login.binding
}

/// Select before reclamation so an expired current-epoch login cannot be recreated.
pub(super) fn select_login(
    journal: &PortableIssuerJournalRecord,
    caller: Principal,
    request: &IssuePortableCredential,
    policy: PortableIssuancePolicy,
    now_ns: u64,
) -> Result<(PortableIssuerLoginRecord, bool), E> {
    let binding = binding(caller, request);
    if let Some(login) = journal.logins.iter().find(|login| login.binding == binding) {
        if login.browser_public_key != request.browser_public_key {
            return Err(E::Conflict);
        }
        if now_ns >= login.deadline_ns {
            return Err(E::Expired);
        }
        return Ok((login.clone(), false));
    }
    if request.login_epoch != journal.issuance_epoch {
        return Err(E::EpochClosed);
    }
    Ok((
        PortableIssuerLoginRecord {
            binding,
            browser_public_key: request.browser_public_key,
            started_at_ns: now_ns,
            deadline_ns: now_ns.checked_add(policy.login_ttl_ns).ok_or(E::Capacity)?,
        },
        true,
    ))
}

fn expired_login_without_retained_work(
    journal: &PortableIssuerJournalRecord,
    login: &PortableIssuerLoginRecord,
    now_ns: u64,
) -> bool {
    login.deadline_ns <= now_ns
        && !journal
            .issuances
            .iter()
            .any(|row| now_ns < row.remove_at_ns && references(row, login))
}

/// Close a still-current epoch before forgetting it; already closed history needs no new epoch.
pub(super) fn maintain(
    journal: &mut PortableIssuerJournalRecord,
    now_ns: u64,
) -> Result<PortableIssuanceMaintenance, E> {
    let remove_issuances = journal
        .issuances
        .iter()
        .any(|row| now_ns >= row.remove_at_ns);
    let remove_logins = journal
        .logins
        .iter()
        .any(|login| expired_login_without_retained_work(journal, login, now_ns));
    let previous_issuances = journal.issuances.len();
    let previous_logins = journal.logins.len();
    if remove_issuances || remove_logins {
        let closes_current =
            journal.issuances.iter().any(|row| {
                row.remove_at_ns <= now_ns && row.request.epoch == journal.issuance_epoch
            }) || journal.logins.iter().any(|login| {
                login.binding.epoch == journal.issuance_epoch
                    && expired_login_without_retained_work(journal, login, now_ns)
            });
        let next_epoch = if closes_current {
            journal.issuance_epoch.checked_add(1).ok_or(E::Capacity)?
        } else {
            journal.issuance_epoch
        };
        journal.issuances.retain(|row| now_ns < row.remove_at_ns);
        journal.logins.retain(|login| {
            login.deadline_ns > now_ns || journal.issuances.iter().any(|row| references(row, login))
        });
        journal.issuance_epoch = next_epoch;
    }
    Ok(PortableIssuanceMaintenance {
        epoch: journal.issuance_epoch,
        removed_issuances: u32::try_from(previous_issuances - journal.issuances.len())
            .map_err(|_| E::Storage)?,
        removed_logins: u32::try_from(previous_logins - journal.logins.len())
            .map_err(|_| E::Storage)?,
    })
}
