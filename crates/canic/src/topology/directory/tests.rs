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
        services: vec![service("hub", 1), service("two", 2)],
    };
    view.validate().unwrap();
    assert_eq!(view.by_name("two"), Some(view.services[1].canister));
    assert_eq!(view.by_name("hub"), Some(view.services[0].canister));
    assert_eq!(view.unique_by_role("two"), Err("service role missing"));
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
                    shard_pool: Some("p".repeat(64)),
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

#[cfg(feature = "auth")]
#[test]
fn approved_shard_topology_checks_both_install_envelopes() {
    use crate::{auth::install, topology::Install};
    let root = Principal::from_slice(&[3, 1]);
    let parent = Principal::from_slice(&[4, 1]);
    let direct = Install {
        root,
        parent,
        role: "user_shard".into(),
        application: vec![7; 16],
    }
    .encode()
    .unwrap();
    let mut auth = install::Install {
        storage: install::Storage::Volatile,
        root,
        issuer: parent,
        network: [1; 32],
        audience: [2; 32],
        role: "user".into(),
        limits: install::LimitsPolicy {
            default_session_secs: 60,
            max_session_secs: 60,
            max_proof_ttl_ns: 60,
            max_sessions: None,
        },
        application: direct.clone(),
    };
    for frame in [direct, auth.encode().unwrap()] {
        let selected = Install::approved(&frame).unwrap().unwrap();
        assert_eq!(
            (selected.root, selected.parent, selected.role.as_str()),
            (root, parent, "user_shard")
        );
        assert_eq!(selected.application, [7; 16]);
    }
    auth.root = parent;
    assert!(Install::approved(&auth.encode().unwrap()).is_err());
    auth.root = root;
    auth.application.pop();
    assert!(Install::approved(&auth.encode().unwrap()).is_err());
    auth.application = vec![5; 16];
    assert!(
        Install::approved(&auth.encode().unwrap())
            .unwrap()
            .is_none()
    );
    let mut truncated = auth.encode().unwrap();
    truncated.pop();
    assert!(Install::approved(&truncated).is_err());
}

#[cfg(feature = "storage")]
#[test]
fn an_unversioned_directory_cannot_reopen_as_a_pool_aware_index() {
    let key = "app.services.v1";
    let runtime =
        crate::storage::bootstrap(ic_storage::VectorMemory::default(), true, "app", &[key], 16);
    // Exact v1 field order. Empty children make this the ambiguous predecessor case.
    let old = ic_wire::encode(
        &(
            Principal::from_slice(&[8, 1]),
            "hub",
            None::<[u8; 32]>,
            Vec::<()>::new(),
            None::<Directory>,
        ),
        ic_wire::Encoding::Payload,
        DIRECTORY_LIMITS,
    )
    .unwrap();
    let cell = crate::storage::cell(&runtime, key, 8192, Some(old.clone()));
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            retained::Index::open(&runtime, key)
        }))
        .is_err()
    );
    assert_eq!(cell.get(), &old);
}

#[cfg(all(feature = "storage", feature = "method"))]
#[test]
fn acknowledged_children_include_only_confirmed_direct_children_of_the_role_after_reopen() {
    let route = || retained::get_shard("wallets", Principal::from_slice(&[3, 2]));
    assert_eq!(
        route(),
        Err(retained::ShardLookupError::DirectoryUnavailable)
    );
    let memory = ic_storage::DefaultMemoryImpl::default();
    let key = "app.services.v1";
    let open = |fresh| crate::storage::bootstrap(memory.clone(), fresh, "app", &[key], 16);
    let directory = Directory {
        revision: 1,
        services: vec![service("unrelated", 9)],
    };
    let children = ["shard", "shard", "shard", "worker", "shard"].map(|role| role.to_owned());
    let children: Vec<_> = children
        .into_iter()
        .zip(1u8..)
        .map(|(role, id)| Hop {
            shard_pool: match id {
                4 => None,
                5 => Some("projects".into()),
                _ => Some("wallets".into()),
            },
            canister: Principal::from_slice(&[id, 1]),
            role,
            generation: [id; 32],
        })
        .collect();
    let expected = [
        children[0].canister,
        children[1].canister,
        children[4].canister,
    ];
    {
        let runtime = open(true);
        let mut index =
            retained::Index::create(&runtime, key, Principal::from_slice(&[8, 1]), "hub");
        retained::DIRECTORY.with_borrow_mut(|slot| *slot = Some(index));
        assert_eq!(
            route(),
            Err(retained::ShardLookupError::DirectoryUnavailable)
        );
        index = retained::DIRECTORY.with_borrow_mut(Option::take).unwrap();
        index.apply(&directory, [7; 32], None).unwrap();
        retained::DIRECTORY.with_borrow_mut(|slot| *slot = Some(index));
        assert_eq!(route(), Err(retained::ShardLookupError::NoReadyShard));
        index = retained::DIRECTORY.with_borrow_mut(Option::take).unwrap();
        for child in &children {
            index.apply(&directory, [7; 32], Some(child)).unwrap();
        }
        for child in [&children[0], &children[1], &children[3], &children[4]] {
            index.acknowledge(&directory, child).unwrap();
        }
        assert_eq!(
            index.acknowledged_children("shard").collect::<Vec<_>>(),
            expected
        );
        let newer = Directory {
            revision: 2,
            ..directory
        };
        index.apply(&newer, [7; 32], None).unwrap();
        retained::DIRECTORY.with_borrow_mut(|slot| *slot = Some(index));
        assert_eq!(route(), Ok(expected[1]));
        retained::DIRECTORY.with_borrow_mut(Option::take);
    }
    let index = retained::Index::open(&open(false), key);
    assert_eq!(
        index.acknowledged_children("shard").collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        index.acknowledged_children("worker").collect::<Vec<_>>(),
        [children[3].canister]
    );
    assert_eq!(index.acknowledged_children("hub").count(), 0);
    retained::DIRECTORY.with_borrow_mut(|slot| *slot = Some(index));
    assert_eq!(route(), Ok(expected[1]));
    assert_eq!(
        retained::get_shard("projects", expected[0]),
        Ok(expected[2])
    );
    assert_eq!(
        retained::get_shard("shard", expected[0]),
        Err(retained::ShardLookupError::NoReadyShard)
    );
    retained::DIRECTORY.with_borrow_mut(Option::take);
}

#[cfg(all(feature = "storage", feature = "method"))]
#[test]
fn withdrawn_generation_refuses_delayed_delivery_and_ack_after_reopen() {
    let memory = ic_storage::VectorMemory::default();
    let key = "app.services.v1";
    let open = |fresh| crate::storage::bootstrap(memory.clone(), fresh, "app", &[key], 16);
    let directory = Directory {
        revision: 1,
        services: vec![],
    };
    let child = Hop {
        shard_pool: Some("users".into()),
        canister: Principal::from_slice(&[2, 1]),
        role: "shard".into(),
        generation: [1; 32],
    };
    let mut index =
        retained::Index::create(&open(true), key, Principal::from_slice(&[8, 1]), "hub");
    index.apply(&directory, [9; 32], Some(&child)).unwrap();
    index.withdraw((child.canister, child.generation)).unwrap();
    drop(index);
    let mut index = retained::Index::open(&open(false), key);
    let replacement = Hop {
        generation: [2; 32],
        ..child.clone()
    };
    index
        .apply(&directory, [9; 32], Some(&replacement))
        .unwrap();
    index.acknowledge(&directory, &replacement).unwrap();
    index.withdraw((child.canister, child.generation)).unwrap();
    assert!(index.apply(&directory, [9; 32], Some(&child)).is_err());
    assert!(index.acknowledge(&directory, &child).is_err());
    assert_eq!(
        index.child_delivery(child.canister),
        Some((&replacement, Some(1)))
    );
}

#[cfg(feature = "storage")]
#[test]
fn rendezvous_preserves_toko_wallet_routes_and_only_moves_keys_to_added_children() {
    let [a, b, c] = [1, 2, 3].map(|n| Principal::from_slice(&[n, 1]));
    let select = |children: &[Principal], key: &[u8]| {
        retained::select_child(children.iter().copied(), Principal::from_slice(key))
    };
    for (n, expected) in [(1, c), (2, b), (5, a), (29, a), (100, b)] {
        assert_eq!(select(&[a, b, c], &[n, 2]), Some(expected));
    }
    assert_eq!(select(&[a, b, c], &(0..29).collect::<Vec<_>>()), Some(c));
    for n in 0..=255 {
        let key = [n, 2];
        let old = select(&[a, b], &key);
        let new = select(&[b, c, a], &key);
        assert_eq!(new, select(&[c, a, b], &key));
        assert!(new == Some(c) || new == old);
    }
}
