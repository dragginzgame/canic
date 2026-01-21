#[derive(Clone, Copy, Debug)]
pub struct DelegatedTokenValid;

#[must_use]
pub const fn delegated_token_valid() -> DelegatedTokenValid {
    DelegatedTokenValid
}
