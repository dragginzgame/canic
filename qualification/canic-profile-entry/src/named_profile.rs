//! Qualification-only Toko-shaped methods with an intentionally small reply budget.
use canic_profile_protocol::{LIMITS, Profile};
use ic_platform_runtime::system;
use std::cell::RefCell;

const REQUEST: ic_wire::Limits = ic_wire::Limits {
    bytes: 128,
    ..LIMITS
};
const REPLY: ic_wire::Limits = ic_wire::Limits {
    bytes: 72,
    ..LIMITS
};
thread_local! { static RECORD: RefCell<Option<Profile>> = const { RefCell::new(None) }; }

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

#[canic::query(method = 20, guard = member)]
fn get_me() -> Option<Profile> {
    ops::get()
}
#[canic::update(method = 21, limits = REQUEST, reply_limits = REPLY, guard = member)]
fn set_display_name(name: String) -> Profile {
    ops::set(name)
}

// A new public endpoint can use every default. Use the same hash in its client.
#[canic::query]
fn protocol_version() -> u8 {
    1
}
