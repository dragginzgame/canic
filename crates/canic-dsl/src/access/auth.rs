#[derive(Clone, Copy, Debug)]
pub struct Authenticated {
    pub required_scope: &'static str,
}

#[must_use]
pub const fn authenticated(required_scope: &'static str) -> Authenticated {
    Authenticated { required_scope }
}
