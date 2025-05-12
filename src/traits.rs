///
/// Canister
///

pub trait Canister: Sized + ToString {
    fn path(&self) -> String;
    fn is_sharded(&self) -> bool;
}
