#![allow(non_upper_case_globals)]

#[derive(Clone, Copy, Debug)]
pub struct CallerIsController;

#[derive(Clone, Copy, Debug)]
pub struct CallerIsParent;

#[derive(Clone, Copy, Debug)]
pub struct CallerIsChild;

#[derive(Clone, Copy, Debug)]
pub struct CallerIsRoot;

#[derive(Clone, Copy, Debug)]
pub struct CallerIsSameCanister;

#[derive(Clone, Copy, Debug)]
pub struct CallerIsRegisteredToSubnet;

#[derive(Clone, Copy, Debug)]
pub struct CallerIsWhitelisted;

pub const caller_is_controller: CallerIsController = CallerIsController;
pub const caller_is_parent: CallerIsParent = CallerIsParent;
pub const caller_is_child: CallerIsChild = CallerIsChild;
pub const caller_is_root: CallerIsRoot = CallerIsRoot;
pub const caller_is_same_canister: CallerIsSameCanister = CallerIsSameCanister;
pub const caller_is_registered_to_subnet: CallerIsRegisteredToSubnet = CallerIsRegisteredToSubnet;
pub const caller_is_whitelisted: CallerIsWhitelisted = CallerIsWhitelisted;
