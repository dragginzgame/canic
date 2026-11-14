use super::*;
use ic_storage::VectorMemory;

#[test]
fn ownership_revision_capacity_and_strict_reopen() {
    let backing = VectorMemory::default();
    let mut state = State::open(backing.clone(), true);
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
    let state = State::open(backing, false);
    assert_eq!(state.get(&[1], 1).unwrap(), changed);
    assert_eq!(state.get(&[2], 1), Err(ProfileError::Forbidden));
}
