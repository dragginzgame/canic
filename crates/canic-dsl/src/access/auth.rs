#[derive(Clone, Copy, Debug)]
pub struct Authenticated;

#[must_use]
pub const fn authenticated() -> Authenticated {
    Authenticated
}
