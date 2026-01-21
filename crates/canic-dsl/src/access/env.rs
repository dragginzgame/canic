#[derive(Clone, Copy, Debug)]
pub struct IsPrimeSubnet;

#[derive(Clone, Copy, Debug)]
pub struct IsPrimeRoot;

#[derive(Clone, Copy, Debug)]
pub struct BuildIcOnly;

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
