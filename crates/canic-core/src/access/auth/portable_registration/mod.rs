//! Synchronous endpoint admission for issuers with retained Root governance approval.

use crate::{
    access::{
        AccessError,
        expr::{AccessCheck, AccessContext},
    },
    ops::auth::enrollment::registration,
};

/// Admit only an actual IC caller named by retained controller-approved intent.
pub struct ApprovedPortableIssuerPredicate;

impl AccessCheck for ApprovedPortableIssuerPredicate {
    fn eval(&self, ctx: &AccessContext) -> Result<(), AccessError> {
        let domain =
            registration::current_domain().map_err(|_| AccessError::PortableIssuerRequired)?;
        registration::authorize_issuer(domain, ctx.transport_caller().into())
            .map_err(|_| AccessError::PortableIssuerRequired)
    }

    fn name(&self) -> &'static str {
        "caller_is_approved_portable_issuer"
    }
}
