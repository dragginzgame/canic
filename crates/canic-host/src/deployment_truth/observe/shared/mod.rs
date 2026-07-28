use super::super::*;

pub(super) fn observation_gap(
    key: impl Into<String>,
    description: impl Into<String>,
) -> DeploymentObservationGapV1 {
    DeploymentObservationGapV1 {
        key: key.into(),
        description: description.into(),
    }
}
