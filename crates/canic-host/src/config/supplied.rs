//! Resolve supplied infrastructure once into the ordinary slot used by every consumer.
use super::{AppConfig, BuildInput, CanisterDefinition, Config, FleetDeclaration};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceConfig {
    pub apps: BTreeMap<String, SourceApp>,
}

#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceApp {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub wasm_store: bool,
    #[serde(default)]
    pub canisters: BTreeMap<String, CanisterDefinition>,
    #[serde(default)]
    pub fleets: BTreeMap<String, FleetDeclaration>,
}

/// Resolve Root whenever the app uses managed infrastructure. A plain app can build independently.
fn supply_root(app: &mut AppConfig, store: bool) {
    let slots = &mut app.canisters;
    if slots.contains_key("root")
        || slots
            .values()
            .any(|slot| slot.role.as_deref() == Some("root"))
    {
        return;
    }
    if !store
        && !slots
            .values()
            .any(|slot| slot.parent.as_deref() == Some("root"))
    {
        return;
    }
    let mut manifests = slots
        .values()
        .filter_map(|slot| slot.build.as_ref().map(|r| &r.manifest));
    let manifest = manifests
        .next()
        .filter(|first| manifests.all(|other| other == *first))
        .cloned()
        .unwrap_or_else(|| PathBuf::from("Cargo.toml"));
    let mut features = std::collections::BTreeSet::from(["child-metrics"]);
    for fleet in app.fleets.values() {
        for (family, selected) in [
            (
                "auth",
                !fleet.auth.is_empty() || fleet.receiver_policy.is_some(),
            ),
            ("role-issuer", !fleet.role_receivers.is_empty()),
            (
                "child-funding",
                fleet.top_up_children.iter().any(|edge| {
                    slots.get(edge.child()).and_then(|c| c.parent.as_deref()) == Some("root")
                }),
            ),
            (
                "child-creation",
                fleet.registry.is_some()
                    || !fleet.shard_pools.is_empty()
                    || !fleet.worker_pools.is_empty(),
            ),
            (
                "reserve-pool",
                fleet.standby_pool.is_some()
                    || fleet
                        .worker_pools
                        .iter()
                        .any(|pool| pool.minimum_reuse_cycles.is_some()),
            ),
        ] {
            if selected {
                features.insert(family);
            }
        }
    }
    slots.insert(
        "root".into(),
        CanisterDefinition {
            role: Some("root".into()),
            parent: None,
            wasm: Some("artifacts/root.wasm".into()),
            build: Some(BuildInput {
                manifest,
                package: "canic-root".into(),
                features: features.into_iter().map(str::to_owned).collect(),
                client: Some("canic-root-client".into()),
            }),
        },
    );
}

impl TryFrom<SourceConfig> for Config {
    type Error = Error;

    fn try_from(source: SourceConfig) -> Result<Self> {
        let mut apps = BTreeMap::new();
        for (name, source) in source.apps {
            let mut app = AppConfig {
                canisters: source.canisters,
                fleets: source.fleets,
            };
            supply_root(&mut app, source.wasm_store);
            if source.wasm_store {
                let slots = &mut app.canisters;
                let invalid = |reason: &str| {
                    Error::Invalid(format!("apps.{name}.wasm_store = true; {reason}"))
                };
                if slots.contains_key("store")
                    || slots
                        .values()
                        .any(|slot| slot.role.as_deref() == Some("wasm-store"))
                {
                    return Err(invalid(
                        "conflicts with an explicit store slot or wasm-store role",
                    ));
                }
                let mut roots = slots.iter().filter(|(_, slot)| {
                    slot.role.as_deref() == Some("root") && slot.parent.is_none()
                });
                let Some((root_name, root)) = roots.next() else {
                    return Err(invalid("expected one top-level canister with role root"));
                };
                if roots.next().is_some() {
                    return Err(invalid(
                        "multiple top-level canisters have role root; expected one",
                    ));
                }
                let recipe = root.build.as_ref().ok_or_else(|| {
                    invalid(&format!("canisters.{root_name}.build is missing; the Store uses Root's locked workspace"))
                })?;
                let store = CanisterDefinition {
                    role: Some("wasm-store".into()),
                    parent: Some(root_name.clone()),
                    wasm: Some("artifacts/wasm_store.wasm".into()),
                    build: Some(BuildInput {
                        manifest: recipe.manifest.clone(),
                        package: "ic-wasm-store".into(),
                        features: Vec::new(),
                        client: None,
                    }),
                };
                slots.insert("store".into(), store);
            }
            apps.insert(name, app);
        }
        Ok(Self { apps })
    }
}
