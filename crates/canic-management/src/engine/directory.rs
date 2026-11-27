//! Publish the directory derived from permanent ready application slots.
use super::{Artifact, STATE, snapshot};
use canic::topology::directory::{
    Delivery, Directory, Hop, ReceiveDirectory, RedeliveryError, Service, ServiceInstall,
};
use canic_management_protocol::children::ChildId;
use ic_platform_types::Principal;
use ic_timers::{Lifetime, Observation, OnceDemand, OnceNext, OnceResult};

enum Binding {
    Service(ServiceInstall),
    Child(canic::topology::Install),
}

fn binding(artifact: &Artifact) -> Option<Binding> {
    let Artifact::Application(_, lifecycle) = artifact else {
        return None;
    };
    let bytes = lifecycle.creation.as_slice();
    let root = ic_platform_runtime::system::canister_self();
    if let Ok(service) = ServiceInstall::approved(bytes, root) {
        return Some(Binding::Service(service));
    }
    canic::topology::Install::approved(bytes)
        .ok()
        .flatten()
        .filter(|child| child.root == root)
        .map(Binding::Child)
}

pub(super) fn named_service(slot: ChildId) -> Option<ServiceInstall> {
    slot.service_index()?;
    STATE.with_borrow(|state| {
        match binding(
            state
                .as_ref()
                .expect("open engine")
                .artifact
                .rows()
                .get(&slot)?,
        )? {
            Binding::Service(service) => Some(service),
            Binding::Child(_) => None,
        }
    })
}

/// Project ready named services from permanent ordinary install records.
/// A terminal ready record and its approved service name are immutable. The number of
/// published names therefore increases strictly whenever this first directory changes.
/// # Errors
/// Refuses empty directories and placement-only owners.
/// # Panics
/// Traps if the management engine has not opened.
fn service_directory() -> Result<Directory, RedeliveryError> {
    let mut services = Vec::new();
    for slot in super::service_slots() {
        let status = snapshot(slot);
        if status.phase == canic_management_protocol::Phase::Ready
            && let Some(named) = named_service(slot)
        {
            services.push(Service {
                name: named.name,
                role: named.role,
                canister: super::worker(&status),
            });
        }
    }
    if services.is_empty() {
        return Err(RedeliveryError::NoReadyServices);
    }
    services.sort_by(|a, b| a.name.cmp(&b.name));
    let directory = Directory {
        revision: services.len() as u64,
        services,
    };
    directory
        .validate()
        .map_err(|_| RedeliveryError::InvalidDirectory)?;
    Ok(directory)
}

// Build paths only from committed installations, using their retained parent and role.
fn paths(recipient: &Service) -> Vec<Vec<Hop>> {
    STATE.with_borrow(|state| {
        let state = state.as_ref().expect("open engine");
        let mut children = std::collections::BTreeMap::<Principal, Vec<Hop>>::new();
        let mut service = None;
        for (slot, journal) in state.journal.rows() {
            #[cfg(feature = "sharding")]
            if state.pools.withdrawn(*slot) {
                continue;
            }
            if journal.0.phase != canic_management_protocol::Phase::Ready {
                continue;
            }
            let artifact = &state.artifact.rows()[slot];
            let Artifact::Application(source, _) = artifact else {
                continue;
            };
            let canister = super::worker(&journal.0);
            let generation = source.creation.operation;
            if canister == recipient.canister {
                service = Some(Hop {
                    shard_pool: None,
                    canister,
                    generation,
                    role: recipient.role.clone(),
                });
            } else if let Some(Binding::Child(binding)) = binding(artifact) {
                children.entry(binding.parent).or_default().push(Hop {
                    #[cfg(feature = "sharding")]
                    shard_pool: state.pools.shard_pool_name(*slot).map(str::to_owned),
                    #[cfg(not(feature = "sharding"))]
                    shard_pool: None,
                    canister,
                    generation,
                    role: binding.role,
                });
            }
        }
        let mut paths = vec![vec![service.expect("ready named installation")]];
        let mut cursor = 0;
        while cursor < paths.len() {
            let path = paths[cursor].clone();
            let parent = path.last().expect("addressed service").canister;
            for child in children.remove(&parent).unwrap_or_default() {
                let mut next = path.clone();
                next.push(child);
                paths.push(next);
            }
            cursor += 1;
        }
        paths
    })
}

/// Retry one service's delivery under its original revision.
/// # Errors
/// Refuses an unknown recipient, superseded revision or unconfirmed delivery.
pub async fn redeliver_directory_revision(
    name: &str,
    revision: u64,
) -> Result<u64, RedeliveryError> {
    let directory = service_directory()?;
    if directory.revision != revision {
        return Err(RedeliveryError::RevisionChanged);
    }
    let recipient = directory
        .services
        .iter()
        .find(|s| s.name == name)
        .ok_or(RedeliveryError::UnknownService)?;
    publish_branch(&directory, recipient, None).await?;
    Ok(revision)
}

// Publish every path in this branch for a directory change or replay. An
// unnamed child's readiness selects only the path ending at that installation.
async fn publish_branch(
    directory: &Directory,
    recipient: &Service,
    child: Option<Principal>,
) -> Result<bool, RedeliveryError> {
    let mut delivered = false;
    let mut result = Ok(());
    for path in paths(recipient) {
        if child.is_some_and(|id| path.last().is_none_or(|hop| hop.canister != id)) {
            continue;
        }
        if service_directory()?.ne(directory) {
            return Err(RedeliveryError::RevisionChanged);
        }
        let delivery = Delivery {
            // Root currently owns manual captures only. Publish an explicit inactive
            // assignment so a recipient can distinguish it from a missed delivery.
            backup_timing: Some(canic::topology::directory::BackupTiming {
                revision: 1,
                daily_at_utc_secs: None,
            }),
            directory: directory.clone(),
            path,
        };
        if matches!(
            canic::method::call::<ReceiveDirectory>(recipient.canister, 5, &delivery).await,
            Ok(Ok(()))
        ) {
            delivered = true;
        } else {
            result = Err(RedeliveryError::DeliveryUnconfirmed);
        }
    }
    result.map(|()| delivered)
}

async fn propagate(revision: u64, child: Option<Principal>) -> OnceResult {
    let mut observation = Observation::NoWork;
    if let Ok(current) = service_directory()
        && current.revision == revision
    {
        for recipient in &current.services {
            match publish_branch(&current, recipient, child).await {
                Err(_) => observation = Observation::ExpectedFailure,
                Ok(true) if observation == Observation::NoWork => {
                    observation = Observation::Success
                }
                Ok(_) => {}
            }
        }
    }
    OnceResult {
        observation,
        next: OnceNext::Stop,
    }
}

/// Arm one bounded pass at first readiness or reconstruct it after Root reopens.
/// A recipient's stable revision makes exact delivery replay harmless.
pub(super) fn schedule(ready: Option<ChildId>) {
    // Two children can join at the same directory revision while a prior send is awaiting a reply.
    let (trigger, child) = if let Some(slot) = ready {
        let target = STATE.with_borrow(|state| {
            let state = state.as_ref().expect("open engine");
            match binding(state.artifact.rows().get(&slot)?)? {
                Binding::Service(_) => Some(None),
                Binding::Child(_) => Some(Some(super::worker(&state.journal.rows()[&slot].0))),
            }
        });
        let Some(child) = target else { return };
        (slot.index().to_string(), child)
    } else {
        ("reopen".into(), None)
    };
    let Ok(directory) = service_directory() else {
        return;
    };
    let revision = directory.revision;
    ic_timers::bind_once(
        ["canic", "directory", &trigger],
        Lifetime::RemoveWhenStopped,
        move || propagate(revision, child),
    )
    .expect("bind directory delivery")
    .set(OnceDemand::At(ic_platform_runtime::system::time()))
    .expect("schedule directory delivery");
}

/// Page existing journal keys in order. The cursor excludes the last scanned slot.
/// # Panics
/// Requires an open Root engine.
#[must_use]
pub fn list_children(
    after: Option<ChildId>,
) -> (
    Vec<canic_management_protocol::children::InstalledChild>,
    Option<ChildId>,
) {
    use canic_management_protocol::children::InstalledChild;
    use std::ops::Bound::{Excluded, Unbounded};
    STATE.with_borrow(|state| {
        let state = state.as_ref().expect("open engine");
        let mut slots = state
            .journal
            .rows()
            .range((after.map_or(Unbounded, Excluded), Unbounded));
        let mut rows = Vec::new();
        let mut last = None;
        for (&slot, journal) in slots.by_ref().take(32) {
            last = Some(slot);
            let Some(canister) = &journal.0.worker else {
                continue;
            };
            #[cfg(feature = "sharding")]
            if state.pools.withdrawn(slot) {
                continue;
            }
            let (role, parent) = match state.artifact.rows().get(&slot).and_then(binding) {
                Some(Binding::Service(service)) => (Some(service.role), None),
                Some(Binding::Child(child)) => (Some(child.role), Some(child.parent.to_text())),
                None => (None, None),
            };
            rows.push(InstalledChild {
                slot,
                canister: canister.clone(),
                role,
                parent,
                phase: journal.0.phase,
            });
        }
        (rows, slots.next().and(last))
    })
}
