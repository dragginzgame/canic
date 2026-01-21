#[derive(Clone, Copy, Debug)]
pub struct AllowsUpdates;

#[derive(Clone, Copy, Debug)]
pub struct IsQueryable;

#[must_use]
pub const fn allows_updates() -> AllowsUpdates {
    AllowsUpdates
}
#[must_use]
pub const fn is_queryable() -> IsQueryable {
    IsQueryable
}
