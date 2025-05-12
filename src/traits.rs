///
/// Canister
///

pub trait Canister: ToString {
    fn path(&self) -> String;
    fn is_sharded(&self) -> bool;
}
