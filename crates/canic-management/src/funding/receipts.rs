//! Settlement evidence is written with the payer's original funding state.
use canic::funding::{FundingReceipt, FundingReceipts, RETAINED_RECEIPTS};
use canic_management_protocol::stored::{ChildFunding, ChildFundingStatus};
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct History(VecDeque<(Principal, u64, u64)>);

const PREFIX: &[u8; 4] = b"FR01";
// Each row holds at most a 29-byte principal and two ten-byte varints.
const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 4096,
    allocation: 262_144,
    nodes: 8192,
    depth: 12,
};
pub(crate) const EXTRA_BYTES: usize = PREFIX.len() + 2 + LIMITS.bytes;

impl History {
    pub(crate) fn record(&mut self, child: Principal, sequence: u64, confirmed_at_ns: u64) {
        if self.0.len() == RETAINED_RECEIPTS {
            self.0.pop_front();
        }
        self.0.push_back((child, sequence, confirmed_at_ns));
    }

    /// Each child's retained rows must form a suffix of its original settled sequences.
    pub(crate) fn validate(&self, paid: impl Fn(Principal) -> Option<u64>) {
        let mut expected = BTreeMap::new();
        for &(child, sequence, _) in self.0.iter().rev() {
            let next = expected
                .entry(child)
                .or_insert_with(|| paid(child).expect("original funding grant"));
            assert!(
                sequence != 0 && sequence == *next,
                "retained funding receipt sequence"
            );
            *next -= 1;
        }
    }

    pub(crate) fn report(
        &self,
        grant: &ChildFunding,
        status: &ChildFundingStatus,
        paid: u64,
    ) -> FundingReceipts {
        let receipts: Vec<_> = self
            .0
            .iter()
            .filter(|(child, _, _)| child == &grant.child)
            .map(|&(_, sequence, confirmed_at_ns)| FundingReceipt {
                sequence,
                confirmed_at_ns,
                amount_cycles: grant.amount_cycles,
            })
            .collect();
        FundingReceipts {
            payer: grant.parent,
            child: grant.child,
            unavailable_through: paid - receipts.len() as u64,
            unresolved: status.issued.then_some(status.attempts),
            receipts,
        }
    }

    // Keep the original state codec and its entire byte budget. Legacy images
    // have no prefix. Older readers refuse the new envelope on downgrade.
    pub(crate) fn wrap(&self, state: Vec<u8>) -> Vec<u8> {
        if self.0.is_empty() {
            return state;
        }
        let history = ic_wire::encode(self, ic_wire::Encoding::Payload, LIMITS)
            .expect("bounded funding receipts");
        let mut bytes = PREFIX.to_vec();
        bytes.extend(
            u16::try_from(history.len())
                .expect("receipt byte limit")
                .to_le_bytes(),
        );
        bytes.extend(history);
        bytes.extend(state);
        bytes
    }

    pub(crate) fn split(bytes: &[u8]) -> (Self, &[u8]) {
        let Some(rest) = bytes.strip_prefix(PREFIX) else {
            return (Self::default(), bytes);
        };
        let (length, rest) = rest.split_at(2);
        let (history, state) =
            rest.split_at(usize::from(u16::from_le_bytes(length.try_into().unwrap())));
        let history: Self = ic_wire::decode(history, ic_wire::Encoding::Payload, LIMITS)
            .expect("retained funding receipts");
        assert!(history.0.len() <= RETAINED_RECEIPTS);
        (history, state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic::{funding::ObserveChildFunding, method::Contract};

    #[test]
    fn original_ledger_has_no_fabricated_receipts_and_eviction_preserves_sequence() {
        let payer = Principal::from_slice(&[1; 29]);
        let child = Principal::from_slice(&[2; 29]);
        let rule = ChildFunding {
            parent: payer,
            child,
            component: "ledger".into(),
            below_cycles: 1,
            amount_cycles: u64::MAX,
            budget_cycles: u64::MAX,
            expires_at_ns: u64::MAX,
        };
        let previous = b"PL12previous custody";
        let (mut history, bytes) = History::split(previous);
        assert_eq!(bytes, previous);
        let mut status = ChildFundingStatus {
            attempts: 4,
            issued: true,
            ..Default::default()
        };
        let old = history.report(&rule, &status, 3);
        assert!(old.receipts.is_empty());
        assert_eq!((old.unavailable_through, old.unresolved), (3, Some(4)));
        status.issued = false;
        for seq in 4..=68 {
            history.record(child, seq, u64::MAX);
        }
        let encoded = history.wrap(previous.to_vec());
        let (mut reopened, bytes) = History::split(&encoded);
        assert_eq!(bytes, previous);
        reopened.validate(|id| (id == child).then_some(68));
        let result = reopened.report(&rule, &status, 68);
        assert_eq!(result.unavailable_through, 4);
        assert_eq!(result.receipts.first().unwrap().sequence, 5);
        assert_eq!(result.receipts.last().unwrap().sequence, 68);
        assert!(ObserveChildFunding::encode_reply(&Some(result)).is_ok());
        for seq in 1..=64 {
            reopened.record(payer, seq, u64::MAX);
        }
        let empty = reopened.report(&rule, &status, 68);
        assert_eq!(empty.unavailable_through, 68);
        assert!(empty.receipts.is_empty());
    }
}
