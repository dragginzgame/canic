use base32::Alphabet;
use candid::{CandidType, Principal};
use crc32fast::Hasher as Crc32Hasher;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fmt::{self, Display, Write},
    hash::{Hash, Hasher},
    str::FromStr,
};

//
// Subaccount
//

pub type Subaccount = [u8; 32];

pub const DEFAULT_SUBACCOUNT: &Subaccount = &[0; 32];

//
// Account
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Account {
    pub owner: Principal,
    pub subaccount: Option<Subaccount>,
}

impl Account {
    // Construct one account from an owner and optional subaccount payload.
    pub fn new<P: Into<Principal>, S: Into<Subaccount>>(owner: P, subaccount: Option<S>) -> Self {
        Self {
            owner: owner.into(),
            subaccount: subaccount.map(Into::into),
        }
    }

    // The effective subaccount of an account is the configured subaccount or
    // the all-zero default when none is present.
    #[must_use]
    pub fn effective_subaccount(&self) -> &Subaccount {
        self.subaccount.as_ref().unwrap_or(DEFAULT_SUBACCOUNT)
    }
}

impl Display for Account {
    // Render the canonical ICRC account text form with checksum-bearing subaccounts.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.subaccount {
            None => Display::fmt(&self.owner, f),
            Some(subaccount) if subaccount == DEFAULT_SUBACCOUNT => Display::fmt(&self.owner, f),
            Some(subaccount) => write!(
                f,
                "{}-{}.{}",
                self.owner,
                full_account_checksum(self.owner.as_slice(), subaccount),
                encode_trimmed_hex(subaccount),
            ),
        }
    }
}

impl Eq for Account {}

impl FromStr for Account {
    type Err = String;

    // Parse the canonical ICRC account text form into owner and subaccount fields.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_once('.') {
            Some((principal_checksum, subaccount)) => {
                let (principal, checksum) = match principal_checksum.rsplit_once('-') {
                    Some((_, checksum)) if checksum.len() != 7 => {
                        return Err("missing checksum".to_string());
                    }
                    Some(parts) => parts,
                    None => return Err("missing checksum".to_string()),
                };

                if subaccount.starts_with('0') {
                    return Err("subaccount should not have leading zeroes".to_string());
                }

                let owner = Principal::from_str(principal)
                    .map_err(|err| format!("invalid principal: {err}"))?;
                let subaccount = decode_subaccount(subaccount)?;

                if &subaccount == DEFAULT_SUBACCOUNT {
                    return Err("default subaccount should be omitted".to_string());
                }

                let expected_checksum = full_account_checksum(owner.as_slice(), &subaccount);
                if checksum != expected_checksum {
                    return Err(format!("invalid checksum (expected: {expected_checksum})"));
                }

                Ok(Self {
                    owner,
                    subaccount: Some(subaccount),
                })
            }
            None => Principal::from_str(s)
                .map(Self::from)
                .map_err(|err| format!("invalid principal: {err}")),
        }
    }
}

impl PartialEq for Account {
    // Compare accounts by owner and effective subaccount semantics.
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner && self.effective_subaccount() == other.effective_subaccount()
    }
}

impl From<Principal> for Account {
    // Promote one principal into its default account representation.
    fn from(owner: Principal) -> Self {
        Self {
            owner,
            subaccount: None,
        }
    }
}

impl Hash for Account {
    // Hash the owner plus effective subaccount so omitted defaults stay equivalent.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.owner.hash(state);
        self.effective_subaccount().hash(state);
    }
}

impl Ord for Account {
    // Order accounts by owner first, then by effective subaccount bytes.
    fn cmp(&self, other: &Self) -> Ordering {
        self.owner.cmp(&other.owner).then_with(|| {
            self.effective_subaccount()
                .cmp(other.effective_subaccount())
        })
    }
}

impl PartialOrd for Account {
    // Delegate partial ordering to the total ordering implementation.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Compute the textual checksum over owner and subaccount bytes.
fn full_account_checksum(owner: &[u8], subaccount: &[u8]) -> String {
    let mut hasher = Crc32Hasher::new();
    hasher.update(owner);
    hasher.update(subaccount);

    base32::encode(
        Alphabet::Rfc4648Lower { padding: false },
        &hasher.finalize().to_be_bytes(),
    )
}

// Encode one subaccount as lowercase hex without leading zeroes.
fn encode_trimmed_hex(subaccount: &Subaccount) -> String {
    let mut encoded = String::with_capacity(64);
    for &byte in subaccount {
        let _ = write!(encoded, "{byte:02x}");
    }

    encoded.trim_start_matches('0').to_string()
}

// Decode one possibly trimmed lowercase or uppercase hex subaccount string.
fn decode_subaccount(encoded: &str) -> Result<Subaccount, String> {
    if encoded.len() > 64 {
        return Err("invalid subaccount: subaccount is longer than 32 bytes".to_string());
    }

    let padded = format!("{encoded:0>64}");
    let mut out = [0_u8; 32];

    for (index, chunk) in padded.as_bytes().chunks_exact(2).enumerate() {
        out[index] = decode_hex_byte(chunk)
            .ok_or_else(|| "invalid subaccount: subaccount is not hex-encoded".to_string())?;
    }

    Ok(out)
}

// Decode one ASCII hex byte pair into its binary representation.
fn decode_hex_byte(pair: &[u8]) -> Option<u8> {
    let high = decode_hex_nibble(pair.first().copied()?)?;
    let low = decode_hex_nibble(pair.get(1).copied()?)?;
    Some((high << 4) | low)
}

// Decode one ASCII hex nibble into its numeric value.
const fn decode_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::Account;
    use candid::Principal;
    use std::str::FromStr;

    #[test]
    // Default accounts should serialize as the principal only.
    fn account_display_omits_default_subaccount() {
        let owner = Principal::anonymous();
        let account = Account::from(owner);

        assert_eq!(account.to_string(), owner.to_string());
    }

    #[test]
    // Non-default subaccounts should trim leading zeroes in text form.
    fn account_display_trims_subaccount_hex() {
        let owner =
            Principal::from_text("k2t6j-2nvnp-4zjm3-25dtz-6xhaa-c7boj-5gayf-oj3xs-i43lp-teztq-6ae")
                .unwrap();
        let subaccount = Some([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1,
        ]);
        let account = Account { owner, subaccount };

        assert_eq!(
            account.to_string(),
            "k2t6j-2nvnp-4zjm3-25dtz-6xhaa-c7boj-5gayf-oj3xs-i43lp-teztq-6ae-6cc627i.1"
        );
    }

    #[test]
    // Bare principals should parse into default accounts.
    fn account_from_str_accepts_principal_only() {
        let text = "k2t6j-2nvnp-4zjm3-25dtz-6xhaa-c7boj-5gayf-oj3xs-i43lp-teztq-6ae";

        assert_eq!(
            Account::from_str(text),
            Ok(Account::from(Principal::from_str(text).unwrap()))
        );
    }

    #[test]
    // Canonical text parsing should reject subaccounts with leading zeroes.
    fn account_from_str_rejects_leading_zeroes() {
        let text = "k2t6j-2nvnp-4zjm3-25dtz-6xhaa-c7boj-5gayf-oj3xs-i43lp-teztq-6ae-6cc627i.01";

        assert_eq!(
            Account::from_str(text),
            Err("subaccount should not have leading zeroes".to_string())
        );
    }

    #[test]
    // Dot-qualified account strings must include the checksum segment.
    fn account_from_str_rejects_missing_checksum() {
        let text = "k2t6j-2nvnp-4zjm3-25dtz-6xhaa-c7boj-5gayf-oj3xs-i43lp-teztq-6ae.1";

        assert_eq!(Account::from_str(text), Err("missing checksum".to_string()));
    }

    #[test]
    // Non-default account text should round-trip through parse and display.
    fn account_from_str_round_trips_non_default_subaccount() {
        let text = "k2t6j-2nvnp-4zjm3-25dtz-6xhaa-c7boj-5gayf-oj3xs-i43lp-teztq-6ae-dfxgiyy.102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
        let owner =
            Principal::from_str("k2t6j-2nvnp-4zjm3-25dtz-6xhaa-c7boj-5gayf-oj3xs-i43lp-teztq-6ae")
                .unwrap();
        let subaccount = Some([
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
            0x1d, 0x1e, 0x1f, 0x20,
        ]);

        assert_eq!(Account::from_str(text), Ok(Account { owner, subaccount }));
        assert_eq!(Account::from_str(text).unwrap().to_string(), text);
    }
}
