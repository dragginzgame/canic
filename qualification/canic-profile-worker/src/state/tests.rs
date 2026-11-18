use super::*;
use ic_storage::VectorMemory;
use ic_storage::{AllocationGrants, GenericAllocationPolicy, MemoryAuthority, adapter::StableCell};

#[test]
fn composed_memory_preserves_other_owner_and_refuses_ungranted_or_missing_profiles() {
    fn runtime(backing: VectorMemory, authority: &str) -> MemoryRuntime<VectorMemory> {
        let mut runtime = MemoryRuntime::new(backing).unwrap();
        let requests = [ROWS, OWNERS].map(|key| MemoryRequest::new(authority, key).unwrap());
        runtime
            .bootstrap(
                &SealedDeclarationSnapshot::new(
                    requests
                        .into_iter()
                        .chain([MemoryRequest::new("catalog", "catalog.record.v1").unwrap()])
                        .collect(),
                )
                .unwrap(),
                &AllocationGrants::new(vec![
                    MemoryAuthority::new(authority, STORAGE_PREFIX).unwrap(),
                    MemoryAuthority::new("catalog", "catalog.").unwrap(),
                ])
                .unwrap(),
                &GenericAllocationPolicy,
            )
            .unwrap();
        runtime
    }
    {
        let backing = VectorMemory::default();
        let host = runtime(backing.clone(), STORAGE_AUTHORITY);
        let before = backing.borrow().clone();
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| State::open(&host))).is_err()
        );
        assert_eq!(
            *backing.borrow(),
            before,
            "reopen must not initialize missing maps"
        );
    }
    let backing = VectorMemory::default();
    {
        let host = runtime(backing.clone(), STORAGE_AUTHORITY);
        let mut profiles = State::create(&host);
        let catalog = StableCell::create(
            host.open_memory(&StableKey::parse("catalog.record.v1").unwrap())
                .unwrap(),
            41_u64,
        );
        profiles
            .put(
                &[1],
                &Put {
                    user_id: 1,
                    expected_revision: 0,
                    display_name: "shared runtime".into(),
                },
            )
            .unwrap();
        assert_eq!(*catalog.get(), 41);
        for key in [ROWS, OWNERS, "catalog.record.v1"] {
            assert!(host.open_memory(&StableKey::parse(key).unwrap()).is_err());
        }
    }
    {
        let host = runtime(backing.clone(), STORAGE_AUTHORITY);
        let profiles = State::open(&host);
        assert_eq!(
            profiles.get(&[1], 1).unwrap().display_name,
            "shared runtime"
        );
        let catalog = StableCell::<u64>::load(
            host.open_memory(&StableKey::parse("catalog.record.v1").unwrap())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(*catalog.get(), 41);
    }
    let before = backing.borrow().clone();
    let host = runtime(backing.clone(), "unrelated");
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| State::open(&host))).is_err());
    assert_eq!(
        *backing.borrow(),
        before,
        "matching keys do not grant profile authority"
    );
}

#[test]
fn ownership_revision_capacity_and_strict_reopen() {
    let backing = VectorMemory::default();
    let mut state = crate::storage::open(backing.clone(), true);
    let input = Put {
        user_id: 1,
        expected_revision: 0,
        display_name: "Private profile".into(),
    };
    assert_eq!(state.put(&[4], &input), Err(ProfileError::Unauthenticated));
    let registered = state.put(&[1], &input).unwrap();
    assert_eq!(state.put(&[1], &input).unwrap(), registered);
    assert_eq!(state.get(&[2], 1), Err(ProfileError::Forbidden));
    assert_eq!(state.put(&[2], &input), Err(ProfileError::Forbidden));
    assert_eq!(
        state.put(
            &[1],
            &Put {
                user_id: 2,
                ..input.clone()
            }
        ),
        Err(ProfileError::AlreadyRegistered)
    );
    assert_eq!(
        state.put(
            &[1],
            &Put {
                display_name: "stale".into(),
                ..input.clone()
            }
        ),
        Err(ProfileError::RevisionConflict)
    );
    assert_eq!(
        state.put(
            &[1],
            &Put {
                display_name: "x".repeat(129),
                ..input.clone()
            }
        ),
        Err(ProfileError::InvalidInput)
    );
    for id in 2..=MAX_PROFILES {
        state
            .put(
                &[id as u8, 1],
                &Put {
                    user_id: id,
                    ..input.clone()
                },
            )
            .unwrap();
    }
    assert_eq!(
        state.put(
            &[99, 1],
            &Put {
                user_id: 99,
                ..input.clone()
            }
        ),
        Err(ProfileError::Capacity)
    );
    let changed = state
        .put(
            &[1],
            &Put {
                expected_revision: 1,
                display_name: "Updated".into(),
                ..input
            },
        )
        .unwrap();
    assert_eq!(changed.revision, 2);
    drop(state);
    let state = crate::storage::open(backing, false);
    assert_eq!(state.get(&[1], 1).unwrap(), changed);
    assert_eq!(state.get(&[2], 1), Err(ProfileError::Forbidden));
}
