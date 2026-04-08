use std::{io::Write, ops::Deref};

use super::*;

pub struct PicBorrow<'a>(pub &'a Pic);

impl Deref for PicBorrow<'_> {
    type Target = Pic;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

// Emit one short progress marker for long grouped PocketIC scenario tests.
pub fn test_progress(test_name: &str, phase: &str) {
    eprintln!("[pic_role_attestation] {test_name}: {phase}");
    let _ = std::io::stderr().flush();
}

pub fn update_call_as<T, A>(
    pic: &Pic,
    canister_id: Principal,
    caller: Principal,
    method: &str,
    args: A,
) -> T
where
    T: candid::CandidType + DeserializeOwned,
    A: ArgumentEncoder,
{
    pic.update_call_as(canister_id, caller, method, args)
        .expect("update_call failed")
}

pub fn update_call_raw_as<A>(
    pic: &Pic,
    canister_id: Principal,
    caller: Principal,
    method: &str,
    args: A,
) -> Result<(), String>
where
    A: ArgumentEncoder,
{
    pic.update_call_as::<Result<(), Error>, _>(canister_id, caller, method, args)
        .map(|_| ())
        .map_err(|err| err.to_string())
}

pub fn query_call_as<T, A>(
    pic: &Pic,
    canister_id: Principal,
    caller: Principal,
    method: &str,
    args: A,
) -> T
where
    T: candid::CandidType + DeserializeOwned,
    A: ArgumentEncoder,
{
    pic.query_call_as(canister_id, caller, method, args)
        .expect("query_call failed")
}
