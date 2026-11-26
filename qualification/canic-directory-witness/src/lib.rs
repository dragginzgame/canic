//! Installed hub that captures its fleet Root and opens one service index.
use canic::topology::Install;
use canic::topology::directory::{
    Delivery, Directory, Hop, ReceiveDirectory, ServiceInstall,
    retained::{DIRECTORY, Index},
};
use ic_platform_types::Principal;
use ic_storage::DefaultMemoryImpl;

#[cfg(feature = "placement")]
mod evacuation;
#[cfg(feature = "funding")]
mod funding;
#[cfg(feature = "funding")]
#[path = "../../canic-shard-hub/src/paid_fixture.rs"]
mod paid_fixture;
mod peer;
#[cfg(feature = "role-probe")]
mod role_probe;

const KEY: &str = "hub.services.v1";

enum Installation {
    Service(ServiceInstall),
    Child(Install),
}

fn open(installation: Option<Installation>) {
    let runtime = canic::storage::bootstrap(
        DefaultMemoryImpl::default(),
        installation.is_some(),
        "hub",
        &[
            KEY,
            #[cfg(feature = "placement")]
            evacuation::KEYS[0],
            #[cfg(feature = "placement")]
            evacuation::KEYS[1],
            #[cfg(feature = "placement")]
            evacuation::KEYS[2],
            #[cfg(feature = "placement")]
            "hub.projects.v1",
            #[cfg(feature = "funding")]
            funding::KEY,
            #[cfg(feature = "role-probe")]
            role_probe::KEY,
        ],
        16,
    );
    #[cfg(feature = "placement")]
    evacuation::open(
        &runtime,
        installation.as_ref().map(|i| match i {
            Installation::Child(child) => (child.root, None),
            Installation::Service(service) => {
                (ic_platform_runtime::system::caller(), service.hub_operator)
            }
        }),
    );
    #[cfg(feature = "placement")]
    canic::sharding::registry::open(
        &runtime,
        "hub.projects.v1",
        canic::sharding::routes::root(),
        installation.is_some(),
    );
    #[cfg(feature = "funding")]
    {
        ic_timers::initialize().expect("timer runtime");
        canic_management::funding::open(
            &runtime,
            funding::KEY,
            installation.as_ref().map(|i| match i {
                Installation::Service(service) => service.funding_operator,
                Installation::Child(child) => ServiceInstall::decode(&child.application)
                    .ok()
                    .and_then(|s| s.funding_operator),
            }),
        );
    }
    #[cfg(feature = "role-probe")]
    let pin = installation.as_ref().map(|i| match i {
        Installation::Service(service) => (
            ic_platform_runtime::system::caller(),
            service.application.clone(),
        ),
        Installation::Child(child) => (child.root, vec![]),
    });
    DIRECTORY.set(Some(match installation {
        Some(Installation::Service(service)) => Index::install(&runtime, KEY, &service.role),
        Some(Installation::Child(child)) => Index::install_child(&runtime, KEY, &child),
        None => Index::open(&runtime, KEY),
    }));
    #[cfg(feature = "role-probe")]
    role_probe::open(&runtime, pin);
}

fn install_args(bytes: &[u8]) -> Result<Installation, &'static str> {
    // The inline fixture has one compiled role; Store-backed installs declare it.
    if bytes.is_empty() {
        return Ok(Installation::Service(ServiceInstall {
            funding_operator: None,
            hub_operator: None,
            name: "hub".into(),
            role: "hub".into(),
            application: vec![],
        }));
    }
    if let Ok(child) = Install::decode(bytes) {
        if !child.application.is_empty() {
            let binding = ServiceInstall::decode(&child.application)?;
            if binding.role != child.role || !binding.application.is_empty() {
                return Err("invalid witness child binding");
            }
        }
        return Ok(Installation::Child(child));
    }
    let named = ServiceInstall::decode(bytes)?;
    if !(named.application.is_empty()
        || (cfg!(feature = "role-probe") && named.application.len() == 32))
    {
        return Err("expected empty payload or issuer key");
    }
    Ok(Installation::Service(named))
}
fn install(value: Installation) {
    open(Some(value));
}
fn reopen() {
    open(None);
}
canic::mount_lifecycle!(
    input = Installation,
    max_bytes = 4096,
    decode = install_args,
    install = install,
    reopen = reopen
);

fn readiness() -> canic::readiness::State {
    if DIRECTORY.with_borrow(|index| index.is_some()) {
        canic::readiness::State::Ready
    } else {
        canic::readiness::State::Unavailable
    }
}
canic::mount_readiness_endpoint!(crate::readiness);

// Read the retained installer and last applied revision.
#[canic::query]
fn directory_status() -> (Principal, Option<u64>) {
    DIRECTORY.with_borrow(|index| {
        let index = index.as_ref().expect("open service index");
        (index.publisher(), index.current().map(|d| d.revision))
    })
}

// Read this canister's local service directory without contacting Root.
#[canic::query]
fn resolve_service(role: String) -> Option<(u64, Vec<Principal>)> {
    DIRECTORY.with_borrow(|index| {
        let directory = index.as_ref()?.current()?;
        Some((directory.revision, directory.by_role(&role).collect()))
    })
}

// Select one target only when the local revision identifies a unique service.
#[canic::query]
fn unique_service(role: String) -> Option<(u64, Result<Principal, String>)> {
    DIRECTORY.with_borrow(|index| {
        let directory = index.as_ref()?.current()?;
        Some((
            directory.revision,
            directory.unique_by_role(&role).map_err(str::to_owned),
        ))
    })
}

// pool_children
// List acknowledged direct shards for the named pool.
// guard  : public qualification observation
// limits : default Wire limits
#[canic::query]
fn pool_children(pool: String) -> Vec<Principal> {
    DIRECTORY.with_borrow(|index| {
        index
            .as_ref()
            .expect("open index")
            .acknowledged_shards(&pool)
            .collect()
    })
}

// Accept a directory update under the retained installer authority.
canic::mount_service_directory!();

// Inspect the role and direct-child acknowledgement held by this actor.
#[canic::query]
fn topology_status(child: Principal) -> (String, Option<Option<u64>>) {
    DIRECTORY.with_borrow(|index| {
        let index = index.as_ref().expect("open service index");
        (
            index.role().to_owned(),
            index.child_delivery(child).map(|(_, revision)| revision),
        )
    })
}

// Have a real installed sibling try to publish while impersonation is impossible.
#[canic::update]
async fn attempt_delivery() -> Principal {
    let target = DIRECTORY
        .with_borrow(|index| index.as_ref()?.current()?.by_name("accounts_hub"))
        .expect("accounts_hub must be published");
    let result = canic::method::call::<ReceiveDirectory>(
        target,
        5,
        &Delivery {
            backup_timing: None,
            directory: Directory {
                revision: 1,
                services: Vec::new(),
            },
            path: vec![Hop {
                shard_pool: None,
                canister: target,
                role: "hub".into(),
                generation: [0; 32],
            }],
        },
    )
    .await;
    match result {
        Ok(Err(canic::topology::directory::DirectoryError::Unauthorized)) => target,
        other => panic!("unexpected directory outcome: {other:?}"),
    }
}

// Use the installed hub's identity and retained Root for placement qualification.
#[cfg(feature = "placement")]
#[canic::update(reply_limits = canic::sharding::LIMITS)]
async fn place(request: (Option<String>, u8)) -> Result<canic::sharding::Placement, String> {
    let (pool, key) = request;
    match pool {
        Some(pool) => canic::sharding::routes::assign_shard(
            &pool,
            Principal::self_authenticating([key; 32]),
            || Ok::<_, std::convert::Infallible>(()),
        )
        .await
        .map_err(|err| match err {
            canic::sharding::routes::ReservationError::Placement(e) => e.to_owned(),
            canic::sharding::routes::ReservationError::Authorization(never) => match never {},
        }),
        None => canic::sharding::registry::assign_instance([key; 32])
            .await
            .map_err(|error| format!("{error:?}")),
    }
}

// release_route
// Release the fixture account from its exact draining allocation.
// guard  : actual hub controller
// limits : shard Wire frame
#[cfg(feature = "placement")]
#[canic::update(limits = canic::sharding::LIMITS, guard = canic::guard::require_controller)]
fn release_route(
    (key, target): (canic::sharding::Key, canic::sharding::ShardTarget),
) -> Result<(), String> {
    canic::sharding::routes::release("users", Principal::self_authenticating(key), &target)
        .map_err(str::to_owned)
}
