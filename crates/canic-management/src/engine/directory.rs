//! Publish the directory derived from permanent ready application slots.
use super::{Artifact, STATE, snapshot};
use canic::topology::directory::{Directory, ReceiveDirectory, Service, ServiceInstall};
use canic_management_protocol::children::ChildId;
use ic_timers::{Lifetime, Observation, OnceDemand, OnceNext, OnceResult};

pub(super) fn named_service(slot: ChildId) -> Option<ServiceInstall> {
    #[cfg(feature = "sharding")]
    if !super::sharding::ordinary_slot(slot) {
        return None;
    }
    STATE.with_borrow(
        |state| match &state.as_ref().expect("open engine").artifact.0[&slot] {
            Artifact::Application(_, lifecycle) => {
                ServiceInstall::decode(lifecycle.creation.as_slice()).ok()
            }
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

async fn deliver(directory: &Directory, recipient: &Service) -> Result<u64, &'static str> {
    canic::method::call::<ReceiveDirectory>(recipient.canister, 5, directory)
        .await
        .map_err(|_| "directory delivery unconfirmed")?;
    Ok(directory.revision)
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
    deliver(&directory, recipient).await
}

async fn propagate(revision: u64) -> OnceResult {
    let mut observation = Observation::NoWork;
    if let Ok(current) = service_directory()
        && current.revision == revision
    {
        observation = Observation::Success;
        for recipient in &current.services {
            // Recheck the original revision before each send after an await.
            if publish_directory(&recipient.name, revision).await.is_err() {
                observation = Observation::ExpectedFailure;
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
pub(super) fn schedule() {
    let Ok(directory) = service_directory() else {
        return;
    };
    let revision = directory.revision;
    ic_timers::bind_once(
        ["canic", "directory", &revision.to_string()],
        Lifetime::RemoveWhenStopped,
        move || propagate(revision),
    )
    .expect("bind directory delivery")
    .set(OnceDemand::At(ic_platform_runtime::system::time()))
    .expect("schedule directory delivery");
}
