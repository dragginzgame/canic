use super::{CYCLES_LEDGER, ICP_LEDGER, amount, report};
use crate::{Error, tests::wire_fixture, wallet_address};

#[test]
fn default_receive_address_matches_the_icp_cli() {
    // Independent ICP CLI 1.7.0 derivation for the deployed Canic controller.
    assert_eq!(
        wallet_address("o5trf-oqyg7-cawjp-xs4pw-aomb3-iwki5-hyezf-qahfz-j3ffd-jh4fc-oqe").unwrap(),
        "b38bdabcc30b47b6dda966a2a7c49e4540514ce6a36c78aa42424492a98c59d3"
    );
    for invalid in ["canic-mainnet", "2vxsx-fae", "aaaaa-aa", "not-a-principal"] {
        assert!(wallet_address(invalid).is_err());
    }
}

#[test]
fn balance_request_keeps_the_existing_account_encoding() {
    let owner = "o5trf-oqyg7-cawjp-xs4pw-aomb3-iwki5-hyezf-qahfz-j3ffd-jh4fc-oqe";
    let account = super::account(owner).unwrap();
    let bytes = ic_platform_protocol::ledger::encode_balance_account(&account, 256).unwrap();
    assert_eq!(bytes, wire_fixture("balance-account"));
}

#[test]
fn balances_keep_integer_precision_and_unknown_is_not_zero() {
    let bytes = wire_fixture("balance-icp");
    let icp = report(ICP_LEDGER, Ok(bytes), 8, "ICP", 42);
    assert_eq!(icp.amount.as_deref(), Some("510925125"));
    assert_eq!(icp.display.as_deref(), Some("5.10925125 ICP"));
    let bytes = wire_fixture("balance-cycles");
    assert_eq!(
        amount(&bytes, 12, "T cycles").unwrap().1,
        "0.405555115186 T cycles"
    );
    let bytes = wire_fixture("balance-max");
    assert_eq!(amount(&bytes, 8, "ICP").unwrap().0, u128::MAX.to_string());
    assert!(amount(&[0; 1025], 8, "ICP").is_err());
    assert!(amount(b"invalid reply", 8, "ICP").is_err());
    let mut trailing = bytes;
    trailing.push(0);
    assert!(amount(&trailing, 8, "ICP").is_err());
    let wrong_type = wire_fixture("balance-wrong-type");
    assert!(amount(&wrong_type, 8, "ICP").is_err());
    let bytes = wire_fixture("balance-overflow");
    assert!(matches!(amount(&bytes, 8, "ICP"), Err(Error::Invalid(_))));
    let failed = report(
        CYCLES_LEDGER,
        Err(Error::Blocked("provider unavailable".into())),
        12,
        "T cycles",
        43,
    );
    assert!(failed.amount.is_none());
    assert!(failed.display.is_none());
    assert!(
        failed
            .observation_error
            .unwrap()
            .contains("provider unavailable")
    );
    let zero = wire_fixture("balance-zero");
    assert_eq!(
        report(CYCLES_LEDGER, Ok(zero), 12, "T cycles", 44)
            .display
            .as_deref(),
        Some("0.000000000000 T cycles")
    );
}
