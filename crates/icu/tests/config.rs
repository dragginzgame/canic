#[test]
fn parse_minimal_config() {
    let toml = r#"
        controllers = ["aaaaa-aa"]

        [pool]
        minimum_size = 1

        [canisters.example]
        initial_cycles = "1T"
        uses_directory = false

        [standards]
        icrc21 = true
    "#;

    // Should parse and validate
    icu::config::Config::init_from_toml(toml).expect("init config");
    let cfg = icu::config::Config::try_get().expect("get config");
    assert!(cfg.icrc21_enabled());
}

