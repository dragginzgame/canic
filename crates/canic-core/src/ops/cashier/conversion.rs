//! Module: ops::cashier::conversion
//!
//! Responsibility: convert Cashier DTO responses into bounded internal values.
//! Does not own: inter-canister calls, billing policy, or stable storage writes.
//! Boundary: workflow calls these helpers before applying Cashier-derived facts.

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::{candid::Int, types::Principal},
    dto::blob_storage::{BlobStorageCashierAccountCycleBalances, BlobStorageCashierDebtTarget},
};
use thiserror::Error as ThisError;

///
/// CashierCycleBalances
///
/// Internal unsigned representation of a decoded Cashier balance record.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CashierCycleBalances {
    pub total: u128,
    pub cycles_prepaid: u128,
    pub cycles_promo: u128,
    pub debt_target: BlobStorageCashierDebtTarget,
    pub cycles_ledger: u128,
}

///
/// CashierDecodeError
///
/// Typed failure for malformed Cashier responses.
///

#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum CashierDecodeError {
    #[error("Cashier cycle balance field `{field}` is negative or exceeds u128")]
    InvalidCycleBalance { field: &'static str },

    #[error("Cashier gateway principal list contains invalid principal {principal}")]
    InvalidGatewayPrincipal { principal: Principal },

    #[error("Cashier gateway principal list has {actual} principals, maximum is {max}")]
    TooManyGatewayPrincipals { actual: usize, max: usize },
}

impl From<CashierDecodeError> for InternalError {
    fn from(err: CashierDecodeError) -> Self {
        Self::ops(InternalErrorOrigin::Ops, err.to_string())
    }
}

///
/// CashierConversionOps
///
/// Zero-cost namespace for Cashier response conversion.
///

pub struct CashierConversionOps;

impl CashierConversionOps {
    pub fn account_cycle_balances_to_u128(
        balances: &BlobStorageCashierAccountCycleBalances,
    ) -> Result<CashierCycleBalances, CashierDecodeError> {
        Ok(CashierCycleBalances {
            total: Self::int_to_u128("total", &balances.total)?,
            cycles_prepaid: Self::int_to_u128("cycles_prepaid", &balances.cycles_prepaid)?,
            cycles_promo: Self::int_to_u128("cycles_promo", &balances.cycles_promo)?,
            debt_target: balances.debt_target.clone(),
            cycles_ledger: Self::int_to_u128("cycles_ledger", &balances.cycles_ledger)?,
        })
    }

    pub fn int_to_u128(field: &'static str, value: &Int) -> Result<u128, CashierDecodeError> {
        u128::try_from(value.0.clone())
            .map_err(|_| CashierDecodeError::InvalidCycleBalance { field })
    }

    pub fn normalize_gateway_principals(
        principals: Vec<Principal>,
        max: usize,
    ) -> Result<Vec<Principal>, CashierDecodeError> {
        let mut normalized = Vec::new();

        for principal in principals {
            if principal == Principal::anonymous() || principal == Principal::management_canister()
            {
                return Err(CashierDecodeError::InvalidGatewayPrincipal { principal });
            }
            if !normalized.contains(&principal) {
                normalized.push(principal);
            }
            if normalized.len() > max {
                return Err(CashierDecodeError::TooManyGatewayPrincipals {
                    actual: normalized.len(),
                    max,
                });
            }
        }

        Ok(normalized)
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn cashier_int(value: &str) -> Int {
        Int::parse(value.as_bytes()).expect("valid test integer")
    }

    fn principal(byte: u8) -> Principal {
        Principal::from_slice(&[byte])
    }

    fn balances(total: &str) -> BlobStorageCashierAccountCycleBalances {
        BlobStorageCashierAccountCycleBalances {
            total: cashier_int(total),
            cycles_prepaid: cashier_int("1"),
            cycles_promo: cashier_int("2"),
            debt_target: BlobStorageCashierDebtTarget::Prepaid,
            cycles_ledger: cashier_int("3"),
        }
    }

    #[test]
    fn int_to_u128_accepts_maximum_u128() {
        let value = cashier_int("340282366920938463463374607431768211455");

        assert_eq!(
            CashierConversionOps::int_to_u128("total", &value),
            Ok(u128::MAX)
        );
    }

    #[test]
    fn int_to_u128_rejects_negative_and_oversized_values() {
        let negative = cashier_int("-1");
        let oversized = cashier_int("340282366920938463463374607431768211456");

        assert_eq!(
            CashierConversionOps::int_to_u128("total", &negative),
            Err(CashierDecodeError::InvalidCycleBalance { field: "total" })
        );
        assert_eq!(
            CashierConversionOps::int_to_u128("total", &oversized),
            Err(CashierDecodeError::InvalidCycleBalance { field: "total" })
        );
    }

    #[test]
    fn account_cycle_balances_convert_all_signed_fields() {
        let converted = CashierConversionOps::account_cycle_balances_to_u128(&balances("10"))
            .expect("balances convert");

        assert_eq!(
            converted,
            CashierCycleBalances {
                total: 10,
                cycles_prepaid: 1,
                cycles_promo: 2,
                debt_target: BlobStorageCashierDebtTarget::Prepaid,
                cycles_ledger: 3,
            }
        );
    }

    #[test]
    fn account_cycle_balances_reject_malformed_signed_fields() {
        assert_eq!(
            CashierConversionOps::account_cycle_balances_to_u128(&balances("-1")),
            Err(CashierDecodeError::InvalidCycleBalance { field: "total" })
        );
    }

    #[test]
    fn gateway_principal_normalization_deduplicates_in_order() {
        let first = principal(1);
        let second = principal(2);

        assert_eq!(
            CashierConversionOps::normalize_gateway_principals(
                vec![first, second, first, second],
                4
            ),
            Ok(vec![first, second])
        );
    }

    #[test]
    fn gateway_principal_normalization_rejects_invalid_principals() {
        assert_eq!(
            CashierConversionOps::normalize_gateway_principals(vec![Principal::anonymous()], 4),
            Err(CashierDecodeError::InvalidGatewayPrincipal {
                principal: Principal::anonymous()
            })
        );
        assert_eq!(
            CashierConversionOps::normalize_gateway_principals(
                vec![Principal::management_canister()],
                4
            ),
            Err(CashierDecodeError::InvalidGatewayPrincipal {
                principal: Principal::management_canister()
            })
        );
    }

    #[test]
    fn gateway_principal_normalization_enforces_unique_maximum() {
        assert_eq!(
            CashierConversionOps::normalize_gateway_principals(vec![principal(1), principal(2)], 1),
            Err(CashierDecodeError::TooManyGatewayPrincipals { actual: 2, max: 1 })
        );
    }
}
