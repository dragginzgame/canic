//! Reserve readable completed-result shapes before intent and again before signing.
//!
//! Sizing copies never enter storage or responses. Auth owns canonical claims;
//! nonces and Ed25519 signature bytes have fixed-width native encodings.

use super::{E, claims, wire_result};
use crate::{
    model::auth::portable_issuer::PortableIssuerJournalRecord,
    ops::auth::portable_issuer::encode_journal,
};

pub(super) fn admit_completions(journal: &PortableIssuerJournalRecord) -> Result<(), E> {
    let mut projected = journal.clone();
    for row in &mut projected.issuances {
        if row.result.is_some() {
            continue;
        }
        let index = usize::try_from(row.key.sequence)
            .ok()
            .and_then(|index| index.checked_sub(1))
            .ok_or(E::Storage)?;
        let public_key = journal
            .keys
            .get(index)
            .and_then(|key| key.public_key)
            .ok_or(E::Storage)?;
        row.challenge.get_or_insert([1; 32]);
        let encoded =
            ic_auth::portable::encode_claims(&claims(row, public_key)?).map_err(|_| E::Invalid)?;
        row.result = Some(wire_result(row, encoded, vec![0; 64])?);
    }
    encode_journal(&projected).map(|_| ()).map_err(Into::into)
}
