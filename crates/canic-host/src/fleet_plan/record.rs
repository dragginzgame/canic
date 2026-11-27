//! Retained fleet format and journal locations; callers use identified collections.
use super::{ChildId, FleetPlan, invalid};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ServiceRecord<Name, Root, Children, Store> {
    contract: Name,
    version: u32,
    root: Root,
    children: Children,
    store: Store,
}

impl Serialize for FleetPlan {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        self.validate().map_err(serde::ser::Error::custom)?;
        let record = ServiceRecord {
            contract: super::CONTRACT,
            version: self.version,
            root: &self.root,
            children: &self.children,
            store: &self.store,
        };
        record.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for FleetPlan {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let r = ServiceRecord::<String, _, _, _>::deserialize(deserializer)?;
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

pub(super) fn child_leaf(slot: ChildId) -> String {
    format!(
        "child-{}",
        slot.service_index().expect("permanent child record") + 1
    )
}
