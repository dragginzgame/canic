use super::*;

fn service(name: &str, id: u8) -> Service {
    Service {
        name: name.into(),
        role: "hub".into(),
        canister: Principal::from_slice(&[id, 1]),
    }
}

#[test]
fn service_names_and_principals_are_unique_and_roles_can_repeat() {
    let mut view = Directory {
        revision: 1,
        services: vec![service("one", 1), service("two", 2)],
    };
    view.validate().unwrap();
    assert_eq!(
        view.by_role("hub").collect::<Vec<_>>(),
        [view.services[0].canister, view.services[1].canister]
    );
    assert_eq!(view.by_role("child").count(), 0);
    view.services.reverse();
    assert!(view.validate().is_err());
    view.services = vec![service("one", 1), service("two", 1)];
    assert!(view.validate().is_err());
}

#[cfg(feature = "storage")]
#[test]
fn replacement_survives_reopen_and_late_delivery() {
    use ic_storage::VectorMemory;
    let backing = VectorMemory::default();
    let key = "app.services.v1";
    let open = |fresh| crate::storage::bootstrap(backing.clone(), fresh, "app", &[key], 16);
    let before = Directory {
        revision: 1,
        services: vec![service("one", 1)],
    };
    let after = Directory {
        revision: 3,
        services: vec![service("one", 2)],
    };
    {
        let runtime = open(true);
        let mut index = retained::Index::create(&runtime, key, Principal::from_slice(&[8, 1]));
        assert!(index.current().is_none());
        index.commit(before.clone()).unwrap();
        index.commit(after.clone()).unwrap();
    }
    let runtime = open(false);
    let mut index = retained::Index::open(&runtime, key);
    assert_eq!(index.publisher(), Principal::from_slice(&[8, 1]));
    index.commit(after.clone()).unwrap();
    assert!(index.commit(before).is_err());
    assert!(
        index
            .commit(Directory {
                revision: 3,
                services: vec![]
            })
            .is_err()
    );
    assert_eq!(index.current(), Some(&after));
}

#[test]
fn full_directory_fits_and_bad_publications_are_refused() {
    let mut directory = Directory {
        revision: u64::MAX,
        services: (0..32)
            .map(|id| Service {
                name: format!("{id:064}"),
                role: "a".repeat(64),
                canister: Principal::from_slice(&{
                    let mut bytes = [1; 29];
                    bytes[0] = id;
                    bytes
                }),
            })
            .collect(),
    };
    directory.validate().unwrap();
    // Exercise the same encode and strict reopen budgets used by installed actors.
    #[cfg(feature = "storage")]
    {
        let memory = ic_storage::VectorMemory::default();
        let key = "app.services.v1";
        let open = |fresh| crate::storage::bootstrap(memory.clone(), fresh, "app", &[key], 16);
        let publisher = Principal::from_slice(&[1; 29]);
        let runtime = open(true);
        let mut index = retained::Index::create(&runtime, key, publisher);
        index.commit(directory.clone()).unwrap();
        drop(index);
        drop(runtime);
        assert_eq!(
            retained::Index::open(&open(false), key).current(),
            Some(&directory)
        );
    }
    directory.services.push(service("overflow", 32));
    assert!(directory.validate().is_err());
    directory.services.pop();
    for principal in [Principal::anonymous(), Principal::management_canister()] {
        directory.services[0].canister = principal;
        assert!(directory.validate().is_err());
    }
    directory.services.clear();
    directory.validate().unwrap();
    directory.revision = 0;
    assert!(directory.validate().is_err());
}
