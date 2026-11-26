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
    #[serde(default)]
    pub canisters: BTreeMap<String, CanisterDefinition>,
    #[serde(default)]
    pub fleets: BTreeMap<String, FleetDeclaration>,
}

/// Resolve the mandatory Root in the application build workspace.
fn supply_root(app: &mut AppConfig) {
    let slots = &mut app.canisters;
    if slots.contains_key("root")
        || slots
            .values()
            .any(|slot| slot.role.as_deref() == Some("root"))
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
    slots.insert(
        "root".into(),
        CanisterDefinition {
            role: Some("root".into()),
            parent: None,
            wasm: Some("artifacts/root.wasm".into()),
            build: Some(BuildInput {
                manifest,
                package: "canic-root".into(),
                features: Vec::new(),
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
            supply_root(&mut app);
            let slots = &mut app.canisters;
            let invalid = |reason: &str| Error::Invalid(format!("apps.{name}.canisters: {reason}"));
            let mut roots = slots
                .iter()
                .filter(|(_, slot)| slot.role.as_deref() == Some("root") && slot.parent.is_none());
            let Some((root_name, root)) = roots.next() else {
                return Err(invalid("expected one top-level canister with role root"));
            };
            if roots.next().is_some() {
                return Err(invalid(
                    "multiple top-level canisters have role root; expected one",
                ));
            }
            let manifest = root.build.as_ref().map_or_else(
                || PathBuf::from("Cargo.toml"),
                |recipe| recipe.manifest.clone(),
            );
            let parent = root_name.clone();
            let store = CanisterDefinition {
                role: Some("wasm-store".into()),
                parent: Some(parent),
                wasm: Some("artifacts/wasm_store.wasm".into()),
                build: Some(BuildInput {
                    manifest,
                    package: "ic-wasm-store".into(),
                    features: Vec::new(),
                    client: None,
                }),
            };
            if slots.get("store").is_some_and(|declared| {
                declared.role != store.role || declared.parent != store.parent
            }) || slots
                .iter()
                .any(|(name, slot)| name != "store" && slot.role.as_deref() == Some("wasm-store"))
            {
                return Err(invalid("conflicts with the mandatory supplied store slot"));
            }
            slots.entry("store".into()).or_insert(store);
            apps.insert(name, app);
        }
        Ok(Self { apps })
    }
}
