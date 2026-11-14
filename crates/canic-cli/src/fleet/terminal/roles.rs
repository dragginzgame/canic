use super::paint;
use canic_host::{Fleet, FleetCanister};

pub(super) fn badges(
    snapshot: &Fleet,
    target: &FleetCanister,
    links: &[String],
    color: bool,
) -> String {
    let mut labels = Vec::new();
    if snapshot.canisters.iter().any(|child| {
        child.network_id == target.network_id
            && child.canister != target.canister
            && child.observation_error.is_none()
            && child
                .controllers
                .as_ref()
                .is_some_and(|controllers| controllers.contains(&target.canister))
    }) {
        labels.push(paint("[CONTROLLER CANISTER]", "1;35", color));
    }
    if snapshot.managers.iter().any(|manager| {
        manager.network_id == target.network_id && manager.manager == target.canister
    }) {
        labels.push(paint("[MANAGER / selected]", "1;36", color));
    }
    if snapshot.managers.iter().any(|manager| {
        manager.network_id == target.network_id
            && manager
                .reported
                .as_ref()
                .is_some_and(|status| status.worker.as_ref() == Some(&target.canister))
    }) {
        labels.push(paint("[WORKER / reported]", "1;34", color));
    }
    // The caller admits each declared route against one unambiguous network first.
    for link in links {
        if let Some((entry, worker)) = link.split_once(':') {
            if ![entry, worker].iter().all(|principal| {
                snapshot
                    .canisters
                    .iter()
                    .any(|item| item.network_id == target.network_id && item.canister == *principal)
            }) {
                continue;
            }
            if target.canister == entry {
                labels.push(paint("[ENTRY / declared]", "1;34", color));
            } else if target.canister == worker {
                labels.push(paint("[WORKER / declared]", "1;34", color));
            }
        }
    }
    if labels.is_empty() {
        labels.push(paint("[CANISTER]", "1", color));
    }
    labels.dedup();
    labels.join(" ")
}
