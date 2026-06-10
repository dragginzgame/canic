use crate::{dto::error::Error, ids::CanisterRole};

///
/// ProtectedInternalEndpoint
///
/// Generated metadata for one protected Canic internal endpoint.
///
/// Endpoint macros emit this descriptor next to protected internal endpoints.
/// In 0.65, the outbound protected-internal client surface is removed, but the
/// descriptor remains the retained source of method names and accepted-role
/// metadata for verifier/rejection paths and future replacement work.
///

#[derive(Clone, Debug)]
pub struct ProtectedInternalEndpoint {
    method: &'static str,
    accepted_roles: Vec<CanisterRole>,
}

impl ProtectedInternalEndpoint {
    #[must_use]
    #[track_caller]
    pub fn new(method: &'static str, roles: impl IntoIterator<Item = CanisterRole>) -> Self {
        assert!(
            !method.trim().is_empty(),
            "protected internal endpoint descriptor method must not be empty"
        );
        let accepted_roles = roles.into_iter().collect::<Vec<_>>();
        assert!(
            !accepted_roles.is_empty(),
            "protected internal endpoint descriptor '{method}' must accept at least one caller role"
        );
        for (index, role) in accepted_roles.iter().enumerate() {
            assert!(
                !role.as_str().trim().is_empty(),
                "protected internal endpoint descriptor '{method}' has an empty caller role at index {index}"
            );
            assert!(
                !accepted_roles[..index].iter().any(|prior| prior == role),
                "protected internal endpoint descriptor '{method}' contains duplicate caller role '{role}'"
            );
        }
        Self {
            method,
            accepted_roles,
        }
    }

    #[must_use]
    pub const fn method(&self) -> &'static str {
        self.method
    }

    #[must_use]
    pub fn accepted_roles(&self) -> &[CanisterRole] {
        &self.accepted_roles
    }

    #[must_use]
    pub fn accepted_roles_label(&self) -> String {
        self.accepted_roles
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }

    #[must_use]
    pub fn accepts_role(&self, role: &CanisterRole) -> bool {
        self.accepted_roles.iter().any(|accepted| accepted == role)
    }

    #[must_use]
    pub fn single_role(&self) -> Option<&CanisterRole> {
        match self.accepted_roles.as_slice() {
            [role] => Some(role),
            _ => None,
        }
    }

    pub fn required_single_role(&self) -> Result<CanisterRole, Error> {
        self.single_role().cloned().ok_or_else(|| {
            Error::invalid(format!(
                "protected internal endpoint '{}' accepts {} roles [{}]; choose a caller role explicitly",
                self.method(),
                self.accepted_roles.len(),
                self.accepted_roles_label()
            ))
        })
    }
}
