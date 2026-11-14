use canic_host::{Fleet, FleetCanister};

pub(super) fn role<'a>(snapshot: &'a Fleet, target: &FleetCanister, links: &[String]) -> &'a str {
    if let Some(name) = name(snapshot, &target.network_id, &target.canister)
        && let Some(role) = snapshot
            .configuration
            .as_ref()
            .and_then(|config| config.config.canisters.get(&name))
            .and_then(|slot| slot.role.as_deref())
    {
        return role;
    }
    if let Some(manager) = snapshot.managers.iter().find(|manager| {
        manager.network_id == target.network_id && manager.manager == target.canister
    }) {
        return if super::super::manager_kind(snapshot, manager) == "Root" {
            "root"
        } else {
            "manager"
        };
    }
    if let Some(manager) = snapshot.managers.iter().find(|manager| {
        manager.network_id == target.network_id
            && manager
                .reported
                .as_ref()
                .is_some_and(|status| status.worker.as_ref() == Some(&target.canister))
    }) {
        return if super::super::manager_kind(snapshot, manager) == "Root" {
            "child"
        } else {
            "worker"
        };
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
                return "entry";
            } else if target.canister == worker {
                return "worker";
            }
        }
    }
    "—"
}

pub(super) fn name(snapshot: &Fleet, network: &str, principal: &str) -> Option<String> {
    let config = snapshot.configuration.as_ref()?;
    if config.config.network.binding_id().ok().as_deref() != Some(network) {
        return None;
    }
    let names: Vec<_> = config
        .config
        .canisters
        .keys()
        .filter(|slot| super::super::slot_state(snapshot, slot).identity() == Some(principal))
        .cloned()
        .collect();
    match names.as_slice() {
        [name] => Some(name.clone()),
        _ => None,
    }
}

pub(super) fn is_controller(snapshot: &Fleet, target: &FleetCanister) -> bool {
    snapshot.canisters.iter().any(|child| {
        child.network_id == target.network_id
            && child.canister != target.canister
            && child.observation_error.is_none()
            && child
                .controllers
                .as_ref()
                .is_some_and(|controllers| controllers.contains(&target.canister))
    })
}
