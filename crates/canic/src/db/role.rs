//! Installed hub and child layouts. Readiness observes successful local attachment.
use crate::{auth::install::Install, readiness::State, storage, topology};
use ic_platform_runtime::system as api;
use ic_platform_types::Principal;
use ic_storage::{DefaultMemoryImpl, MemoryRuntime, StorageConfig};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cell::Cell;

type Attach = fn(&MemoryRuntime<DefaultMemoryImpl>, Option<[u8; 16]>);

// Field order is the existing hub configuration payload.
#[doc(hidden)]
#[derive(Serialize, Deserialize)]
pub struct HubConfig {
    nonce: [u8; 16],
    root: Principal,
    issuer: Principal,
    network: [u8; 32],
    fleet: [u8; 32],
}

const CONFIG_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 512,
    allocation: 16384,
    nodes: 512,
    depth: 8,
};

thread_local! {
    static ROOT: Cell<Option<Principal>> = const { Cell::new(None) };
}

/// Routing Root recovered from this hub's own configuration.
#[must_use]
pub fn root() -> Option<Principal> {
    ROOT.get()
}

fn nonce(bytes: &[u8]) -> Result<[u8; 16], &'static str> {
    let nonce = bytes
        .try_into()
        .map_err(|_| "expected a 16-byte install nonce")?;
    if nonce == [0; 16] {
        return Err("zero installation nonce");
    }
    Ok(nonce)
}

impl HubConfig {
    fn validate(&self) -> Result<(), &'static str> {
        if self.nonce == [0; 16]
            || self.root == api::canister_self()
            || self.root.as_slice().last() != Some(&1)
            || self.issuer.as_slice().last() != Some(&1)
        {
            return Err("invalid hub configuration");
        }
        Ok(())
    }
}

/// Decode the approved hub context before initializing the fresh receiver.
/// # Errors
/// Rejects an invalid nonce or invalid routing and issuer principals.
#[doc(hidden)]
pub fn decode_hub(input: &Install) -> Result<HubConfig, &'static str> {
    let config = HubConfig {
        nonce: nonce(&input.application)?,
        root: input.root,
        issuer: input.issuer,
        network: input.network,
        fleet: input.audience,
    };
    config.validate()?;
    Ok(config)
}

fn incarnation(key: &str, nonce: [u8; 16]) -> [u8; 16] {
    let digest = Sha256::new()
        .chain_update(key)
        .chain_update(nonce)
        .chain_update(api::canister_self().as_slice())
        .finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&digest[..16]);
    id
}

#[doc(hidden)]
/// Attach the two-key hub directory.
/// # Panics
/// Traps on invalid backing, mismatched membership or failed store admission.
pub fn open_hub(
    initial: Option<HubConfig>,
    authority: &str,
    key: &str,
    config_key: &str,
    pages: u64,
    attach: Attach,
) {
    let host = storage::bootstrap(
        DefaultMemoryImpl::default(),
        initial.is_some(),
        authority,
        &[config_key, key],
        pages,
    );
    let bytes = initial.as_ref().map(|config| {
        ic_wire::encode(config, ic_wire::Encoding::Payload, CONFIG_LIMITS)
            .expect("encode hub config")
    });
    let cell = storage::cell(&host, config_key, CONFIG_LIMITS.bytes, bytes);
    let config: HubConfig = ic_wire::decode(cell.get(), ic_wire::Encoding::Payload, CONFIG_LIMITS)
        .expect("retained hub configuration");
    config.validate().expect("validate retained hub");
    attach(&host, initial.map(|c| incarnation(key, c.nonce)));
    ROOT.set(Some(config.root));
}

#[doc(hidden)]
pub struct ShardConfig {
    binding: topology::Install,
    nonce: [u8; 16],
}

#[doc(hidden)]
/// Decode the shard's nested topology and check the outer Auth Root.
/// # Errors
/// Rejects a missing frame, conflicting Root or invalid nonce.
pub fn decode_shard(input: &Install) -> Result<ShardConfig, &'static str> {
    let binding = topology::Install::decode(&input.application)?;
    binding.check_root(input.root)?;
    Ok(ShardConfig {
        nonce: nonce(&binding.application)?,
        binding,
    })
}

#[doc(hidden)]
/// Attach the two-key shard directory. Parent admission checks the real installer.
/// # Panics
/// Traps on invalid storage, parent or database. Never creates on failed reopen.
pub fn open_shard(
    initial: Option<ShardConfig>,
    authority: &str,
    prefix: &str,
    key: &str,
    parent_key: &str,
    attach: Attach,
) {
    let host = storage::bootstrap_with_prefix(
        DefaultMemoryImpl::default(),
        initial.is_some(),
        authority,
        prefix,
        &[key, parent_key],
        StorageConfig::default(),
    );
    topology::attach_parent(&host, parent_key, initial.as_ref().map(|i| &i.binding));
    attach(&host, initial.map(|i| incarnation(key, i.nonce)));
}

#[doc(hidden)]
#[must_use]
pub fn hub_readiness() -> State {
    if root().is_some() {
        State::Ready
    } else {
        State::Unavailable
    }
}
#[doc(hidden)]
#[must_use]
pub fn shard_readiness() -> State {
    if topology::parent().is_some() {
        State::Ready
    } else {
        State::Unavailable
    }
}
