use super::{paint, roles};
use canic_host::{Fleet, FleetCanister};
use ic_host_text::{
    escape::terminal_text,
    table::{self, Alignment, Options},
};
use std::{collections::BTreeMap, fmt::Write as _};

pub(super) fn render(
    snapshot: &Fleet,
    canisters: &[&FleetCanister],
    links: &[String],
    details: bool,
    color: bool,
) -> String {
    let mut output = String::new();
    let mut external = BTreeMap::new();
    for target in canisters {
        if target.observation_error.is_none() {
            for controller in target.controllers.iter().flatten() {
                if !canisters.iter().any(|item| item.canister == *controller) {
                    external.insert(controller, String::new());
                }
            }
        }
    }
    for (index, label) in external.values_mut().enumerate() {
        *label = format!("external:{}", index + 1);
    }
    let configuration = snapshot.configuration.as_ref().filter(|config| {
        canisters.first().is_some_and(|target| {
            config.config.network.binding_id().ok().as_deref() == Some(&target.network_id)
        })
    });
    let tree = configuration
        .map(|config| {
            crate::topology::rows(
                config
                    .config
                    .canisters
                    .iter()
                    .map(|(name, slot)| (name.as_str(), slot.parent.as_deref())),
            )
        })
        .unwrap_or_default();
    let mut canisters = canisters.to_vec();
    canisters.sort_by_cached_key(|target| {
        let name = roles::name(snapshot, &target.network_id, &target.canister);
        (
            tree.iter()
                .position(|(slot, _)| Some(*slot) == name.as_deref())
                .unwrap_or(usize::MAX),
            name.unwrap_or_default(),
            target.canister.clone(),
        )
    });
    let rows: Vec<_> = canisters
        .iter()
        .map(|target| {
            let controllers = if target.observation_error.is_some() {
                "unknown".into()
            } else {
                target.controllers.as_ref().map_or_else(
                    || "unknown".into(),
                    |controllers| {
                        if controllers.is_empty() {
                            return "none".into();
                        }
                        controllers
                            .iter()
                            .map(|id| {
                                external.get(id).cloned().unwrap_or_else(|| {
                                    roles::name(snapshot, &target.network_id, id)
                                        .unwrap_or_else(|| id.clone())
                                })
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    },
                )
            };
            let name = roles::name(snapshot, &target.network_id, &target.canister);
            let mut label = name.clone().unwrap_or_else(|| "—".into());
            if let Some(config) = configuration
                && let Some(name) = &name
            {
                let mut parent = config.config.canisters[name].parent.as_deref();
                let mut complete = true;
                let mut visited = std::collections::BTreeSet::new();
                while let Some(slot) = parent {
                    if !visited.insert(slot)
                        || !canisters.iter().any(|candidate| {
                            super::super::slot_state(snapshot, slot).identity()
                                == Some(candidate.canister.as_str())
                        })
                    {
                        complete = false;
                        break;
                    }
                    parent = config
                        .config
                        .canisters
                        .get(slot)
                        .and_then(|slot| slot.parent.as_deref());
                }
                if complete {
                    if let Some((_, prefix)) = tree.iter().find(|(slot, _)| *slot == name) {
                        label = format!("{prefix}{name}");
                    }
                } else {
                    writeln!(
                        output,
                        "Topology: {} has an unresolved parent; shown without nesting.",
                        terminal_text(name)
                    )
                    .expect("writing to String");
                }
            }
            [
                label,
                target.canister.clone(),
                roles::role(snapshot, target, links).into(),
                controllers,
                if target.observation_error.is_none()
                    && target.module_hash.is_some()
                    && target.controllers.is_some()
                {
                    "verified"
                } else {
                    "UNKNOWN"
                }
                .into(),
            ]
        })
        .collect();
    let table = table::render(
        &[
            "CANISTER (DECLARED TREE)",
            "PRINCIPAL",
            "ROLE",
            "CONTROLLERS",
            "CODE / CONTROL",
        ],
        &rows,
        &[Alignment::Left; 5],
        Options {
            column_gap: 2,
            header_separator: true,
        },
    );
    // IC Host owns escaping and alignment. Apply colour only after it has rendered rows.
    for (index, line) in table.lines().enumerate() {
        let style = index
            .checked_sub(2)
            .and_then(|index| rows.get(index).map(|row| (row, canisters[index])))
            .and_then(|(row, target)| {
                if row[4] == "UNKNOWN" {
                    Some("33")
                } else {
                    if roles::is_controller(snapshot, target) {
                        return Some("35");
                    }
                    match row[2].as_str() {
                        "manager" => Some("36"),
                        _ => None,
                    }
                }
            });
        if let Some(style) = style.filter(|_| color) {
            writeln!(output, "\x1b[{style}m{line}\x1b[0m").expect("writing to String");
        } else {
            writeln!(output, "{line}").expect("writing to String");
        }
    }
    if !external.is_empty() {
        output.push('\n');
    }
    for (id, label) in external {
        writeln!(output, "  {label} = {}", terminal_text(id)).expect("writing to String");
    }
    for target in canisters {
        let name = roles::name(snapshot, &target.network_id, &target.canister)
            .unwrap_or_else(|| target.canister.clone());
        if details {
            writeln!(
                output,
                "  {} module: {}",
                terminal_text(&name),
                terminal_text(target.module_hash.as_deref().unwrap_or("unknown"))
            )
            .expect("writing to String");
        }
        if let Some(error) = &target.observation_error {
            writeln!(
                output,
                "  {}: {}",
                terminal_text(&name),
                paint(error, "33", color)
            )
            .expect("writing to String");
        }
    }
    output
}
