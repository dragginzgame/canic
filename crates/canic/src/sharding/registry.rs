//! Registry-local bindings from application project keys to installed children.
use super::{Allocation, Key, Placement, client, payment::AcquireError};
use crate::method::CallError;
use ic_platform_types::Principal;
use ic_storage::{DefaultMemoryImpl, MemoryRuntime, StableKey, adapter::StableBTreeMap};
use sha2::{Digest, Sha256};
use std::cell::RefCell;

type Projects = StableBTreeMap<Key, (u32, [u8; 32], [u8; 30])>;
thread_local! {
    static PROJECTS: RefCell<Option<(Principal, Projects)>> = const { RefCell::new(None) };
}

/// Open the registry's own lease on install or upgrade.
/// # Panics
/// Requires an admitted lease and the matching Storage lifecycle.
pub fn open(runtime: &MemoryRuntime<DefaultMemoryImpl>, key: &str, root: Principal, fresh: bool) {
    let lease = runtime
        .open_memory(&StableKey::parse(key).expect("registry key"))
        .expect("registry lease");
    PROJECTS.set(Some((
        root,
        if fresh {
            Projects::create(lease)
        } else {
            Projects::load(lease)
        },
    )));
}

fn request(key: &Key) -> [u8; 32] {
    // Root authenticates the paying registry separately. This domain is part of recovery.
    Sha256::new()
        .chain_update(b"canic.registry.creation_request.v1")
        .chain_update(key)
        .finalize()
        .into()
}

fn binding(key: &Key) -> (Principal, Option<Placement>) {
    PROJECTS.with_borrow(|rows| {
        let (root, rows) = rows.as_ref().expect("open registry");
        (
            *root,
            rows.get(key).map(|(slot, generation, principal)| {
                Placement::Assigned(
                    Allocation { slot, generation },
                    Principal::from_slice(&principal[1..=usize::from(principal[0])]).to_text(),
                )
            }),
        )
    })
}

fn retain(key: Key, result: Placement) -> Placement {
    if let Some((allocation, principal)) = result
        .assigned_target()
        .expect("invalid Root child principal")
    {
        let bytes = principal.as_slice();
        let mut encoded = [0; 30];
        encoded[0] = u8::try_from(bytes.len()).expect("principal length");
        encoded[1..=bytes.len()].copy_from_slice(bytes);
        let value = (allocation.slot, allocation.generation, encoded);
        PROJECTS.with_borrow_mut(|rows| {
            let (_, rows) = rows.as_mut().expect("open registry");
            if let Some(old) = rows.get(&key) {
                assert_eq!(old, value, "registry child changed");
            } else {
                rows.insert(key, value);
            }
        });
    }
    binding(&key).1.unwrap_or(result)
}

/// Return the registry’s permanent binding or request its original creation from Root.
/// # Errors
/// Preserves the original acquisition failure. Retrying the same key uses the same request identity.
/// # Panics
/// Requires an opened registry. Conflicting installation evidence traps before replacement.
pub async fn assign_instance(key: Key) -> Result<Placement, AcquireError> {
    let (root, completed) = binding(&key);
    if let Some(child) = completed {
        return Ok(child);
    }
    client::acquire_instance(root, request(&key))
        .await
        .map(|result| retain(key, result))
}

/// Resolve a completed local claim, or observe its original creation without charging cycles.
/// # Errors
/// Preserves a failed observation of a still unresolved claim.
/// # Panics
/// Requires an opened registry and a valid immutable child binding.
pub async fn lookup_instance(key: Key) -> Result<Placement, CallError> {
    let (root, completed) = binding(&key);
    if let Some(child) = completed {
        return Ok(child);
    }
    client::observe_instance(root, request(&key))
        .await
        .map(|result| retain(key, result))
}
