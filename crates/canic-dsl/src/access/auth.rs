#[derive(Clone, Copy, Debug)]
pub struct Authenticated {
    pub required_scope: Option<&'static str>,
}

#[must_use]
pub const fn is_authenticated(required_scope: Option<&'static str>) -> Authenticated {
    Authenticated { required_scope }
}
