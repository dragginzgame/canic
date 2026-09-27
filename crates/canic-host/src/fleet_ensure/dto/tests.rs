//! Generated desired documents must round-trip through the public TOML loader.

use super::*;
use crate::fleet_ensure::model::capacity_import::CapacityImportBootstrapRecord;
use candid::Principal;

/// Exercise the real generator output in the ordinary host suite before PocketIC.
pub(in crate::fleet_ensure) fn qualify_generated_toml(root: &Path, desired: &DesiredFleet) {
    let path = root.join("desired-roundtrip.toml");
    let hold = CapacityImportBootstrapRecord {
        review_sha256: [7; 32],
        operator: Principal::from_text(&desired.operator).unwrap(),
        sources: vec![Principal::from_slice(&[8])],
    };
    for expected_hold in [None, Some(hold)] {
        let mut expected = desired.clone();
        expected.bootstrap.as_mut().unwrap().roots[0].capacity_import_bootstrap = expected_hold;
        let text = toml::to_string_pretty(&expected).unwrap();
        fs::write(&path, &text).unwrap();
        let loaded = load_desired_fleet(&path).expect("load freshly generated TOML");
        assert_eq!(loaded.desired, expected);
        assert_eq!(loaded.sha256, sha256_hex(text.as_bytes()));

        let mut document: toml::Value = toml::from_str(&text).unwrap();
        if let Some(hold) = document["bootstrap"]["roots"][0]
            .as_table_mut()
            .unwrap()
            .get_mut("capacity_import_bootstrap")
        {
            hold.as_table_mut().unwrap().remove("operator");
        } else {
            // Required recovery-controller authority remains explicit in TOML.
            document["bootstrap"]
                .as_table_mut()
                .unwrap()
                .remove("recovery_controllers");
        }
        fs::write(&path, toml::to_string_pretty(&document).unwrap()).unwrap();
        assert!(matches!(
            load_desired_fleet(&path),
            Err(DesiredFleetLoadError::Parse { .. })
        ));
    }
    fs::remove_file(path).unwrap();
}
