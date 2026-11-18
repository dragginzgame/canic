use super::*;
use ic_platform_types::Principal;

#[test]
fn assigned_target_preserves_generation_and_refuses_invalid_principals() {
    let original = Allocation {
        slot: 2,
        generation: [7; 32],
    };
    let target = Principal::from_slice(&[4, 1]);
    let placement = |text| Placement::Assigned(original.clone(), text);
    assert_eq!(
        placement(target.to_text()).assigned_target().unwrap(),
        Some((&original, target))
    );
    for text in [
        target.to_text().to_uppercase(),
        Principal::anonymous().to_text(),
        Principal::management_canister().to_text(),
        Principal::from_slice(&[7, 2]).to_text(),
        "invalid".into(),
    ] {
        assert!(placement(text).assigned_target().is_err());
    }
    assert_eq!(
        Placement::Pending(original).assigned_target().unwrap(),
        None
    );
}
