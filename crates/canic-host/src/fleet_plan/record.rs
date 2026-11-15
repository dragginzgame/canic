//! Retained fleet format and journal locations; callers use identified collections.
use super::{FleetChild, FleetPlan, Slot, invalid};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u32,
    root: crate::Plan,
    child: Child,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sibling: Option<Child>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    store: Option<super::FleetStore>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Child {
    deployment: crate::DeploymentName,
    parent: super::CreatedCanister,
    artifact: crate::Artifact,
    creation_cycles: u64,
    execute_at_ns: u64,
}
impl Child {
    fn into_child(self, slot: Slot) -> FleetChild {
        FleetChild {
            slot,
            deployment: self.deployment,
            parent: self.parent,
            artifact: self.artifact,
            creation_cycles: self.creation_cycles,
            execute_at_ns: self.execute_at_ns,
        }
    }
    fn from_child(c: &FleetChild) -> Self {
        Self {
            deployment: c.deployment.clone(),
            parent: c.parent.clone(),
            artifact: c.artifact.clone(),
            creation_cycles: c.creation_cycles,
            execute_at_ns: c.execute_at_ns,
        }
    }
}
impl Serialize for FleetPlan {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let record = match self.children.as_slice() {
            [first] if first.slot == Slot::First && self.version == 1 => Record {
                version: self.version,
                root: self.root.clone(),
                child: Child::from_child(first),
                sibling: None,
                store: self.store.clone(),
            },
            [first, second]
                if first.slot == Slot::First
                    && second.slot == Slot::Second
                    && self.version == 2 =>
            {
                Record {
                    version: self.version,
                    root: self.root.clone(),
                    child: Child::from_child(first),
                    sibling: Some(Child::from_child(second)),
                    store: self.store.clone(),
                }
            }
            _ => return Err(serde::ser::Error::custom(invalid())),
        };
        record.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for FleetPlan {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let r = Record::deserialize(deserializer)?;
        let mut children = vec![r.child.into_child(Slot::First)];
        children.extend(r.sibling.map(|c| c.into_child(Slot::Second)));
        let value = Self {
            version: r.version,
            root: r.root,
            children,
            store: r.store,
        };
        value.validate().map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}
// Existing journal paths never move, including records with retained signed attempts.
pub(super) fn child_leaf(slot: Slot) -> &'static str {
    match slot {
        Slot::First => "child",
        Slot::Second => "sibling",
    }
}

/// Every historical effect directory, including the optional second slot.
pub(super) fn effect_directories()
-> impl Iterator<Item = (&'static str, &'static [&'static str], bool)> {
    [
        ("root", &["module.wasm", "plan.json"][..], true),
        ("child", &["module.wasm"][..], true),
        ("sibling", &["module.wasm"][..], false),
        ("store", &["module.wasm"][..], false),
    ]
    .into_iter()
}
