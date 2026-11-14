use super::{CYCLES_LEDGER, ICP_LEDGER, amount, report};
use crate::{Error, wallet_address};
use ic_platform::candid_codec::{self, Nat};

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
fn balances_keep_integer_precision_and_unknown_is_not_zero() {
    let bytes = candid_codec::encode_one(&Nat::from(510_925_125u64), 1024).unwrap();
    let icp = report(ICP_LEDGER, Ok(bytes), 8, "ICP", 42);
    assert_eq!(icp.amount.as_deref(), Some("510925125"));
    assert_eq!(icp.display.as_deref(), Some("5.10925125 ICP"));
    let bytes = candid_codec::encode_one(&Nat::from(405_555_115_186u64), 1024).unwrap();
    assert_eq!(
        amount(&bytes, 12, "T cycles").unwrap().1,
        "0.405555115186 T cycles"
    );
    let bytes = candid_codec::encode_one(&Nat::from(u128::MAX), 1024).unwrap();
    assert_eq!(amount(&bytes, 8, "ICP").unwrap().0, u128::MAX.to_string());
    assert!(amount(&[0; 1025], 8, "ICP").is_err());
    assert!(amount(b"invalid reply", 8, "ICP").is_err());
    let oversized: Nat = "340282366920938463463374607431768211456".parse().unwrap();
    let bytes = candid_codec::encode_one(&oversized, 1024).unwrap();
    assert!(amount(&bytes, 8, "ICP").is_err());
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
    let zero = candid_codec::encode_one(&Nat::from(0u64), 1024).unwrap();
    assert_eq!(
        report(CYCLES_LEDGER, Ok(zero), 12, "T cycles", 44)
            .display
            .as_deref(),
        Some("0.000000000000 T cycles")
    );
}
