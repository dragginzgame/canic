use crate::auth::install::Install;
use crate::{readiness::State, storage, topology};
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
    #[cfg(feature = "role-attestation")]
    service_role: String,
    #[cfg(feature = "role-attestation")]
    issuer_key: [u8; 32],
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
    #[cfg(feature = "role-attestation")]
    let (service, bindings) = {
        topology::require_sender(input.root)?;
        let service = topology::directory::ServiceInstall::decode(&input.application)?;
        let bindings = topology::directory::DbService::decode(&service.application)?;
        (service, bindings)
    };
    let config = HubConfig {
        #[cfg(not(feature = "role-attestation"))]
        nonce: nonce(&input.application)?,
        #[cfg(feature = "role-attestation")]
        nonce: bindings.nonce,
        #[cfg(feature = "role-attestation")]
        service_role: service.role,
        #[cfg(feature = "role-attestation")]
        issuer_key: bindings.issuer_key,
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
/// Reopen or initialize the hub database and its selected service bindings.
/// # Panics
/// Traps on invalid backing, mismatched membership or failed store admission.
pub fn open_hub(
    initial: Option<HubConfig>,
    authority: &str,
    prefix: &str,
    key: &str,
    config_key: &str,
    pages: u64,
    attach: Attach,
) {
    #[cfg(feature = "role-attestation")]
    let directory = format!("{prefix}services.v1");
    let keys = &[
        config_key,
        key,
        #[cfg(feature = "role-attestation")]
        &directory,
    ];
    let host = storage::bootstrap_with_prefix(
        DefaultMemoryImpl::default(),
        initial.is_some(),
        authority,
        prefix,
        keys,
        StorageConfig {
            allocations: keys.len(),
            pages,
            ..Default::default()
        },
    );
    let bytes = initial.as_ref().map(|config| {
        ic_wire::encode(config, ic_wire::Encoding::Payload, CONFIG_LIMITS)
            .expect("encode hub config")
    });
    let cell = storage::cell(&host, config_key, CONFIG_LIMITS.bytes, bytes);
    let config: HubConfig = ic_wire::decode(cell.get(), ic_wire::Encoding::Payload, CONFIG_LIMITS)
        .expect("retained hub configuration");
    config.validate().expect("validate retained hub");
    #[cfg(feature = "role-attestation")]
    {
        use topology::directory::retained::{DIRECTORY, Index};
        DIRECTORY.set(Some(if initial.is_some() {
            Index::install(&host, &directory, &config.service_role)
        } else {
            Index::open(&host, &directory)
        }));
        topology::attestation::RoleVerifier::new(config.root, &config.issuer_key)
            .and_then(topology::attestation::RoleVerifier::attach)
            .expect("retained service trust");
    }
    attach(&host, initial.map(|c| incarnation(key, c.nonce)));
    ROOT.set(Some(config.root));
}

#[doc(hidden)]
/// Reopen or initialize the shard database and its selected service bindings. Parent admission checks the real installer.
/// # Panics
/// Traps on invalid storage, parent or database. Never creates on failed reopen.
pub fn open_shard(
    initial: Option<ShardConfig>,
    authority: &str,
    prefix: &str,
    key: &str,
    parent_key: &str,
    pages: Option<u64>,
    attach: Attach,
) {
    let mut config = StorageConfig::default();
    if let Some(pages) = pages {
        config.pages = pages;
    }
    #[cfg(feature = "role-attestation")]
    let directory = format!("{prefix}services.v1");
    #[cfg(feature = "role-attestation")]
    let root_key = format!("{prefix}routing_root.v1");
    let host = storage::bootstrap_with_prefix(
        DefaultMemoryImpl::default(),
        initial.is_some(),
        authority,
        prefix,
        &[
            key,
            parent_key,
            #[cfg(feature = "role-attestation")]
            &directory,
            #[cfg(feature = "role-attestation")]
            &root_key,
        ],
        config,
    );
    let parts = initial.map(ShardConfig::into_parts);
    topology::attach_parent(
        &host,
        parent_key,
        parts.as_ref().map(|(binding, _)| binding),
    );
    #[cfg(feature = "role-attestation")]
    {
        use topology::directory::retained::{DIRECTORY, Index};
        let root = storage::cell(
            &host,
            &root_key,
            29,
            parts
                .as_ref()
                .map(|(binding, _)| binding.root.as_slice().to_vec()),
        );
        let root = Principal::try_from_slice(root.get()).expect("retained routing Root");
        topology::attach_role_calls(root);
        DIRECTORY.set(Some(match &parts {
            Some((binding, _)) => Index::install_child(&host, &directory, binding),
            None => Index::open(&host, &directory),
        }));
    }
    attach(&host, parts.map(|(_, nonce)| incarnation(key, nonce)));
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

fn nonce(bytes: &[u8]) -> Result<[u8; 16], &'static str> {
    let nonce = bytes
        .try_into()
        .map_err(|_| "expected a 16-byte install nonce")?;
    if nonce == [0; 16] {
        return Err("zero installation nonce");
    }
    Ok(nonce)
}

#[doc(hidden)]
pub struct ShardConfig {
    binding: topology::Install,
    nonce: [u8; 16],
}

impl ShardConfig {
    /// Consume the validated installation context and its application nonce.
    #[must_use]
    pub fn into_parts(self) -> (topology::Install, [u8; 16]) {
        (self.binding, self.nonce)
    }
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
