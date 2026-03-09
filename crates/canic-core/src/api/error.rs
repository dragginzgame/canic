use crate::{InternalError, InternalErrorClass, InternalErrorOrigin, dto::error::Error};

fn internal_error_to_public(err: &InternalError) -> Error {
    if let Some(public) = err.public_error() {
        return public.clone();
    }

    match err.class() {
        InternalErrorClass::Access => Error::unauthorized(err.to_string()),

        InternalErrorClass::Domain => match err.origin() {
            InternalErrorOrigin::Config => Error::invalid(err.to_string()),
            _ => Error::conflict(err.to_string()),
        },

        InternalErrorClass::Invariant => Error::invariant(err.to_string()),

        InternalErrorClass::Infra | InternalErrorClass::Ops | InternalErrorClass::Workflow => {
            Error::internal(err.to_string())
        }
    }
}

impl From<&InternalError> for Error {
    fn from(err: &InternalError) -> Self {
        internal_error_to_public(err)
    }
}

impl From<InternalError> for Error {
    fn from(err: InternalError) -> Self {
        internal_error_to_public(&err)
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{access::AccessError, dto::error::ErrorCode};

    #[test]
    fn internal_error_mapping_matches_class_contract() {
        let access: Error = InternalError::from(AccessError::Denied("denied".to_string())).into();
        assert_eq!(access.code, ErrorCode::Unauthorized);

        let domain_config: Error =
            InternalError::domain(InternalErrorOrigin::Config, "bad config").into();
        assert_eq!(domain_config.code, ErrorCode::InvalidInput);

        let domain_other: Error =
            InternalError::domain(InternalErrorOrigin::Domain, "conflict").into();
        assert_eq!(domain_other.code, ErrorCode::Conflict);

        let invariant: Error =
            InternalError::invariant(InternalErrorOrigin::Ops, "broken invariant").into();
        assert_eq!(invariant.code, ErrorCode::InvariantViolation);

        let infra: Error = InternalError::infra(InternalErrorOrigin::Infra, "infra fail").into();
        assert_eq!(infra.code, ErrorCode::Internal);

        let ops: Error = InternalError::ops(InternalErrorOrigin::Ops, "ops fail").into();
        assert_eq!(ops.code, ErrorCode::Internal);

        let workflow: Error =
            InternalError::workflow(InternalErrorOrigin::Workflow, "workflow fail").into();
        assert_eq!(workflow.code, ErrorCode::Internal);
    }

    #[test]
    fn public_error_is_preserved_without_remap() {
        let public = Error::not_found("missing");
        let remapped: Error = InternalError::public(public.clone()).into();
        assert_eq!(remapped, public);
    }
}
