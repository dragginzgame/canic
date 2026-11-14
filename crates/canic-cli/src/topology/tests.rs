use super::*;
use canic_host::{CanisterConfig, Network};
use std::collections::BTreeMap;

fn config() -> FleetConfig {
    FleetConfig {
        network: Network::Mainnet,
        journal: None,
        canisters: [
            ("root", None),
            ("app", Some("root")),
            ("leaf", Some("app")),
            ("worker", Some("app")),
            ("other", Some("root")),
        ]
        .into_iter()
        .map(|(slot, parent)| {
            (
                slot.into(),
                CanisterConfig {
                    role: Some("repeated-role".into()),
                    parent: parent.map(String::from),
                    ..Default::default()
                },
            )
        })
        .collect::<BTreeMap<_, _>>(),
    }
}

#[test]
fn nested_tree_preserves_immediate_parents_and_repeated_roles() {
    let config = config();
    let before = config.clone();
    assert_eq!(
        rows(&config),
        [
            ("root", "".into()),
            ("app", "├─ ".into()),
            ("leaf", "│  ├─ ".into()),
            ("worker", "│  └─ ".into()),
            ("other", "└─ ".into()),
        ]
    );
    assert_eq!(config, before);
}

#[test]
fn partial_or_malformed_snapshot_keeps_every_slot_visible_once() {
    let mut config = config();
    config.canisters.remove("root");
    assert_eq!(rows(&config).len(), 4);
    config.canisters.get_mut("app").unwrap().parent = Some("leaf".into());
    let rows = rows(&config);
    assert_eq!(rows.len(), 4);
    assert_eq!(
        rows.iter()
            .map(|(slot, _)| slot)
            .collect::<BTreeSet<_>>()
            .len(),
        4
    );
}
