use super::*;
use crate::config::schema::MetricProviderStorage;
use ic_memory::{MemoryManagerConfig, VectorMemory, testing::fixture_runtime};
use ic_storage::adapter::StableBTreeMap;

const DEFINITIONS: &[MetricDefinition] = &[MetricDefinition {
    kind: MetricKind::Counter,
    name: "accepted",
    unit: "events",
}];
const KEY: &str = "metrics.rows.v1";
fn mixed() -> MetricStorageConfig {
    MetricStorageConfig {
        storage: MetricStorage::Heap,
        providers: [(
            "blob".into(),
            MetricProviderStorage {
                storage: MetricStorage::Stable,
            },
        )]
        .into(),
    }
}
fn declared() -> ProviderMetrics<VectorMemory> {
    let mut registry = ProviderMetrics::default();
    registry.register("blob", DEFINITIONS).unwrap();
    registry.register("auth", DEFINITIONS).unwrap();
    registry
}
#[test]
fn default_heap_does_not_open_storage_and_new_window_starts_empty() {
    let mut registry = declared();
    registry
        .activate(&MetricStorageConfig::default(), true, 7, || {
            panic!("heap cannot open storage")
        })
        .unwrap();
    registry.add("blob", "accepted", 0).unwrap();
    registry.add("auth", "accepted", 42).unwrap();
    assert_eq!(
        registry.snapshot("blob").unwrap()[0]
            .value
            .counter()
            .unwrap()
            .total(),
        0
    );
    assert_eq!(
        registry.snapshot("auth").unwrap()[0]
            .value
            .counter()
            .unwrap()
            .total(),
        42
    );
    drop(registry);
    let mut reopened = declared();
    reopened
        .activate(&MetricStorageConfig::default(), false, 8, || {
            panic!("heap restore cannot open storage")
        })
        .unwrap();
    let observed = reopened.snapshot("auth").unwrap();
    assert_eq!(observed[0].value.counter().unwrap().total(), 0);
    assert_eq!(observed[0].window, 8);
}
#[test]
fn mixed_providers_share_one_store_and_only_stable_counters_survive_reopen() {
    let memory = VectorMemory::default();
    let runtime = fixture_runtime(
        memory,
        "metrics",
        "metrics.",
        &[KEY],
        MemoryManagerConfig::new(1).unwrap(),
    )
    .unwrap();
    let mut registry = declared();
    registry
        .activate(&mixed(), true, 7, || {
            Ok(MetricsStore::new(StableBTreeMap::new(
                runtime.open_memory(KEY).unwrap(),
            )))
        })
        .unwrap();
    registry.add("blob", "accepted", 42).unwrap();
    registry.add("auth", "accepted", 17).unwrap();
    drop(registry);
    let mut reopened = declared();
    reopened
        .activate(&mixed(), false, 8, || {
            Ok(MetricsStore::new(StableBTreeMap::load(
                runtime.open_memory(KEY).unwrap(),
            )))
        })
        .unwrap();
    let durable = reopened.snapshot("blob").unwrap();
    assert_eq!(durable[0].value.counter().unwrap().total(), 42);
    assert_eq!(durable[0].window, 8);
    assert_eq!(durable[0].storage, MetricStorage::Stable);
    assert_eq!(
        reopened.snapshot("auth").unwrap()[0]
            .value
            .counter()
            .unwrap()
            .total(),
        0
    );
    assert_eq!(reopened.snapshot("auth").unwrap()[0].window, 8);
}
#[test]
fn caps_are_exact_until_actual_overflow_and_quality_survives_storage() {
    let memory = VectorMemory::default();
    let runtime = fixture_runtime(
        memory,
        "metrics",
        "metrics.",
        &[KEY],
        MemoryManagerConfig::new(1).unwrap(),
    )
    .unwrap();
    let mut registry = declared();
    registry
        .activate(&mixed(), true, 7, || {
            Ok(MetricsStore::new(StableBTreeMap::new(
                runtime.open_memory(KEY).unwrap(),
            )))
        })
        .unwrap();
    registry.add("blob", "accepted", u128::MAX).unwrap();
    registry.add("blob", "accepted", 0).unwrap();
    assert!(
        !registry.snapshot("blob").unwrap()[0]
            .value
            .counter()
            .unwrap()
            .total_overflowed()
    );
    registry.add("blob", "accepted", 1).unwrap();
    assert!(
        registry.snapshot("blob").unwrap()[0]
            .value
            .counter()
            .unwrap()
            .total_overflowed()
    );
    drop(registry);
    let mut reopened = declared();
    reopened
        .activate(&mixed(), false, 8, || {
            Ok(MetricsStore::new(StableBTreeMap::load(
                runtime.open_memory(KEY).unwrap(),
            )))
        })
        .unwrap();
    assert_eq!(
        reopened.snapshot("blob").unwrap()[0]
            .value
            .counter()
            .unwrap()
            .total(),
        u128::MAX
    );
    assert!(
        reopened.snapshot("blob").unwrap()[0]
            .value
            .counter()
            .unwrap()
            .total_overflowed()
    );
}
#[test]
fn unknown_configuration_and_duplicate_rosters_refuse_before_store_open() {
    let mut registry = declared();
    assert!(matches!(
        registry.register("blob", DEFINITIONS),
        Err(ProviderMetricsError::Duplicate)
    ));
    let config = MetricStorageConfig {
        providers: [(
            "blbo".into(),
            MetricProviderStorage {
                storage: MetricStorage::Stable,
            },
        )]
        .into(),
        ..Default::default()
    };
    assert!(matches!(
        registry.activate(&config, true, 7, || panic!("bad policy opened storage")),
        Err(ProviderMetricsError::UnknownProvider)
    ));
    registry
        .activate(&MetricStorageConfig::default(), true, 7, || panic!())
        .unwrap();
    assert!(matches!(
        registry.add("blob", "request-derived-name", 1),
        Err(ProviderMetricsError::UnknownCounter)
    ));
    assert_eq!(
        registry.snapshot("blob").unwrap()[0]
            .value
            .counter()
            .unwrap()
            .total(),
        0
    );
    assert!(matches!(
        registry.register("late", DEFINITIONS),
        Err(ProviderMetricsError::Lifecycle)
    ));
}
#[test]
fn restore_never_initializes_missing_stable_counter_records() {
    let memory = VectorMemory::default();
    let runtime = fixture_runtime(
        memory.clone(),
        "metrics",
        "metrics.",
        &[KEY],
        MemoryManagerConfig::new(1).unwrap(),
    )
    .unwrap();
    drop(MetricsStore::new(
        StableBTreeMap::<Vec<u8>, Vec<u8>, _>::new(runtime.open_memory(KEY).unwrap()),
    ));
    let before = memory.borrow().clone();
    let mut registry = declared();
    assert!(matches!(
        registry.activate(&mixed(), false, 8, || Ok(MetricsStore::new(
            StableBTreeMap::load(runtime.open_memory(KEY).unwrap())
        ))),
        Err(ProviderMetricsError::Missing)
    ));
    assert_eq!(*memory.borrow(), before);
    assert!(matches!(
        registry.snapshot("blob"),
        Err(ProviderMetricsError::Lifecycle)
    ));
}
#[test]
fn rejected_storage_update_keeps_old_value_and_makes_reporting_unavailable() {
    let runtime = fixture_runtime(
        VectorMemory::default(),
        "metrics",
        "metrics.",
        &[KEY],
        MemoryManagerConfig::new(1).unwrap(),
    )
    .unwrap();
    let mut registry = declared();
    registry
        .activate(&mixed(), true, 7, || {
            Ok(MetricsStore::new(StableBTreeMap::new(
                runtime.open_memory(KEY).unwrap(),
            )))
        })
        .unwrap();
    let mut ahead = MeasurementCounter::ZERO;
    ahead.add(100);
    registry
        .stable
        .as_mut()
        .unwrap()
        .save_counter(
            &MetricKey::new("blob", "accepted").unwrap(),
            &MetricIdentity::new("events", 0).unwrap(),
            &ahead,
        )
        .unwrap();
    assert!(matches!(
        registry.add("blob", "accepted", 1),
        Err(ProviderMetricsError::Store(_))
    ));
    assert!(matches!(
        registry.snapshot("blob"),
        Err(ProviderMetricsError::Unavailable)
    ));
    assert!(matches!(
        registry.add("blob", "accepted", 1),
        Err(ProviderMetricsError::Unavailable)
    ));
}

#[test]
fn retained_measurements_keep_real_zero_distinct_from_no_samples() {
    const MEASUREMENTS: &[MetricDefinition] = &[MetricDefinition {
        name: "instructions",
        unit: "instructions",
        kind: MetricKind::Measurement,
    }];
    let runtime = fixture_runtime(
        VectorMemory::default(),
        "metrics",
        "metrics.",
        &[KEY],
        MemoryManagerConfig::new(1).unwrap(),
    )
    .unwrap();
    let mut registry = ProviderMetrics::default();
    registry.register("blob", MEASUREMENTS).unwrap();
    registry
        .activate(&mixed(), true, 7, || {
            Ok(MetricsStore::new(StableBTreeMap::new(
                runtime.open_memory(KEY).unwrap(),
            )))
        })
        .unwrap();
    let empty = registry.snapshot("blob").unwrap()[0]
        .value
        .measurement()
        .unwrap();
    assert_eq!(empty.samples(), 0);
    assert_eq!(empty.maximum(), None);
    assert!(matches!(
        registry.add("blob", "instructions", 1),
        Err(ProviderMetricsError::WrongKind)
    ));
    registry.record("blob", "instructions", 0).unwrap();
    drop(registry);
    let mut reopened = ProviderMetrics::default();
    reopened.register("blob", MEASUREMENTS).unwrap();
    reopened
        .activate(&mixed(), false, 8, || {
            Ok(MetricsStore::new(StableBTreeMap::load(
                runtime.open_memory(KEY).unwrap(),
            )))
        })
        .unwrap();
    let zero = reopened.snapshot("blob").unwrap()[0]
        .value
        .measurement()
        .unwrap();
    assert_eq!(zero.samples(), 1);
    assert_eq!(zero.total(), 0);
    assert_eq!(zero.maximum(), Some(0));
    assert_eq!(zero.mean().unwrap(), Some(0));
}

#[test]
fn exact_roster_caps_and_rejected_declarations_never_consume_a_slot() {
    const DUPLICATED: &[MetricDefinition] = &[
        MetricDefinition {
            kind: MetricKind::Counter,
            name: "same",
            unit: "events",
        },
        MetricDefinition {
            kind: MetricKind::Counter,
            name: "same",
            unit: "events",
        },
    ];
    let mut registry: ProviderMetrics<VectorMemory> = ProviderMetrics::default();
    assert!(matches!(
        registry.register("p0", DUPLICATED),
        Err(ProviderMetricsError::Duplicate)
    ));
    for index in 0..MAX_PROVIDERS {
        let provider = Box::leak(format!("p{index}").into_boxed_str());
        registry.register(provider, DEFINITIONS).unwrap();
    }
    assert!(matches!(
        registry.register("overflow", DEFINITIONS),
        Err(ProviderMetricsError::Capacity)
    ));
    registry
        .activate(&MetricStorageConfig::default(), true, 1, || panic!())
        .unwrap();
    assert_eq!(registry.snapshot("p0").unwrap().len(), 1);

    let mut full: ProviderMetrics<VectorMemory> = ProviderMetrics::default();
    let definitions = (0..MAX_COUNTERS)
        .map(|index| MetricDefinition {
            kind: MetricKind::Counter,
            name: Box::leak(format!("metric{index}").into_boxed_str()),
            unit: "events",
        })
        .collect::<Vec<_>>();
    full.register("full", Box::leak(definitions.into_boxed_slice()))
        .unwrap();
    assert!(matches!(
        full.register("extra", DEFINITIONS),
        Err(ProviderMetricsError::Capacity)
    ));
    full.activate(&MetricStorageConfig::default(), true, 1, || panic!())
        .unwrap();
    assert_eq!(full.snapshot("full").unwrap().len(), MAX_COUNTERS);
}

#[test]
fn malformed_retained_record_is_not_an_empty_or_fresh_counter() {
    let memory = VectorMemory::default();
    let runtime = fixture_runtime(
        memory.clone(),
        "metrics",
        "metrics.",
        &[KEY],
        MemoryManagerConfig::new(1).unwrap(),
    )
    .unwrap();
    let mut registry = declared();
    registry
        .activate(&mixed(), true, 1, || {
            Ok(MetricsStore::new(StableBTreeMap::new(
                runtime.open_memory(KEY).unwrap(),
            )))
        })
        .unwrap();
    let mut map = registry.stable.take().unwrap().into_map();
    map.insert(b"blob\0accepted".to_vec(), vec![0xff]);
    drop(map);
    drop(registry);
    let before = memory.borrow().clone();
    let mut reopened = declared();
    assert!(matches!(
        reopened.activate(&mixed(), false, 2, || Ok(MetricsStore::new(
            StableBTreeMap::load(runtime.open_memory(KEY).unwrap())
        ))),
        Err(ProviderMetricsError::Store(_))
    ));
    assert_eq!(*memory.borrow(), before);
}
