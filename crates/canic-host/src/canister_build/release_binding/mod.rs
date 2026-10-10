//! Bind a release identity into one active Wasm data segment before finalization.
//!
//! Only the reserved fixed-width slot may change. Cargo outputs remain templates;
//! callers bind a private copy before optimization, hashing and publication.

#[cfg(test)]
mod tests;

use canic_contracts::ids::ReleaseBuildId;
use canic_core::bootstrap::release_binding::{
    RELEASE_BINDING_BYTES, RELEASE_BINDING_ID_BYTES, RELEASE_BINDING_PREFIX, RELEASE_BINDING_SUFFIX,
};
use std::ops::Range;

/// A template cannot be bound unless it has exactly one valid unbound data slot.
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum ReleaseBindingError {
    #[error("invalid or unsupported Wasm data layout for release binding")]
    InvalidWasm,
    #[error("Wasm has no active release binding slot")]
    MissingSlot,
    #[error("Wasm has multiple release binding slots")]
    MultipleSlots,
    #[error("Wasm release binding slot is already bound")]
    AlreadyBound,
}

/// Fill one unbound template slot without changing code, offsets or file length.
///
/// All validation finishes before mutation; an error leaves the input unchanged.
/// A bound artifact cannot be rebound, including to the same identity. Retries
/// start from the retained template or reuse the already qualified final artifact.
pub fn bind_release_build_id(
    wasm: &mut [u8],
    identity: ReleaseBuildId,
) -> Result<(), ReleaseBindingError> {
    let slot = find_slot(wasm)?;
    if !wasm[slot.clone()].iter().all(|byte| *byte == b'?') {
        return Err(ReleaseBindingError::AlreadyBound);
    }
    wasm[slot].copy_from_slice(identity.to_string().as_bytes());
    Ok(())
}

fn find_slot(wasm: &[u8]) -> Result<Range<usize>, ReleaseBindingError> {
    if wasm.get(..8) != Some(b"\0asm\x01\0\0\0") {
        return Err(ReleaseBindingError::InvalidWasm);
    }
    let mut cursor = 8;
    let mut found = None;
    let mut data_seen = false;
    while cursor < wasm.len() {
        let section = byte(wasm, &mut cursor)?;
        let size = length(wasm, &mut cursor)?;
        let end = cursor
            .checked_add(size)
            .filter(|end| *end <= wasm.len())
            .ok_or(ReleaseBindingError::InvalidWasm)?;
        if section == 11 {
            if data_seen {
                return Err(ReleaseBindingError::InvalidWasm);
            }
            data_seen = true;
            found = data_slot(&wasm[..end], &mut cursor)?;
            if cursor != end {
                return Err(ReleaseBindingError::InvalidWasm);
            }
        }
        cursor = end;
    }
    found.ok_or(ReleaseBindingError::MissingSlot)
}

fn data_slot(
    payload: &[u8],
    cursor: &mut usize,
) -> Result<Option<Range<usize>>, ReleaseBindingError> {
    let count = length(payload, cursor)?;
    let mut found = None;
    let mut memory_ranges = Vec::<Range<u64>>::new();
    for _ in 0..count {
        let offset = match length(payload, cursor)? {
            0 => Some(offset_expression(payload, cursor)?),
            1 => None, // Passive data cannot establish the runtime binding.
            2 => {
                if length(payload, cursor)? != 0 {
                    return Err(ReleaseBindingError::InvalidWasm);
                }
                Some(offset_expression(payload, cursor)?)
            }
            _ => return Err(ReleaseBindingError::InvalidWasm),
        };
        let size = length(payload, cursor)?;
        let end = cursor
            .checked_add(size)
            .filter(|end| *end <= payload.len())
            .ok_or(ReleaseBindingError::InvalidWasm)?;
        if let Some(offset) = offset {
            let start = u64::from(offset);
            let range = start..start + size as u64;
            if range.end > 1 << 32
                || memory_ranges
                    .iter()
                    .any(|other| range.start.max(other.start) < range.end.min(other.end))
            {
                // Overlapping initializers could overwrite the binding at load time.
                return Err(ReleaseBindingError::InvalidWasm);
            }
            memory_ranges.push(range);
            for slot in segment_slots(&payload[*cursor..end]) {
                let slot = slot.start + *cursor..slot.end + *cursor;
                if found.replace(slot).is_some() {
                    return Err(ReleaseBindingError::MultipleSlots);
                }
            }
        }
        *cursor = end;
    }
    Ok(found)
}

fn segment_slots(data: &[u8]) -> impl Iterator<Item = Range<usize>> + '_ {
    data.windows(RELEASE_BINDING_BYTES)
        .enumerate()
        .filter_map(|(offset, window)| {
            // The prefix alone also occurs as a runtime validation constant.
            if window.starts_with(RELEASE_BINDING_PREFIX)
                && window.ends_with(RELEASE_BINDING_SUFFIX)
            {
                let start = offset + RELEASE_BINDING_PREFIX.len();
                Some(start..start + RELEASE_BINDING_ID_BYTES)
            } else {
                None
            }
        })
}

fn byte(bytes: &[u8], cursor: &mut usize) -> Result<u8, ReleaseBindingError> {
    let value = *bytes.get(*cursor).ok_or(ReleaseBindingError::InvalidWasm)?;
    *cursor += 1;
    Ok(value)
}

fn length(bytes: &[u8], cursor: &mut usize) -> Result<usize, ReleaseBindingError> {
    let mut value = 0_u32;
    for shift in (0..35).step_by(7) {
        let next = byte(bytes, cursor)?;
        if shift == 28 && next > 0x0f {
            return Err(ReleaseBindingError::InvalidWasm);
        }
        value |= u32::from(next & 0x7f) << shift;
        if next & 0x80 == 0 {
            return usize::try_from(value).map_err(|_| ReleaseBindingError::InvalidWasm);
        }
    }
    Err(ReleaseBindingError::InvalidWasm)
}

fn offset_expression(bytes: &[u8], cursor: &mut usize) -> Result<u32, ReleaseBindingError> {
    if byte(bytes, cursor)? != 0x41 {
        // i32.const in the maintained Rust wasm32 layout.
        return Err(ReleaseBindingError::InvalidWasm);
    }
    let mut value = 0_u32;
    for index in 0..5 {
        let next = byte(bytes, cursor)?;
        if index == 4 && !(next <= 0x07 || (0x78..=0x7f).contains(&next)) {
            return Err(ReleaseBindingError::InvalidWasm);
        }
        let shift = index * 7;
        value |= u32::from(next & 0x7f) << shift;
        if next & 0x80 == 0 {
            if shift < 28 && next & 0x40 != 0 {
                value |= u32::MAX << (shift + 7);
            }
            return if byte(bytes, cursor)? == 0x0b {
                Ok(value)
            } else {
                Err(ReleaseBindingError::InvalidWasm)
            };
        }
    }
    Err(ReleaseBindingError::InvalidWasm)
}
