use std::fmt::Display;

///
/// Canister
///

pub trait Canister: Display {
    fn path(&self) -> String;
    fn is_sharded(&self) -> bool;
}
