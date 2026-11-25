use crate::auth::install::{Install, Storage};
use crate::{readiness::State, storage, topology};
use ic_platform_runtime::system as api;
use ic_platform_types::Principal;
use ic_storage::{DefaultMemoryImpl, MemoryRuntime, StorageConfig};
use serde::{Deserialize, Serialize};
use std::cell::Cell;

// Field order is the existing hub configuration payload.
#[doc(hidden)]
#[derive(Serialize, Deserialize)]
pub struct HubConfig {
    operator: Option<Principal>,
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

fn bootstrap(
    mode: Option<Storage>,
    authority: &str,
    prefix: &str,
    keys: &[&str],
    config: StorageConfig,
) -> MemoryRuntime<DefaultMemoryImpl> {
    let application = (authority, prefix, keys);
    use crate::auth::session::stable;
    let auth_keys = [stable::ENGINE_KEY, stable::HOST_KEY];
    let config = StorageConfig {
        allocations: config.allocations.max(keys.len() + auth_keys.len()),
        ..config
    };
    let fresh = mode.is_some();
    let runtime = storage::open_directory(DefaultMemoryImpl::default(), fresh, config);
    match mode.unwrap_or_else(|| stable::layout(&runtime)) {
        Storage::Stable => storage::admit_groups(
            runtime,
            fresh,
            &[application, ("auth", "auth.receiver.", &auth_keys)],
        ),
        Storage::Volatile => storage::admit_groups(runtime, fresh, &[application]),
    }
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
        crate::guard::require_sender(input.root)?;
        let service = topology::directory::ServiceInstall::decode(&input.application)?;
        let bindings = topology::directory::DbService::decode(&service.application)?;
        (service, bindings)
    };
    #[cfg(not(feature = "role-attestation"))]
    let (operator, nonce) = if input.application.len() == 16 {
        (None, nonce(&input.application)?)
    } else {
        crate::guard::require_sender(input.root)?;
        let service = topology::directory::ServiceInstall::decode(&input.application)?;
        (service.hub_operator, nonce(&service.application)?)
    };
    let config = HubConfig {
        #[cfg(feature = "role-attestation")]
        operator: service.hub_operator,
        #[cfg(not(feature = "role-attestation"))]
        operator,
        #[cfg(not(feature = "role-attestation"))]
        nonce,
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

#[doc(hidden)]
/// Reopen or initialize the hub database and its selected service bindings.
/// # Panics
/// Traps on invalid backing, mismatched membership or failed store admission.
#[must_use]
pub fn open_hub(
    install: Option<&Install>,
    authority: &str,
    prefix: &str,
    keys: &[&str],
    config_key: &str,
    pages: u64,
) -> (MemoryRuntime<DefaultMemoryImpl>, Option<[u8; 16]>) {
    let initial = install
        .map(decode_hub)
        .transpose()
        .expect("approved role installation");
    #[cfg(feature = "role-attestation")]
    let directory = format!("{prefix}services.v1");
    let mut keys = keys.to_vec();
    let roster = format!("{prefix}shard_roster.v1");
    let owners = format!("{prefix}shard_owners.v1");
    keys.extend([config_key, &roster, &owners]);
    #[cfg(feature = "role-attestation")]
    keys.push(&directory);
    let host = bootstrap(
        install.map(|input| input.storage),
        authority,
        prefix,
        &keys,
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
        topology::attach_role_calls(config.root);
    }
    crate::sharding::routes::open(
        &host,
        &roster,
        &owners,
        initial.as_ref().map(|c| (c.root, c.operator)),
    );
    ROOT.set(Some(config.root));
    (host, initial.map(|c| c.nonce))
}

#[doc(hidden)]
/// Reopen or initialize the shard database and its selected service bindings. Parent admission checks the real installer.
/// # Panics
/// Traps on invalid storage, parent or database. Never creates on failed reopen.
pub fn open_shard(
    install: Option<&Install>,
    authority: &str,
    prefix: &str,
    keys: &[&str],
    parent_key: &str,
    pages: Option<u64>,
) -> (MemoryRuntime<DefaultMemoryImpl>, Option<[u8; 16]>) {
    let initial = install
        .map(decode_shard)
        .transpose()
        .expect("approved role installation");
    let mut config = StorageConfig::default();
    if let Some(pages) = pages {
        config.pages = pages;
    }
    #[cfg(feature = "role-attestation")]
    let directory = format!("{prefix}services.v1");
    #[cfg(feature = "role-attestation")]
    let root_key = format!("{prefix}routing_root.v1");
    let mut keys = keys.to_vec();
    keys.push(parent_key);
    #[cfg(feature = "role-attestation")]
    keys.extend([directory.as_str(), root_key.as_str()]);
    let host = bootstrap(
        install.map(|input| input.storage),
        authority,
        prefix,
        &keys,
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
    (host, parts.map(|(_, nonce)| nonce))
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
