use crate::{
    cdk::utils::time,
    config::{Config, schema::ConfigModel},
    log::Level,
    model::memory::log::{LogEntry, StableLog, apply_retention},
    types::PageRequest,
};

///
/// TESTS
///

#[test]
fn retention_trims_old_and_excess_entries() {
    Config::reset_for_tests();
    let mut cfg = ConfigModel::test_default();
    cfg.log.max_entries = 2;
    cfg.log.max_age_secs = Some(5);
    Config::init_from_toml(&toml::to_string(&cfg).unwrap()).unwrap();

    let now = time::now_secs();
    // fresh entry
    StableLog::append("test", Option::<&str>::None, Level::Info, "fresh1").unwrap();

    // old entry (backdated)
    let mut old = LogEntry::new("test", Level::Info, None, "old");
    old.created_at = now.saturating_sub(10);
    StableLog::append_entry(old).unwrap();

    // another fresh entry
    StableLog::append("test", Option::<&str>::None, Level::Info, "fresh2").unwrap();

    apply_retention().unwrap();

    let (entries, total) =
        StableLog::entries_page_filtered(None, None, None, PageRequest::new(10, 0));
    assert_eq!(total, 2);
    let msgs: Vec<_> = entries.into_iter().map(|(_, e)| e.message).collect();
    assert!(msgs.contains(&"fresh1".to_string()));
    assert!(msgs.contains(&"fresh2".to_string()));
    assert!(!msgs.contains(&"old".to_string()));
}
