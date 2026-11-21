//! Publish the directory derived from permanent ready application slots.
use super::{Artifact, STATE, snapshot};
use canic::topology::directory::{
    Delivery, Directory, Hop, ReceiveDirectory, Service, ServiceInstall,
};
use canic_management_protocol::children::ChildId;
use ic_platform_types::Principal;
use ic_timers::{Lifetime, Observation, OnceDemand, OnceNext, OnceResult};

pub(super) fn named_service(slot: ChildId) -> Option<ServiceInstall> {
    if !ChildId::ALL.contains(&slot) {
        return None;
    }
    STATE.with_borrow(
        |state| match &state.as_ref().expect("open engine").artifact.0[&slot] {
            Artifact::Application(_, lifecycle) => ServiceInstall::approved(
                lifecycle.creation.as_slice(),
                ic_platform_runtime::system::canister_self(),
            )
            .ok(),
            _ => None,
        },
    )
}

/// Project ready named services from permanent ordinary install records.
/// A terminal ready record and its approved service name are immutable. The number of
/// published names therefore increases strictly whenever this first directory changes.
/// # Errors
/// Refuses empty directories and placement-only owners.
/// # Panics
/// Traps if the management engine has not opened.
fn service_directory() -> Result<Directory, &'static str> {
    let mut services = Vec::new();
    for slot in ChildId::ALL {
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
        return Err("no ready named services");
    }
    services.sort_by(|a, b| a.name.cmp(&b.name));
    let directory = Directory {
        revision: services.len() as u64,
        services,
    };
    directory.validate()?;
    Ok(directory)
}

// Build paths only from committed installations, using their retained parent and role.
fn paths(recipient: &Service) -> Vec<Vec<Hop>> {
    STATE.with_borrow(|state| {
        let state = state.as_ref().expect("open engine");
        let mut children = Vec::new();
        let mut service = None;
        for (slot, journal) in &state.journal.0 {
            #[cfg(feature = "sharding")]
            if state.pools.withdrawn(*slot) {
                continue;
            }
            if journal.0.phase != canic_management_protocol::Phase::Ready {
                continue;
            }
            let Artifact::Application(source, lifecycle) = &state.artifact.0[slot] else {
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
            } else if let Ok(Some(binding)) =
                canic::topology::Install::approved(lifecycle.creation.as_slice())
                && binding.root == ic_platform_runtime::system::canister_self()
            {
                children.push((
                    binding.parent,
                    Hop {
                        #[cfg(feature = "sharding")]
                        shard_pool: state.pools.shard_pool_name(*slot).map(str::to_owned),
                        #[cfg(not(feature = "sharding"))]
                        shard_pool: None,
                        canister,
                        generation,
                        role: binding.role,
                    },
                ));
            }
        }
        let mut paths = vec![vec![service.expect("ready named installation")]];
        let mut cursor = 0;
        while cursor < paths.len() {
            let path = paths[cursor].clone();
            let parent = path.last().expect("addressed service").canister;
            for (_, child) in children.iter().filter(|(p, _)| *p == parent) {
                let mut next = path.clone();
                next.push(child.clone());
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
pub async fn publish_directory(name: &str, revision: u64) -> Result<u64, &'static str> {
    let directory = service_directory()?;
    if directory.revision != revision {
        return Err("directory revision changed");
    }
    let recipient = directory
        .services
        .iter()
        .find(|s| s.name == name)
        .ok_or("unknown service")?;
    publish_branch(&directory, recipient, None).await?;
    Ok(revision)
}

// Publish every path in this branch for a directory change or replay. An
// unnamed child's readiness selects only the path ending at that installation.
async fn publish_branch(
    directory: &Directory,
    recipient: &Service,
    child: Option<Principal>,
) -> Result<bool, &'static str> {
    let mut delivered = false;
    let mut result = Ok(());
    for path in paths(recipient) {
        if child.is_some_and(|id| path.last().is_none_or(|hop| hop.canister != id)) {
            continue;
        }
        if service_directory()?.ne(directory) {
            return Err("directory revision changed");
        }
        let delivery = Delivery {
            directory: directory.clone(),
            path,
        };
        if matches!(
            canic::method::call::<ReceiveDirectory>(recipient.canister, 5, &delivery).await,
            Ok(Ok(()))
        ) {
            delivered = true;
        } else {
            result = Err("directory delivery unconfirmed");
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
            let Artifact::Application(_, lifecycle) = &state.artifact.0[&slot] else {
                return None;
            };
            let args = lifecycle.creation.as_slice();
            if ServiceInstall::approved(args, ic_platform_runtime::system::canister_self()).is_ok()
            {
                Some(None)
            } else {
                canic::topology::Install::approved(args)
                    .ok()
                    .flatten()
                    .map(|_| Some(super::worker(&state.journal.0[&slot].0)))
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
