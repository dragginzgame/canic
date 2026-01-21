#![allow(non_upper_case_globals)]

#[derive(Clone, Copy, Debug)]
pub struct AppAllowsUpdates;

#[derive(Clone, Copy, Debug)]
pub struct AppIsQueryable;

pub const app_allows_updates: AppAllowsUpdates = AppAllowsUpdates;
pub const app_is_queryable: AppIsQueryable = AppIsQueryable;
