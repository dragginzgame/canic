use crate::dto::prelude::*;

///
/// ValidationReport
///

#[expect(clippy::struct_excessive_bools)]
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidationReport {
    pub ok: bool,
    pub registry_directory_consistent: bool,
    pub unique_directory_roles: bool,
    pub env_complete: bool,
    pub issues: Vec<ValidationIssue>,
}

///
/// ValidationIssue
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidationIssue {
    pub code: String,
    pub message: String,
}
