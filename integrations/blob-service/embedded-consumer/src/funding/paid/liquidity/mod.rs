//! Installed full-offer liquidity refusals retain authoritative call costs and holds.

use super::{CashierTopUpStatus, FundingDispatchResult, FundingPreparationResult, payment};
use crate::funding::{dispatch, funding, preparation};

#[canic::canic_raw_update(public, payload(max_bytes = 0))]
async fn test_funding_liquidity() {
    for operation in 3..=5 {
        let mut input = payment(operation);
        input.offered = 9_900.try_into().unwrap();
        assert!(matches!(
            funding::prepare(input, || preparation(input, true)).unwrap(),
            FundingPreparationResult::Prepared(_)
        ));
        let FundingDispatchResult::Settled {
            observation,
            call_cost,
        } = funding::dispatch(input, || {
            let mut evidence = dispatch(input, true);
            let holds = evidence.holds.as_mut().unwrap();
            let liquid = ic_platform::ic0::canister_liquid_cycle_balance128();
            match operation {
                3 => holds.operating_reserve = liquid.try_into().unwrap(),
                4 => holds.other_liabilities = u128::MAX,
                5 => {
                    // Apparent room for the full attachment before call costs and
                    // final journal-write charges must not authorize dispatch.
                    holds.other_liabilities = liquid
                        .checked_sub(holds.operating_reserve.get())
                        .unwrap()
                        .checked_sub(input.offered.get())
                        .unwrap();
                }
                _ => unreachable!(),
            }
            evidence
        })
        .await
        .unwrap()
        else {
            panic!("complete evidence reaches exact platform liquidity admission")
        };
        assert!(call_cost > 0);
        assert_eq!(observation.status(), CashierTopUpStatus::NotDispatched);
        assert!(ic_platform::cdk::api::canister_cycle_balance() > 100_000_000);
        assert!(matches!(
            funding::dispatch(input, || dispatch(input, true))
                .await
                .unwrap(),
            FundingDispatchResult::Blocked { .. }
        ));
    }
    ic_platform::cdk::api::msg_reply([]);
}

#[canic::canic_raw_update(public, payload(max_bytes = 0))]
async fn test_funding_liquidity_replay() {
    for operation in 3..=5 {
        let mut input = payment(operation);
        input.offered = 9_900.try_into().unwrap();
        assert!(matches!(
            funding::dispatch(input, || dispatch(input, true))
                .await
                .unwrap(),
            FundingDispatchResult::Blocked { .. }
        ));
    }
    ic_platform::cdk::api::msg_reply([]);
}
