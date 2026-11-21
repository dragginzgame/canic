//! Retained fleet format and journal locations; callers use identified collections.
use super::{ChildId, FleetChild, FleetPlan, invalid};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u32,
    root: crate::Plan,
    child: Child,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sibling: Option<Child>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    additional: Vec<Child>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    store: Option<super::FleetStore>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Child {
    deployment: crate::DeploymentName,
    parent: super::CreatedCanister,
    artifact: crate::Artifact,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    release: Option<ic_wasm_store_protocol::Manifest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    receiver_role: Option<String>,
    creation_cycles: u64,
    execute_at_ns: u64,
}
impl Child {
    fn into_child(self, slot: ChildId) -> FleetChild {
        FleetChild {
            slot,
            deployment: self.deployment,
            parent: self.parent,
            artifact: self.artifact,
            release: self.release,
            receiver_role: self.receiver_role,
            creation_cycles: self.creation_cycles,
            execute_at_ns: self.execute_at_ns,
        }
    }

    fn from_child(c: &FleetChild) -> Self {
        Self {
            deployment: c.deployment.clone(),
            parent: c.parent.clone(),
            artifact: c.artifact.clone(),
            release: c.release.clone(),
            receiver_role: c.receiver_role.clone(),
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
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut children = self.children.iter().map(Child::from_child);
        let record = Record {
            version: self.version,
            root: self.root.clone(),
            child: children
                .next()
                .ok_or_else(|| serde::ser::Error::custom(invalid()))?,
            sibling: children.next(),
            additional: children.collect(),
            store: self.store.clone(),
        };
        record.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for FleetPlan {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let r = Record::deserialize(deserializer)?;
        let mut children = vec![r.child.into_child(ChildId::FIRST)];
        children.extend(r.sibling.map(|c| c.into_child(ChildId::SECOND)));
        for child in r.additional {
            let slot = ChildId::ALL
                .get(children.len())
                .ok_or_else(|| serde::de::Error::custom(invalid()))?;
            children.push(child.into_child(*slot));
        }
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

const LEAVES: [&str; ChildId::ALL.len()] = [
    "child", "sibling", "child-3", "child-4", "child-5", "child-6", "child-7", "child-8",
];

// Existing journal paths never move, including records with retained signed attempts.
pub(super) fn child_leaf(slot: ChildId) -> &'static str {
    LEAVES[slot.service_index().expect("permanent child record")]
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
    .chain(
        LEAVES[2..]
            .iter()
            .map(|&leaf| (leaf, &["module.wasm"][..], false)),
    )
}
