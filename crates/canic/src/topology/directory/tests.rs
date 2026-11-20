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
    assert_eq!(view.by_name("two"), Some(view.services[1].canister));
    assert_eq!(view.by_name("hub"), None);
    assert_eq!(
        view.by_role("hub").collect::<Vec<_>>(),
        [view.services[0].canister, view.services[1].canister]
    );
    assert_eq!(view.by_role("child").count(), 0);
    assert_eq!(view.unique_by_role("child"), Err("service role missing"));
    assert_eq!(view.unique_by_role("hub"), Err("service role ambiguous"));
    view.services[1].role = "worker".into();
    assert_eq!(view.unique_by_role("hub"), Ok(view.services[0].canister));
    view.services.reverse();
    assert!(view.validate().is_err());
    view.services = vec![service("one", 1), service("two", 1)];
    assert!(view.validate().is_err());
}

#[cfg(feature = "storage")]
#[test]
fn published_bindings_survive_growth_reopen_and_late_delivery() {
    use ic_storage::VectorMemory;
    let backing = VectorMemory::default();
    let key = "app.services.v1";
    let open = |fresh| crate::storage::bootstrap(backing.clone(), fresh, "app", &[key], 16);
    let before = Directory {
        revision: 1,
        services: vec![service("one", 1)],
    };
    let after = Directory {
        revision: 2,
        services: vec![service("one", 1), service("two", 2)],
    };
    {
        let runtime = open(true);
        let mut index =
            retained::Index::create(&runtime, key, Principal::from_slice(&[8, 1]), "hub");
        assert!(index.current().is_none());
        index.apply(&before, [7; 32], None).unwrap();
        index.apply(&after, [7; 32], None).unwrap();
    }
    let runtime = open(false);
    let mut index = retained::Index::open(&runtime, key);
    assert_eq!(index.publisher(), Principal::from_slice(&[8, 1]));
    index.apply(&after, [7; 32], None).unwrap();
    assert_eq!(
        index.apply(&after, [8; 32], None),
        Err("conflicting receiver installation")
    );
    assert!(index.apply(&before, [7; 32], None).is_err());
    for services in [
        vec![service("one", 3), service("two", 2)],
        vec![service("two", 2)],
        {
            let mut rows = after.services.clone();
            rows[0].role = "worker".into();
            rows
        },
    ] {
        assert_eq!(
            index.apply(
                &Directory {
                    revision: 3,
                    services
                },
                [7; 32],
                None
            ),
            Err("published service binding changed")
        );
    }
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
        let publisher = Principal::from_slice(&[99, 1]);
        // The local index retains child bindings alongside all named services.
        let children: Vec<_> = (32..48)
            .map(|id| {
                let mut bytes = [1; 29];
                bytes[0] = id;
                Hop {
                    canister: Principal::from_slice(&bytes),
                    role: "w".repeat(64),
                    generation: [u8::MAX; 32],
                }
            })
            .collect();
        let runtime = open(true);
        let mut index = retained::Index::create(&runtime, key, publisher, "hub");
        for child in &children {
            index.apply(&directory, [7; 32], Some(child)).unwrap();
        }
        drop(index);
        drop(runtime);
        let reopened = retained::Index::open(&open(false), key);
        assert_eq!(reopened.current(), Some(&directory));
        for child in &children {
            assert_eq!(reopened.child_delivery(child.canister), Some((child, None)));
        }
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
