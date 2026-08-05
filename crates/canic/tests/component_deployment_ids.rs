use canic::{
    api::canister::deployment::ComponentDeploymentApi,
    dto::component_deployment::ProtectedComponentDeployment,
    ids::{
        ComponentDeploymentConfigurationDigest, ComponentGroupDeploymentId, ComponentGroupMemberId,
        ComponentGroupMemberPath, ComponentGroupPlacementId, ComponentGroupSpecId, FleetServiceId,
    },
};

#[test]
fn facade_exposes_bounded_component_deployment_identities() {
    let deployment = "project_data_cells"
        .parse::<ComponentGroupDeploymentId>()
        .expect("Component Group deployment ID");
    let path = ComponentGroupMemberPath::try_from(vec![
        "databases"
            .parse::<ComponentGroupMemberId>()
            .expect("included group member"),
        "database_a"
            .parse::<ComponentGroupMemberId>()
            .expect("Component member"),
    ])
    .expect("Component Group member path");
    let placement = ComponentGroupPlacementId {
        deployment: deployment.clone(),
        ordinal: 3,
    };

    assert_eq!(
        "project_data_cell"
            .parse::<ComponentGroupSpecId>()
            .expect("Component Group Spec ID")
            .as_str(),
        "project_data_cell"
    );
    assert_eq!(
        "database_a"
            .parse::<FleetServiceId>()
            .expect("Fleet Service ID")
            .as_str(),
        "database_a"
    );
    assert_eq!(placement.deployment, deployment);
    assert_eq!(placement.ordinal, 3);
    assert_eq!(path.len(), 2);
}

#[test]
fn facade_exposes_protected_component_deployment_policy() {
    let current: fn() -> Result<ProtectedComponentDeployment, canic::Error> =
        ComponentDeploymentApi::current;
    let digest = ComponentDeploymentConfigurationDigest::from_bytes([7; 32]);

    assert_eq!(digest.as_bytes(), &[7; 32]);
    let _ = current;
}
