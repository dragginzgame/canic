//! Resolve an operator's shard-pool declaration against the chosen app and fleet.
use super::{CanisterConfig, FleetConfig, name};
use crate::{
    DeploymentName, Error, FleetPlanning, Result, SelectedFleet, artifact, store, supplier,
};
use canic_management_protocol::{children::ChildId, stored};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Fleet-specific limits and explicitly named infrastructure for one shard component.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShardPoolConfig {
    /// Component to allocate. Its declared parent is the requesting hub.
    pub component: String,
    /// Configured effect owner.
    pub root: String,
    /// Configured artifact supplier.
    pub store: String,
    /// Exact release label in the selected Store. Planning computes its manifest from the app Wasm.
    pub release: ic_wasm_store_protocol::Release,
    /// Optional file of exact application initialization bytes, relative to canic.toml.
    pub install_args: Option<PathBuf>,
    /// Maximum lifetime shard count.
    pub max_shards: u8,
    /// Maximum lifetime keys per shard.
    pub keys_per_shard: u16,
    /// Total authorized creation attachments in cycles.
    pub creation_budget: u64,
    /// Final deadline for dispatching new creation work, in IC nanoseconds.
    pub expires_at_ns: Option<u64>,
}
impl ShardPoolConfig {
    pub(crate) fn preview(
        &self,
        config_path: &Path,
        selected: &SelectedFleet,
        controller: &str,
    ) -> Result<FleetPlanning> {
        let principal = crate::network::principal(controller)?;
        if principal.to_text() != controller
            || principal == ic_platform_types::Principal::anonymous()
            || principal.as_slice().is_empty()
        {
            return Err(Error::Invalid(
                "expected a canonical controller principal".into(),
            ));
        }
        if self.deadline()? <= crate::now()? {
            return Err(Error::Invalid("shard growth approval has expired".into()));
        }
        let bytes = store::read(
            &selected.wasm_path(config_path, &self.component, None)?,
            artifact::MAX_ARTIFACT_BYTES,
        )?;
        artifact::admit_bounded(&bytes, artifact::MAX_ARTIFACT_BYTES)?;
        let manifest = crate::publication::release_manifest(self.release.clone(), &bytes);
        let args = self
            .install_args
            .as_ref()
            .map(|path| {
                store::read(
                    &config_path.with_file_name(path),
                    canic_management_protocol::application::MAX_ARGUMENT_BYTES,
                )
            })
            .transpose()?
            .unwrap_or_default();
        let approval = selected.config.shard_pool_approval(
            manifest,
            canic_management_protocol::application::Arguments::new(args)
                .map_err(crate::supplier)?,
        )?;
        let request = ic_wire::encode(
            &approval,
            ic_wire::Encoding::Frame {
                method: canic_management_protocol::stored::CONFIGURE_SHARDS,
                direction: ic_wire::Direction::Request,
            },
            canic_management_protocol::stored::LIMITS,
        )
        .map_err(supplier)?;
        Ok(FleetPlanning::ShardPool {
            request,
            deployment: DeploymentName {
                app: selected.app.clone(),
                fleet: selected.fleet.clone(),
                canister: self.component.clone(),
            },
            network: selected.config.network.clone(),
            controller: controller.into(),
            approval: Box::new(approval),
        })
    }
    fn deadline(&self) -> Result<u64> {
        self.expires_at_ns.ok_or_else(|| {
            Error::Invalid("shard growth approval requires an explicit deadline".into())
        })
    }
    fn hub<'a>(&self, slots: &'a BTreeMap<String, CanisterConfig>) -> Result<&'a str> {
        slots
            .get(&self.component)
            .and_then(|c| c.parent.as_deref())
            .ok_or_else(|| Error::Invalid("shard component must have a declared parent hub".into()))
    }
    pub(super) fn validate(&self, slots: &BTreeMap<String, CanisterConfig>) -> Result<()> {
        if !self.release.valid()
            || self
                .install_args
                .as_ref()
                .is_some_and(|p| p.as_os_str().is_empty())
        {
            return Err(Error::Invalid(
                "invalid shard release label or install argument path".into(),
            ));
        }
        let hub = self.hub(slots)?;
        let mut names = std::collections::BTreeSet::new();
        for slot in [self.component.as_str(), hub, &self.root, &self.store] {
            name(slot)?;
            if !slots.contains_key(slot) || !names.insert(slot) {
                return Err(Error::Invalid(
                    "shard, hub, Root and Store must be distinct declared components".into(),
                ));
            }
        }
        if !(1..=ChildId::ALL.len()).contains(&usize::from(self.max_shards))
            || !(1..=stored::MAX_KEYS_PER_SHARD).contains(&self.keys_per_shard)
            || self.expires_at_ns == Some(0)
        {
            return Err(Error::Invalid(
                "shard pool bounds or deadline outside the supported range".into(),
            ));
        }
        Ok(())
    }
}
impl FleetConfig {
    /// Prepare the exact controller approval from this fleet and its selected immutable Store release.
    /// Preparation is local. The operator submits the resulting grant to its bound Root.
    /// # Errors
    /// Refuses incomplete declarations, unbound principals or invalid release metadata.
    pub fn shard_pool_approval(
        &self,
        manifest: ic_wasm_store_protocol::Manifest,
        arguments: canic_management_protocol::application::Arguments,
    ) -> Result<stored::ShardPool> {
        let policy = self
            .shard_pool
            .as_ref()
            .ok_or_else(|| Error::Invalid("no shard pool configured".into()))?;
        policy.validate(&self.canisters)?;
        if !manifest.valid() || manifest.release != policy.release {
            return Err(Error::Invalid("invalid shard release manifest".into()));
        }
        let id = |slot: &str| {
            let value = self
                .canisters
                .get(slot)
                .and_then(|c| c.id.clone())
                .ok_or_else(|| Error::Invalid(format!("missing shard pool binding for {slot}")))?;
            let principal = crate::network::principal(&value)?;
            if principal.to_text() != value || principal.as_slice().last() != Some(&1) {
                return Err(Error::Invalid(format!(
                    "pool binding {slot} must identify a canonical canister"
                )));
            }
            Ok(value)
        };
        if self.canisters[&policy.component].id.is_some() {
            return Err(Error::Invalid(
                "a pooled shard component cannot bind a fixed instance".into(),
            ));
        }
        Ok(stored::ShardPool {
            arguments,
            pool: policy.component.clone(),
            root: id(&policy.root)?,
            hub: id(policy.hub(&self.canisters)?)?,
            max_shards: policy.max_shards,
            keys_per_shard: policy.keys_per_shard,
            creation_budget: policy.creation_budget,
            expires_at_ns: policy.deadline()?,
            reference: stored::Reference {
                store: id(&policy.store)?,
                manifest,
            },
        })
    }
}
