//! Qualification-only Toko-shaped methods with an intentionally small reply budget.
use canic_profile_protocol::{LIMITS, Profile};
use ic_platform_runtime::system;
use std::cell::RefCell;

const WRITE_REQUEST: ic_wire::Limits = ic_wire::Limits {
    bytes: 128,
    ..LIMITS
};
const WRITE_REPLY: ic_wire::Limits = ic_wire::Limits {
    bytes: 72,
    ..LIMITS
};

thread_local! { static RECORD: RefCell<Option<Profile>> = const { RefCell::new(None) }; }

/// Admit only the fixed member principal used by this witness.
fn member() -> Result<(), &'static str> {
    if system::caller() == ic_platform_types::Principal::from_slice(&[42, 2]) {
        Ok(())
    } else {
        Err("profile witness denied")
    }
}

mod ops {
    use super::{Profile, RECORD};

    pub(super) fn get() -> Option<Profile> {
        RECORD.with_borrow(Clone::clone)
    }

    pub(super) fn set(display_name: String) -> Profile {
        let record = Profile {
            user_id: 1,
            owner: super::system::caller().as_slice().to_vec(),
            revision: 1,
            display_name,
        };
        RECORD.replace(Some(record.clone()));
        record
    }
}

/// Return the admitted member's profile if present.
#[canic::query(
    contract = GetMe,
    guard = member,
)]
fn get_me() -> Option<Profile> {
    ops::get()
}

/// Store the admitted member's display name and return the resulting profile.
#[canic::update(
    contract = SetDisplayName,
    guard = member,
)]
fn set_display_name(name: String) -> Profile {
    ops::set(name)
}

/// Return the protocol version using the default public endpoint and Wire ID.
#[canic::query]
fn protocol_version() -> u8 {
    1
}

/// Wire contract for the `get_me` endpoint.
struct GetMe;
impl canic::method::Contract for GetMe {
    const NAME: &'static str = "get_me";
    const ID: u64 = 20;
    type Input = ();
    type Output = Option<Profile>;
}

/// Wire contract for the `set_display_name` endpoint.
struct SetDisplayName;
impl canic::method::Contract for SetDisplayName {
    const NAME: &'static str = "set_display_name";
    const ID: u64 = 21;
    const REQUEST: canic::method::wire::Limits = WRITE_REQUEST;
    const REPLY: canic::method::wire::Limits = WRITE_REPLY;
    type Input = String;
    type Output = Profile;
}
