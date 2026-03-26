///
/// IsPrimeSubnet
///

#[derive(Clone, Copy, Debug)]
pub struct IsPrimeSubnet;

///
/// IsPrimeRoot
///

#[derive(Clone, Copy, Debug)]
pub struct IsPrimeRoot;

///
/// BuildIcOnly
///

#[derive(Clone, Copy, Debug)]
pub struct BuildIcOnly;

///
/// BuildLocalOnly
///

#[derive(Clone, Copy, Debug)]
pub struct BuildLocalOnly;

#[must_use]
pub const fn is_prime_subnet() -> IsPrimeSubnet {
    IsPrimeSubnet
}

#[must_use]
pub const fn is_prime_root() -> IsPrimeRoot {
    IsPrimeRoot
}

#[must_use]
pub const fn build_ic_only() -> BuildIcOnly {
    BuildIcOnly
}

#[must_use]
pub const fn build_local_only() -> BuildLocalOnly {
    BuildLocalOnly
}
