//! Bounded fixture checkpoint; no general storage engine or schema migration.

use super::{controller, creation, funding};
use crate::{EXECUTABLE, FUNDING_STATUS, LIMITS, STATUS, UPGRADE};
use ic_platform_runtime::{self as runtime, raw as stable, system as api};

const FUNDING_OFFSET: u64 = LIMITS.bytes as u64 + 4;

#[runtime::pre_upgrade(runtime = runtime, guard = lifecycle_guard, decode = lifecycle_decode, max_bytes = 0, on_error = endpoint_error)]
fn pre_upgrade((): ()) {
    let creation = ic_wire::encode(
        &creation::status(()),
        ic_wire::Encoding::Frame {
            method: STATUS,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .expect("bounded creation checkpoint");
    let funding = ic_wire::encode(
        &funding::funding_status(()),
        ic_wire::Encoding::Frame {
            method: FUNDING_STATUS,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .expect("bounded funding checkpoint");
    // One 64 KiB page holds two 4 KiB frames and their lengths. This fixture
    // exclusively owns stable memory; no application data shares this layout.
    if ic_platform_runtime::raw::stable64_size() == 0 {
        stable::stable64_grow(1).expect("checkpoint page allocation");
    }
    write_frame(0, &creation);
    write_frame(FUNDING_OFFSET, &funding);
}

fn write_frame(offset: u64, bytes: &[u8]) {
    let length = u32::try_from(bytes.len()).expect("bounded checkpoint length");
    stable::stable64_write(offset, &length.to_le_bytes());
    stable::stable64_write(offset + 4, bytes);
}

fn decode_upgrade(bytes: &[u8]) -> Result<bool, std::convert::Infallible> {
    Ok({
        assert!(bytes.len() <= LIMITS.bytes, "upgrade argument too large");
        ic_wire::decode(
            bytes,
            ic_wire::Encoding::Frame {
                method: UPGRADE,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .expect("valid upgrade argument")
    })
}

#[runtime::post_upgrade(runtime = runtime, guard = lifecycle_guard, decode = decode_upgrade, max_bytes = 2097152, on_error = endpoint_error)]
fn post_upgrade(trap_after_restore: bool) {
    assert!(
        ic_platform_runtime::raw::stable64_size() > 0,
        "maintenance checkpoint missing"
    );
    let creation = ic_wire::decode(
        &read_frame(0),
        ic_wire::Encoding::Frame {
            method: STATUS,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .expect("valid creation checkpoint");
    let funding = ic_wire::decode(
        &read_frame(FUNDING_OFFSET),
        ic_wire::Encoding::Frame {
            method: FUNDING_STATUS,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .expect("valid funding checkpoint");
    creation::restore(creation);
    funding::restore(funding);
    if trap_after_restore {
        // Check rollback of stable writes as well as the new executable/heap.
        stable::stable64_write(0, &[0; 4]);
        api::trap("injected maintenance trap after restore");
    }
}

fn read_frame(offset: u64) -> Vec<u8> {
    let mut length = [0; 4];
    stable::stable64_read(offset, &mut length);
    let length = u32::from_le_bytes(length) as usize;
    assert!(
        length > 0 && length <= LIMITS.bytes,
        "invalid checkpoint length"
    );
    let mut bytes = vec![0; length];
    stable::stable64_read(offset + 4, &mut bytes);
    bytes
}

fn decode_executable(bytes: &[u8]) -> Result<(), ic_wire::Error> {
    ic_wire::decode::<()>(
        bytes,
        ic_wire::Encoding::Frame {
            method: EXECUTABLE,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}

fn encode_executable(value: bool) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: EXECUTABLE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}

#[runtime::query(runtime = runtime, guard = controller, decode = decode_executable, max_bytes = 2097152, on_error = endpoint_error, name = "executable", encode = encode_executable)]
fn executable((): ()) -> bool {
    cfg!(feature = "maintenance-candidate")
}

fn endpoint_error(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}

fn lifecycle_guard() -> Result<(), std::convert::Infallible> {
    Ok(())
}

fn lifecycle_decode(_bytes: &[u8]) -> Result<(), std::convert::Infallible> {
    Ok(())
}
