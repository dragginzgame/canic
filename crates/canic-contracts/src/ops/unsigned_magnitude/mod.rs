//! Lossless external Nat conversion using Wire's documented unsigned byte convention.
//! Field bounds precede allocation; received nonminimal magnitudes are rejected.
use ic_platform::candid::contract::Nat;
use ic_wire::{Bytes, Error};
use num_bigint::BigUint;

/// Project arbitrary external evidence without narrowing it to a machine integer.
pub fn from_nat<const N: usize>(value: &Nat) -> Result<Bytes<N>, Error> {
    let bytes_required = value.0.bits().div_ceil(8);
    if bytes_required > u64::try_from(N).map_err(|_| Error::Limit)? {
        return Err(Error::Limit);
    }
    Bytes::new(if bytes_required == 0 {
        Vec::new()
    } else {
        value.0.to_bytes_be()
    })
}

/// Validate numeric canonicality before allocating the external arithmetic value.
pub fn to_nat<const N: usize>(value: &Bytes<N>) -> Result<Nat, Error> {
    if value.as_slice().first() == Some(&0) {
        return Err(Error::Malformed);
    }
    Ok(Nat(BigUint::from_bytes_be(value.as_slice())))
}

#[cfg(test)]
mod tests;
