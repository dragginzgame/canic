//! Retained fleet format and journal locations; callers use identified collections.
use super::{ChildId, FleetChild, FleetPlan, invalid};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ServiceRecord {
    contract: String,
    version: u32,
    root: crate::Plan,
    children: Vec<FleetChild>,
    store: super::FleetStore,
}

impl Serialize for FleetPlan {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        self.validate().map_err(serde::ser::Error::custom)?;
        let record = ServiceRecord {
            contract: super::CONTRACT.into(),
            version: self.version,
            root: self.root.clone(),
            children: self.children.clone(),
            store: self.store.clone(),
        };
        record.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for FleetPlan {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let r = ServiceRecord::deserialize(deserializer)?;
        if r.contract != super::CONTRACT {
            return Err(serde::de::Error::custom(invalid()));
        }
        let value = Self {
            version: r.version,
            root: r.root,
            children: r.children,
            store: r.store,
        };
        value.validate().map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}

// Preserve the first two historical paths; later paths use the service ordinal.
pub(super) fn child_leaf(slot: ChildId) -> String {
    match slot.service_index().expect("permanent child record") {
        0 => "child".into(),
        1 => "sibling".into(),
        index => format!("child-{}", index + 1),
    }
}
