//! Module: memory::pool
//!
//! Responsibility: build host grants and retain a read-only projection after successful bootstrap.
//! Does not own: allocation placement, storage, admission callbacks or lifecycle.
//! Boundary: the artifact supplies explicit component grants at bootstrap.

use crate::memory::{CANIC_CONTROL_PLANE_MEMORY_AUTHORITY, CANIC_CORE_MEMORY_AUTHORITY};
use ic_memory::{MemoryAllocationPool, MemoryAllocationPoolError, MemoryAuthority};

/// Add mandatory framework grants to the artifact's explicit component grants.
/// Exclusions are physical unmanaged custody, never per-component partitions.
pub fn framework_pool(
    mut authorities: Vec<MemoryAuthority>,
    exclusions: Vec<u8>,
) -> Result<MemoryAllocationPool, MemoryAllocationPoolError> {
    authorities.push(MemoryAuthority::new(
        CANIC_CORE_MEMORY_AUTHORITY,
        "canic.core.",
    )?);
    authorities.push(MemoryAuthority::new(
        CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
        "canic.control_plane.",
    )?);
    MemoryAllocationPool::new(authorities, exclusions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_memory::{
        Memory, MemoryRequest, MemoryRuntime, SealedDeclarationSnapshot, VectorMemory,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    static ADMISSIONS: AtomicUsize = AtomicUsize::new(0);

    fn requests(entries: &[(&str, &str)]) -> SealedDeclarationSnapshot {
        SealedDeclarationSnapshot::new(
            entries
                .iter()
                .map(|(owner, key)| MemoryRequest::new(*owner, key).unwrap())
                .collect::<Vec<_>>(),
        )
        .unwrap()
    }

    #[test]
    fn composed_host_retains_keys_and_payload_across_two_cold_reopens() {
        let pool = framework_pool(
            vec![MemoryAuthority::new("app", "app.").unwrap()],
            Vec::new(),
        )
        .unwrap();
        let policy = crate::memory::CanicMemoryManagerPolicy::new().with_admission(
            crate::memory::admission::MemoryBootstrapAdmission::new(
                ic_memory::PolicyIdentity::new([1; 32]),
                |_| {
                    ADMISSIONS.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                },
            ),
        );
        let backing = VectorMemory::default();
        let initial = requests(&[
            (CANIC_CORE_MEMORY_AUTHORITY, "canic.core.rows.v1"),
            ("app", "app.rows.v1"),
        ]);
        let config = ic_memory::MemoryManagerConfig::new(1).unwrap();
        let mut runtime = MemoryRuntime::new_with_config(backing.clone(), config).unwrap();
        runtime.bootstrap(&initial, &pool, &policy).unwrap();
        let core_id = runtime.memory_id("canic.core.rows.v1").unwrap();
        let app_id = runtime.memory_id("app.rows.v1").unwrap();
        assert_ne!(core_id, app_id);
        let rows = runtime.open_memory("canic.core.rows.v1").unwrap();
        rows.grow(1).unwrap();
        rows.write(0, b"retained");
        drop(rows);
        drop(runtime);

        // Omit Core and add a control-plane component without recycling Core's ID.
        let changed = requests(&[
            ("app", "app.rows.v1"),
            (
                CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
                "canic.control_plane.rows.v1",
            ),
        ]);
        let mut runtime = MemoryRuntime::new_with_config(backing.clone(), config).unwrap();
        runtime.bootstrap(&changed, &pool, &policy).unwrap();
        assert_eq!(runtime.memory_id("app.rows.v1").unwrap(), app_id);
        let control_id = runtime.memory_id("canic.control_plane.rows.v1").unwrap();
        assert_ne!(control_id, core_id);
        let before = backing.borrow().clone();
        let admissions = ADMISSIONS.load(Ordering::SeqCst);
        runtime
            .verify_authority(&requests(&[("app", "app.rows.v1")]), "app")
            .unwrap();
        runtime.bootstrap(&changed, &pool, &policy).unwrap();
        assert_eq!(ADMISSIONS.load(Ordering::SeqCst), admissions);
        assert_eq!(*backing.borrow(), before);
        assert_eq!(
            runtime
                .memory_allocation_summary()
                .unwrap()
                .bucket_size_pages,
            1
        );
        drop(runtime);

        // Reordered linked components restore every retained binding and payload.
        let restored = requests(&[
            (
                CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
                "canic.control_plane.rows.v1",
            ),
            ("app", "app.rows.v1"),
            (CANIC_CORE_MEMORY_AUTHORITY, "canic.core.rows.v1"),
        ]);
        let mut runtime = MemoryRuntime::new_with_config(backing, config).unwrap();
        runtime.bootstrap(&restored, &pool, &policy).unwrap();
        assert_eq!(runtime.memory_id("canic.core.rows.v1").unwrap(), core_id);
        assert_eq!(runtime.memory_id("app.rows.v1").unwrap(), app_id);
        assert_eq!(
            runtime.memory_id("canic.control_plane.rows.v1").unwrap(),
            control_id
        );
        let mut payload = [0; 8];
        runtime
            .open_memory("canic.core.rows.v1")
            .unwrap()
            .read(0, &mut payload);
        assert_eq!(&payload, b"retained");
        assert_eq!(ADMISSIONS.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn populated_unmanaged_slot_requires_explicit_physical_exclusion() {
        let backing = crate::memory::fixtures::unmanaged(1);
        let config = ic_memory::MemoryManagerConfig::new(1).unwrap();
        let custody_before = backing.borrow()[65_536..].to_vec();
        let declarations = requests(&[(CANIC_CORE_MEMORY_AUTHORITY, "canic.core.rows.v1")]);
        let policy = crate::memory::CanicMemoryManagerPolicy::new();
        let mut runtime = MemoryRuntime::new_with_config(backing.clone(), config).unwrap();
        let before = backing.borrow().clone();
        assert!(matches!(
            runtime.bootstrap(
                &declarations,
                &framework_pool(Vec::new(), Vec::new()).unwrap(),
                &policy
            ),
            Err(ic_memory::RuntimeBootstrapError::Resolution(
                ic_memory::MemoryResolutionError::UnmanagedAllocation { id: 200 }
            ))
        ));
        assert_eq!(*backing.borrow(), before);
        assert!(!runtime.is_bootstrapped());
        let pool = framework_pool(Vec::new(), vec![200]).unwrap();
        runtime.bootstrap(&declarations, &pool, &policy).unwrap();
        let allocations = runtime.memory_allocations().unwrap();
        assert!(!runtime.allocation_pool().unwrap().contains(200));
        assert_eq!(allocations.memories[200].virtual_extent.wasm_pages, 1);
        assert_eq!(
            &backing.borrow()[65_536..65_536 + custody_before.len()],
            custody_before
        );
    }

    #[test]
    fn explicit_owner_grants_share_physical_eligibility() {
        let pool = framework_pool(
            vec![MemoryAuthority::new("app", "app.").unwrap()],
            vec![200, 201],
        )
        .unwrap();
        for (owner, key) in [
            (CANIC_CORE_MEMORY_AUTHORITY, "canic.core.rows.v1"),
            (
                CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
                "canic.control_plane.rows.v1",
            ),
            ("app", "app.rows.v1"),
        ] {
            pool.validate_authority(&ic_memory::StableKey::parse(key).unwrap(), owner)
                .unwrap();
        }
        for id in [10, 30, 100, 199, 202, 254] {
            assert!(pool.contains(id));
        }
        for id in [0, 9, 200, 201, 255] {
            assert!(!pool.contains(id));
        }
        assert!(matches!(
            pool.validate_authority(
                &ic_memory::StableKey::parse("app.rows.v1").unwrap(),
                CANIC_CORE_MEMORY_AUTHORITY
            ),
            Err(MemoryAllocationPoolError::AuthorityMismatch { .. })
        ));
        assert!(matches!(
            pool.validate_authority(
                &ic_memory::StableKey::parse("other.rows.v1").unwrap(),
                "other"
            ),
            Err(MemoryAllocationPoolError::UnclaimedKey { .. })
        ));
    }
}
