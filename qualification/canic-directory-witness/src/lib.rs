//! Installed hub that captures its fleet Root and opens one service index.
use canic::topology::Install;
use canic::topology::directory::{
    Delivery, Directory, Hop, ReceiveDirectory, ServiceInstall,
    retained::{self, Index},
};
use ic_platform_types::Principal;
use ic_storage::DefaultMemoryImpl;
use std::cell::RefCell;

mod peer;

const KEY: &str = "hub.services.v1";
thread_local! { static INDEX: RefCell<Option<Index<DefaultMemoryImpl>>> = const { RefCell::new(None) }; }

enum Installation {
    Service(String),
    Child(Install),
}

fn open(installation: Option<Installation>) {
    let runtime = canic::storage::bootstrap(
        DefaultMemoryImpl::default(),
        installation.is_some(),
        "hub",
        &[KEY, peer::KEY],
        16,
    );
    peer::open(&runtime, installation.is_some());
    INDEX.set(Some(match installation {
        Some(Installation::Service(role)) => Index::install(&runtime, KEY, &role),
        Some(Installation::Child(child)) => Index::install_child(&runtime, KEY, &child),
        None => Index::open(&runtime, KEY),
    }));
}

fn install_args(bytes: &[u8]) -> Result<Installation, &'static str> {
    // The inline fixture has one compiled role; Store-backed installs declare it.
    if bytes.is_empty() {
        return Ok(Installation::Service("hub".into()));
    }
    if let Ok(child) = Install::decode(bytes) {
        return child
            .application
            .is_empty()
            .then_some(Installation::Child(child))
            .ok_or("expected empty witness payload");
    }
    let named = ServiceInstall::decode(bytes)?;
    named
        .application
        .is_empty()
        .then_some(Installation::Service(named.role))
        .ok_or("expected empty witness payload")
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
    if INDEX.with_borrow(|index| index.is_some()) {
        canic::readiness::State::Ready
    } else {
        canic::readiness::State::Unavailable
    }
}
canic::mount_readiness_endpoint!(crate::readiness);

// Read the retained installer and last applied revision.
#[canic::query]
fn directory_status() -> (Principal, Option<u64>) {
    INDEX.with_borrow(|index| {
        let index = index.as_ref().expect("open service index");
        (index.publisher(), index.current().map(|d| d.revision))
    })
}

// Read this canister's local service directory without contacting Root.
#[canic::query]
fn resolve_service(role: String) -> Option<(u64, Vec<Principal>)> {
    INDEX.with_borrow(|index| {
        let directory = index.as_ref()?.current()?;
        Some((directory.revision, directory.by_role(&role).collect()))
    })
}

// Select one target only when the local revision identifies a unique service.
#[canic::query]
fn unique_service(role: String) -> Option<(u64, Result<Principal, String>)> {
    INDEX.with_borrow(|index| {
        let directory = index.as_ref()?.current()?;
        Some((
            directory.revision,
            directory.unique_by_role(&role).map_err(str::to_owned),
        ))
    })
}

// Accept a directory update under the retained installer authority.
#[canic::update(contract = ReceiveDirectory)]
async fn receive_directory(update: Delivery) -> Result<(), String> {
    retained::receive(&INDEX, update).await
}

// Inspect the role and direct-child acknowledgement held by this actor.
#[canic::query]
fn topology_status(child: Principal) -> (String, Option<Option<u64>>) {
    INDEX.with_borrow(|index| {
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
    let target = INDEX
        .with_borrow(|index| index.as_ref()?.current()?.by_name("accounts_hub"))
        .expect("accounts_hub must be published");
    let result = canic::method::call::<ReceiveDirectory>(
        target,
        5,
        &Delivery {
            directory: Directory {
                revision: 1,
                services: Vec::new(),
            },
            path: vec![Hop {
                canister: target,
                role: "hub".into(),
                generation: [0; 32],
            }],
        },
    )
    .await;
    match result {
        Ok(Err(reason)) if reason == "unexpected topology sender" => target,
        other => panic!("unexpected directory outcome: {other:?}"),
    }
}

// Use the installed hub's identity and retained Root for placement qualification.
#[cfg(feature = "placement")]
#[canic::update(reply_limits = canic::sharding::LIMITS)]
async fn place(request: (bool, u8)) -> canic::sharding::Placement {
    let root = INDEX.with_borrow(|index| index.as_ref().expect("open hub").publisher());
    let (project, key) = request;
    if project {
        canic::sharding::assign_instance(root, [key; 32]).await
    } else {
        canic::sharding::assign(root, [key; 32]).await
    }
    .expect("placement call")
}
