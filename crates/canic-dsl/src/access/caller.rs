#[derive(Clone, Copy, Debug)]
pub struct IsController;

#[derive(Clone, Copy, Debug)]
pub struct IsParent;

#[derive(Clone, Copy, Debug)]
pub struct IsChild;

#[derive(Clone, Copy, Debug)]
pub struct IsRoot;

#[derive(Clone, Copy, Debug)]
pub struct IsSameCanister;

#[derive(Clone, Copy, Debug)]
pub struct IsRegisteredToSubnet;

#[derive(Clone, Copy, Debug)]
pub struct IsWhitelisted;

#[must_use]
pub const fn is_controller() -> IsController {
    IsController
}

#[must_use]
pub const fn is_parent() -> IsParent {
    IsParent
}

#[must_use]
pub const fn is_child() -> IsChild {
    IsChild
}

#[must_use]
pub const fn is_root() -> IsRoot {
    IsRoot
}

#[must_use]
pub const fn is_same_canister() -> IsSameCanister {
    IsSameCanister
}

#[must_use]
pub const fn is_registered_to_subnet() -> IsRegisteredToSubnet {
    IsRegisteredToSubnet
}

#[must_use]
pub const fn is_whitelisted() -> IsWhitelisted {
    IsWhitelisted
}
