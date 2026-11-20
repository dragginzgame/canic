//! Installed hub that captures its fleet Root and opens one service index.
use canic::topology::Install;
use canic::topology::directory::{Directory, ReceiveDirectory, ServiceInstall, retained::Index};
use ic_platform_types::Principal;
use ic_storage::DefaultMemoryImpl;
use std::cell::RefCell;

const KEY: &str = "hub.services.v1";
thread_local! { static INDEX: RefCell<Option<Index<DefaultMemoryImpl>>> = const { RefCell::new(None) }; }

fn open(fresh: bool, child: Option<&Install>) {
    let runtime = canic::storage::bootstrap(DefaultMemoryImpl::default(), fresh, "hub", &[KEY], 16);
    INDEX.set(Some(if fresh {
        match child {
            Some(child) => Index::install_child(&runtime, KEY, child),
            None => Index::install(&runtime, KEY),
        }
    } else {
        Index::open(&runtime, KEY)
    }));
}

fn install_args(bytes: &[u8]) -> Result<Option<Install>, &'static str> {
    if bytes.is_empty() {
        return Ok(None);
    }
    if let Ok(child) = Install::decode(bytes) {
        return child
            .application
            .is_empty()
            .then_some(Some(child))
            .ok_or("expected empty witness payload");
    }
    let named = ServiceInstall::decode(bytes)?;
    named
        .application
        .is_empty()
        .then_some(None)
        .ok_or("expected empty witness payload")
}

#[allow(clippy::needless_pass_by_value)]
fn install(child: Option<Install>) {
    open(true, child.as_ref());
}
fn reopen() {
    open(false, None);
}
canic::mount_lifecycle!(
    input = Option<Install>,
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

// Accept a directory update under the retained installer authority.
#[canic::update(contract = ReceiveDirectory)]
fn receive_directory(directory: Directory) {
    INDEX.with_borrow_mut(|index| {
        index
            .as_mut()
            .expect("open service index")
            .receive(directory)
            .unwrap_or_else(|reason| ic_platform_runtime::system::trap(reason))
    });
}

// Have a real installed sibling try to publish while impersonation is impossible.
#[canic::update]
async fn attempt_delivery(target: Principal) -> bool {
    let result = canic::method::call::<ReceiveDirectory>(
        target,
        5,
        &Directory {
            revision: 1,
            services: Vec::new(),
        },
    )
    .await;
    match result {
        Ok(()) => true,
        Err(
            canic::method::CallError::Rejected(reason) | canic::method::CallError::Unknown(reason),
        ) if reason.message().contains("unexpected topology sender") => false,
        other => panic!("unexpected directory outcome: {other:?}"),
    }
}

// Exercise forwarding of the hub's actual committed directory in the installed fixture.
#[canic::update]
async fn forward_directory(child: Principal) {
    let directory = INDEX.with_borrow(|index| {
        index
            .as_ref()
            .and_then(Index::current)
            .expect("published directory")
            .clone()
    });
    canic::method::call::<ReceiveDirectory>(child, 5, &directory)
        .await
        .expect("child directory delivery");
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
