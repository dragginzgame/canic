//! Give disposable, genuinely completed fixture records the historical declaration shape.
//!
//! This test-only encoder preserves field order and original paid receipts. Production
//! source records are never rewritten; the fixture uses current Wasms with historical records.

use super::*;
use canic_host::fleet_ensure::{
    FleetEnsureReport,
    ops::{read_journal, read_plan},
};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{MapAccess, SeqAccess, Visitor},
    ser::SerializeMap,
};
use std::{collections::BTreeMap, fmt};

#[derive(Clone)]
enum Ordered {
    Object(Vec<(String, Self)>),
    Array(Vec<Self>),
    Scalar(serde_json::Value),
}

impl<'de> Deserialize<'de> for Ordered {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct OrderedVisitor;
        impl<'de> Visitor<'de> for OrderedVisitor {
            type Value = Ordered;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("ordered JSON")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Ordered, A::Error> {
                let mut fields = Vec::new();
                while let Some(entry) = map.next_entry()? {
                    fields.push(entry);
                }
                Ok(Ordered::Object(fields))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Ordered, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element()? {
                    values.push(value);
                }
                Ok(Ordered::Array(values))
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Ordered, E> {
                Ok(Ordered::Scalar(value.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Ordered, E> {
                Ok(Ordered::Scalar(value.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Ordered, E> {
                Ok(Ordered::Scalar(value.into()))
            }
            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Ordered, E> {
                Ok(Ordered::Scalar(value.into()))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Ordered, E> {
                Ok(Ordered::Scalar(serde_json::Value::Null))
            }
        }
        deserializer.deserialize_any(OrderedVisitor)
    }
}

impl Serialize for Ordered {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Object(fields) => {
                let mut map = serializer.serialize_map(Some(fields.len()))?;
                for (name, value) in fields {
                    map.serialize_entry(name, value)?;
                }
                map.end()
            }
            Self::Array(values) => values.serialize(serializer),
            Self::Scalar(value) => value.serialize(serializer),
        }
    }
}

impl Ordered {
    fn normalize(&mut self, projection: &serde_json::Value) {
        match self {
            Self::Object(fields) => {
                for (key, value) in fields {
                    if let Some(projected) = projection.get(key.as_str()) {
                        value.normalize(projected);
                    }
                }
            }
            Self::Array(values) => {
                if let Some(projected) = projection.as_array() {
                    for (value, projected) in values.iter_mut().zip(projected) {
                        value.normalize(projected);
                    }
                }
            }
            Self::Scalar(value) => *value = projection.clone(),
        }
    }

    fn historical(&mut self) {
        match self {
            Self::Object(fields) => {
                fields.retain(|(key, value)| {
                    if key == "recovery_controllers" {
                        assert!(matches!(value, Self::Array(values) if values.is_empty()));
                        false
                    } else {
                        true
                    }
                });
                for (_, value) in fields {
                    value.historical();
                }
            }
            Self::Array(values) => values.iter_mut().for_each(Self::historical),
            Self::Scalar(_) => {}
        }
    }

    fn field_mut(&mut self, key: &str) -> &mut Self {
        let Self::Object(fields) = self else {
            panic!("expected object")
        };
        &mut fields.iter_mut().find(|(name, _)| name == key).unwrap().1
    }

    fn actions(&self, hashes: &mut BTreeMap<String, String>) {
        match self {
            Self::Object(fields) => {
                for (name, value) in fields {
                    if name == "actions" || name == "protocol_actions" {
                        let Self::Array(actions) = value else {
                            panic!("expected actions")
                        };
                        for action in actions {
                            let original = sha256_hex(&serde_json::to_vec(action).unwrap());
                            let mut historical = action.clone();
                            historical.historical();
                            hashes.insert(
                                original,
                                sha256_hex(&serde_json::to_vec(&historical).unwrap()),
                            );
                        }
                    } else {
                        value.actions(hashes);
                    }
                }
            }
            Self::Array(values) => values.iter().for_each(|value| value.actions(hashes)),
            Self::Scalar(_) => {}
        }
    }
}

fn hash(plan: &Ordered) -> String {
    let bytes = serde_json::to_vec(plan).unwrap();
    let mut message = Vec::new();
    for field in [b"canic:fleet-ensure:plan:v1".as_slice(), bytes.as_slice()] {
        message.extend_from_slice(&(field.len() as u64).to_be_bytes());
        message.extend_from_slice(field);
    }
    sha256_hex(&message)
}

fn historical_plan(
    plan: &FleetEnsurePlan,
    hashes: &mut BTreeMap<String, String>,
) -> (String, Vec<u8>) {
    let projection = canic_host::fleet_ensure::report_json_value(&FleetEnsureReport {
        actual_conservation: None,
        effects_applied: 0,
        funding_review: None,
        plan: plan.clone(),
        terminal: true,
    })
    .unwrap();
    let mut ordered: Ordered = serde_json::from_slice(&serde_json::to_vec(plan).unwrap()).unwrap();
    ordered.normalize(&projection["plan"]);
    *ordered.field_mut("plan_sha256") = Ordered::Scalar("".into());
    assert_eq!(
        hash(&ordered),
        plan.plan_sha256,
        "fixture encoder must reproduce current canonical bytes first"
    );
    ordered.actions(hashes);
    ordered.historical();
    let digest = hash(&ordered);
    *ordered.field_mut("plan_sha256") = Ordered::Scalar(digest.clone().into());
    (digest, serde_json::to_vec(&ordered).unwrap())
}

pub(super) fn retain_historical_shape(input: &ReinstallJourney<'_>) {
    let paths = paths(input);
    let plan = read_plan(&paths).unwrap().unwrap();
    let mut journal = read_journal(&paths).unwrap().unwrap();
    assert_eq!(
        journal.completion,
        canic_host::fleet_ensure::model::FleetEnsureCompletion::Converged
    );
    let mut hashes = BTreeMap::new();
    let (digest, bytes) = historical_plan(&plan, &mut hashes);
    journal.plan_sha256 = digest;
    for phase in &mut journal.successor_phases {
        let mut phase_paths = paths.clone();
        phase_paths.plan = paths
            .plan
            .with_file_name("phases")
            .join(format!("{}.json", phase.plan_sha256));
        let plan = read_plan(&phase_paths).unwrap().unwrap();
        let (digest, bytes) = historical_plan(&plan, &mut hashes);
        std::fs::write(
            phase_paths.plan.with_file_name(format!("{digest}.json")),
            bytes,
        )
        .unwrap();
        phase.plan_sha256 = digest;
    }
    for effect in &mut journal.effects {
        effect.action_sha256 = hashes
            .get(&effect.action_sha256)
            .expect("original paid effect bound to source action")
            .clone();
    }
    std::fs::write(&paths.plan, bytes).unwrap();
    std::fs::write(&paths.journal, serde_json::to_vec(&journal).unwrap()).unwrap();
    let mut state: Ordered = serde_json::from_slice(&std::fs::read(&paths.state).unwrap()).unwrap();
    state.historical();
    std::fs::write(&paths.state, serde_json::to_vec(&state).unwrap()).unwrap();
    let release = input.desired.bootstrap.as_ref().unwrap().release_build_id;
    let directory = input
        .adapter_root
        .join(".canic/release-builds")
        .join(release.to_string());
    let manifest = directory.join("current-release-set-manifest.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    assert!(
        value
            .as_object_mut()
            .unwrap()
            .remove("transition_mode")
            .is_some()
    );
    let bytes = serde_json::to_vec(&value).unwrap();
    std::fs::write(&manifest, &bytes).unwrap();
    let plan = directory.join("plan.cbor");
    let mut record: ciborium::value::Value =
        ciborium::de::from_reader(std::fs::read(&plan).unwrap().as_slice()).unwrap();
    record.as_array_mut().unwrap()[5].as_array_mut().unwrap()[1] =
        ciborium::value::Value::Bytes(canic_core::cdk::utils::hash::sha256_bytes(&bytes));
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(&record, &mut bytes).unwrap();
    std::fs::write(plan, bytes).unwrap();
}
