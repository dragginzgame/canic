use super::*;

fn parents() -> BTreeMap<&'static str, Option<&'static str>> {
    BTreeMap::from([
        ("root", None),
        ("app", Some("root")),
        ("leaf", Some("app")),
        ("worker", Some("app")),
        ("other", Some("root")),
    ])
}

#[test]
fn nested_tree_preserves_immediate_parents() {
    assert_eq!(
        rows(parents()),
        [
            ("root", "".into()),
            ("app", "├─ ".into()),
            ("leaf", "│  ├─ ".into()),
            ("worker", "│  └─ ".into()),
            ("other", "└─ ".into()),
        ]
    );
}

#[test]
fn partial_or_malformed_snapshot_keeps_every_slot_visible_once() {
    let mut parents = parents();
    parents.remove("root");
    assert_eq!(rows(parents.clone()).len(), 4);
    parents.insert("app", Some("leaf"));
    let rows = rows(parents);
    assert_eq!(rows.len(), 4);
    assert_eq!(
        rows.iter()
            .map(|(slot, _)| slot)
            .collect::<BTreeSet<_>>()
            .len(),
        4
    );
}
