use super::*;
use crate::fleet_ensure::{ops, tests::protocol_tranche_fixture};
use std::fs;

fn status(principal: &str, operator: &str) -> IcpCanisterStatusReport {
    serde_json::from_value(serde_json::json!({
        "id": principal, "settings": { "controllers": [operator] }, "cycles": "700",
    }))
    .unwrap()
}

#[test]
fn native_readiness_requires_exact_identity_controllers_and_real_balance() {
    let principal = Principal::self_authenticating(b"root").to_text();
    let operator = Principal::self_authenticating(b"operator");
    let expected = BTreeSet::from([operator]);
    let mut original = status(&principal, &operator.to_text());
    original.cycles = Some("407_037_311_267_631".into());
    assert_eq!(
        native_balance(&principal, &expected, &original),
        Ok(407_037_311_267_631)
    );
    for change in 0..4 {
        let mut report = original.clone();
        match change {
            0 => report.id = operator.to_text(),
            1 => report.settings.as_mut().unwrap().controllers.clear(),
            2 => report
                .settings
                .as_mut()
                .unwrap()
                .controllers
                .push(operator.to_text()),
            3 => report.cycles = None,
            _ => unreachable!(),
        }
        assert_eq!(
            native_balance(&principal, &expected, &report),
            Err(if change == 3 {
                RootReadinessUnavailable::BalanceUnavailable
            } else {
                RootReadinessUnavailable::AuthorityMismatch
            })
        );
    }
}

#[test]
fn native_readiness_accepts_exact_plain_and_grouped_cycle_counts() {
    let principal = Principal::self_authenticating(b"root").to_text();
    let operator = Principal::self_authenticating(b"operator");
    let expected = BTreeSet::from([operator]);
    for (cycles, amount) in [
        ("0", 0),
        ("700", 700),
        ("1_000", 1000),
        ("407_037_311_267_631", 407_037_311_267_631),
        ("340282366920938463463374607431768211455", u128::MAX),
        (
            "340_282_366_920_938_463_463_374_607_431_768_211_455",
            u128::MAX,
        ),
    ] {
        let report = serde_json::from_value(serde_json::json!({
            "id": principal,
            "settings": { "controllers": [operator.to_text()] },
            "cycles": cycles,
        }))
        .unwrap();
        assert_eq!(native_balance(&principal, &expected, &report), Ok(amount));
    }
}

#[test]
fn native_readiness_rejects_malformed_or_overflowing_cycle_counts() {
    let principal = Principal::self_authenticating(b"root").to_text();
    let operator = Principal::self_authenticating(b"operator");
    let expected = BTreeSet::from([operator]);
    for cycles in [
        "",
        "_",
        "_700",
        "700_",
        "1__000",
        "12_34",
        "1234_567",
        "1_0000",
        "-1",
        "+1",
        "1.5",
        "1T",
        "1 cycles",
        " 700",
        "700 ",
        "1 000",
        "١",
        "340282366920938463463374607431768211456",
        "340_282_366_920_938_463_463_374_607_431_768_211_456",
        "0000000000000000000000000000000000000000000000000000",
    ] {
        let mut report = status(&principal, &operator.to_text());
        report.cycles = Some(cycles.into());
        assert_eq!(
            native_balance(&principal, &expected, &report),
            Err(RootReadinessUnavailable::BalanceUnavailable)
        );
    }
}

#[test]
fn grouped_native_balance_reports_exact_floor_shortfall() {
    let mut fixture = protocol_tranche_fixture(Vec::new());
    fixture.desired.bootstrap = None;
    fixture.desired.protocol = None;
    let mut root = fixture.desired.canisters[0].clone();
    root.kind = DesiredCanisterKind::Root;
    root.name = "root".into();
    root.principal = Some(Principal::self_authenticating(b"root").to_text());
    root.controllers = vec![fixture.desired.operator.clone()];
    root.controller_canisters.clear();
    root.minimum_cycles = "500T".into();
    fixture.desired.canisters.push(root);
    for (cycles, shortfall) in [
        ("407_037_311_267_631", 92_962_688_732_369),
        ("600_000_000_000_000", 0),
    ] {
        let funding = roots_with(&fixture.root, &fixture.desired, |principal| {
            let mut report = status(principal, &fixture.desired.operator);
            report.cycles = Some(cycles.into());
            Ok(report)
        })
        .unwrap();
        let observed = &funding.roots[0];
        assert_eq!(observed.required_native_floor_cycles, 500_000_000_000_000);
        assert_eq!(observed.floor_shortfall_cycles, Some(shortfall));
        assert!(observed.available_native_cycles.is_some());
        assert_eq!(observed.unavailable, None);
    }
    fs::remove_dir_all(fixture.root).unwrap();
}

#[test]
fn root_floor_is_reported_without_artifacts_or_assumed_execution_reserve() {
    let mut fixture = protocol_tranche_fixture(Vec::new());
    fixture.desired.bootstrap = None;
    fixture.desired.protocol = None;
    let mut root = fixture.desired.canisters[0].clone();
    root.kind = DesiredCanisterKind::Root;
    root.name = "root".into();
    root.principal = Some(Principal::self_authenticating(b"root").to_text());
    root.controllers = vec![fixture.desired.operator.clone()];
    root.controller_canisters.clear();
    root.minimum_cycles = "0.000001B".into();
    root.wasm = Some("unbuilt-artifact.wasm".into());
    fixture.desired.canisters.push(root.clone());
    let funding = roots_with(&fixture.root, &fixture.desired, |principal| {
        Ok(status(principal, &fixture.desired.operator))
    })
    .unwrap();
    assert_eq!(funding.roots.len(), 1);
    let observed = &funding.roots[0];
    assert_eq!(observed.required_native_floor_cycles, 1000);
    assert_eq!(observed.available_native_cycles, Some(700));
    assert_eq!(observed.floor_shortfall_cycles, Some(300));
    assert_eq!(observed.startup_minimum_cycles, None);
    assert!(
        funding
            .unresolved
            .contains(&ReadinessUnresolved::ArtifactExecutionReserve)
    );
    assert!(
        funding
            .unresolved
            .contains(&ReadinessUnresolved::StartupConfiguration)
    );
    assert!(!fixture.root.join("unbuilt-artifact.wasm").exists());
    let unavailable = roots_with(&fixture.root, &fixture.desired, |_| {
        Err(RootReadinessUnavailable::ObservationFailed)
    })
    .unwrap();
    assert_eq!(unavailable.roots[0].floor_shortfall_cycles, None);
    let paths = EnsurePaths::under(&fixture.root, "local", "test-fleet");
    let mut state = read_state(&paths, "test-fleet").unwrap();
    state
        .principals
        .insert("root".into(), fixture.desired.operator.clone());
    ops::write_state(&paths, &state).unwrap();
    // A snapshot cannot join balances to identities that changed during collection.
    let mut original_state = state.clone();
    original_state.principals.remove("root");
    ops::write_state(&paths, &original_state).unwrap();
    let drifted = roots_with(&fixture.root, &fixture.desired, |principal| {
        ops::write_state(&paths, &state).unwrap();
        Ok(status(principal, &fixture.desired.operator))
    });
    assert!(matches!(
        drifted,
        Err(EnsureStateError::StartupConfigurationMismatch)
    ));
    let changed = roots_with(&fixture.root, &fixture.desired, |_| {
        panic!("conflicting identities must not be queried")
    })
    .unwrap();
    assert_eq!(
        changed.roots[0].unavailable,
        Some(RootReadinessUnavailable::AuthorityMismatch)
    );
    assert_eq!(changed.roots[0].available_native_cycles, None);
    fs::remove_dir_all(fixture.root).unwrap();
}

#[test]
#[ignore = "read-only configuration inspection of an explicitly selected workspace"]
fn readiness_inspects_selected_configuration_without_building() {
    let workspace = std::path::PathBuf::from(
        std::env::var_os("CANIC_RETAINED_REVIEW_WORKSPACE").expect("selected workspace"),
    );
    let desired_path = std::env::var_os("CANIC_READINESS_DESIRED").expect("selected desired path");
    let selected = crate::fleet_ensure::load_desired_fleet(&workspace.join(desired_path)).unwrap();
    let funding = roots_with(&workspace, &selected.desired, |_| {
        Err(RootReadinessUnavailable::ObservationFailed)
    })
    .unwrap();
    assert!(!funding.roots.is_empty());
    for root in &funding.roots {
        assert!(root.startup_minimum_cycles.is_some());
        assert!(root.available_native_cycles.is_none());
    }
    println!("desired={}; funding={funding:?}", selected.sha256);
}
