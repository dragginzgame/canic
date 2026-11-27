use super::RoleStorage;
use crate::{
    auth::install::Install,
    readiness::State,
    storage::{self, StorageGroup},
    topology,
};
use ic_platform_runtime::system as api;
use ic_platform_types::Principal;
use ic_storage::{DefaultMemoryImpl, StorageConfig};
use serde::{Deserialize, Serialize};
use std::cell::Cell;

// Field order is the existing hub configuration payload.
#[doc(hidden)]
#[derive(Serialize, Deserialize)]
struct HubConfig {
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
    fresh: bool,
    application: StorageGroup<'_>,
    suppliers: &[StorageGroup<'_>],
    mut config: StorageConfig,
) -> RoleStorage {
    use crate::auth::session::stable;
    let auth = StorageGroup {
        authority: "auth",
        prefix: "auth.receiver.",
        keys: &[stable::ENGINE_KEY, stable::HOST_KEY],
    };
    let mut groups = vec![application, auth];
    groups.extend_from_slice(suppliers);
    let required = groups.iter().map(|group| group.keys.len()).sum();
    config.allocations = config.allocations.max(required);
    storage::bootstrap_groups(DefaultMemoryImpl::default(), fresh, &groups, config)
}

/// Routing Root recovered from this hub’s configuration or this attested shard’s retained binding.
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
fn decode_hub(input: &Install) -> Result<HubConfig, &'static str> {
    #[cfg(feature = "role-attestation")]
    let (operator, nonce, service_role, issuer_key) = {
        crate::guard::require_sender(input.root)?;
        let service = topology::directory::ServiceInstall::decode(&input.application)?;
        let bindings = topology::directory::DbService::decode(&service.application)?;
        (
            service.hub_operator,
            bindings.nonce,
            service.role,
            bindings.issuer_key,
        )
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
        operator,
        nonce,
        #[cfg(feature = "role-attestation")]
        service_role,
        #[cfg(feature = "role-attestation")]
        issuer_key,
        root: input.root,
        issuer: input.issuer,
        network: input.network,
        fleet: input.audience,
    };
    config.validate()?;
    Ok(config)
}

/// Open the hub's application, Auth and supplier allocations in one directory.
/// Pass the same declarations on reopen. Use the returned runtime to attach
/// each supplier, selecting create only on fresh installation.
/// # Panics
/// Traps on invalid role bindings, grants or a different retained key set.
#[must_use]
pub fn open_hub(
    install: Option<&Install>,
    application: StorageGroup<'_>,
    suppliers: &[StorageGroup<'_>],
    config_key: &str,
    pages: u64,
) -> (RoleStorage, Option<[u8; 16]>) {
    let prefix = application.prefix;
    let initial = install
        .map(decode_hub)
        .transpose()
        .expect("approved role installation");
    #[cfg(feature = "role-attestation")]
    let directory = format!("{prefix}services.v1");
    let mut keys = application.keys.to_vec();
    let roster = format!("{prefix}shard_roster.v1");
    let owners = format!("{prefix}shard_owners.v1");
    keys.extend([config_key, &roster, &owners]);
    #[cfg(feature = "role-attestation")]
    keys.push(&directory);
    let host = bootstrap(
        install.is_some(),
        StorageGroup {
            keys: &keys,
            ..application
        },
        suppliers,
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

/// Open the shard's application, Auth and supplier allocations in one directory.
/// Pass the same declarations on reopen. Use the returned runtime to attach
/// each supplier, selecting create only on fresh installation.
/// # Panics
/// Traps on invalid role bindings, grants or a different retained key set.
#[must_use]
pub fn open_shard(
    install: Option<&Install>,
    application: StorageGroup<'_>,
    suppliers: &[StorageGroup<'_>],
    parent_key: &str,
    pages: Option<u64>,
) -> (RoleStorage, Option<[u8; 16]>) {
    #[cfg(feature = "role-attestation")]
    let prefix = application.prefix;
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
    let mut keys = application.keys.to_vec();
    keys.push(parent_key);
    #[cfg(feature = "role-attestation")]
    keys.extend([directory.as_str(), root_key.as_str()]);
    let host = bootstrap(
        install.is_some(),
        StorageGroup {
            keys: &keys,
            ..application
        },
        suppliers,
        config,
    );
    let binding = initial.as_ref().map(|config| &config.binding);
    topology::attach_parent(&host, parent_key, binding);
    #[cfg(feature = "role-attestation")]
    {
        use topology::directory::retained::{DIRECTORY, Index};
        let root = storage::cell(
            &host,
            &root_key,
            29,
            binding.map(|binding| binding.root.as_slice().to_vec()),
        );
        let root = Principal::try_from_slice(root.get()).expect("retained routing Root");
        topology::attach_role_calls(root);
        ROOT.set(Some(root));
        DIRECTORY.set(Some(match binding {
            Some(binding) => Index::install_child(&host, &directory, binding),
            None => Index::open(&host, &directory),
        }));
    }
    (host, initial.map(|config| config.nonce))
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
struct ShardConfig {
    binding: topology::Install,
    nonce: [u8; 16],
}

#[doc(hidden)]
/// Decode the shard's nested topology and check the outer Auth Root.
/// # Errors
/// Rejects a missing frame, conflicting Root or invalid nonce.
fn decode_shard(input: &Install) -> Result<ShardConfig, &'static str> {
    let binding = topology::Install::decode(&input.application)?;
    binding.check_root(input.root)?;
    Ok(ShardConfig {
        nonce: nonce(&binding.application)?,
        binding,
    })
}
