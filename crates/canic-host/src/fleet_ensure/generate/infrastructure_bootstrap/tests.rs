//! Explicit bootstrap compiles artifacts without calling a stopped or empty Root.

use super::*;
use crate::fleet_ensure::{
    generate::{load_toml, validate_identity_seed},
    model::DesiredCanisterKind,
};

pub(in crate::fleet_ensure::generate) fn qualify(request: &FleetGenerateRequest<'_>) {
    let supplied =
        generate_infrastructure_bootstrap(request, BootstrapCoordinatorSelection::Initialize, None)
            .unwrap();
    assert!(
        supplied
            .canisters
            .iter()
            .all(|entry| entry.principal.is_some())
    );
    assert!(!request.root.join("root-status-count").exists());
    assert!(!supplied.bootstrap.as_ref().unwrap().fresh_estate);
    assert!(matches!(
        generate_infrastructure_bootstrap(request, BootstrapCoordinatorSelection::Create, None),
        Err(FleetGenerateError::SeedTopology(_))
    ));
    let mut seed: EstateSeed = load_toml(request.seed, "seed").unwrap();
    seed.coordinator = "create".into();
    let path = request.root.join("explicit-bootstrap-create.toml");
    std::fs::write(&path, toml::to_string_pretty(&seed).unwrap()).unwrap();
    let create_request = FleetGenerateRequest {
        seed: &path,
        ..*request
    };
    let created = generate_infrastructure_bootstrap(
        &create_request,
        BootstrapCoordinatorSelection::Create,
        None,
    )
    .unwrap();
    assert!(
        created
            .canisters
            .iter()
            .all(|entry| entry.principal.is_none()
                == (entry.kind == DesiredCanisterKind::Coordinator))
    );
    let source: FleetSource = load_toml(request.source, "source").unwrap();
    assert!(validate_identity_seed(&source, &seed).is_err());
    assert!(matches!(
        generate_infrastructure_bootstrap(
            &create_request,
            BootstrapCoordinatorSelection::Ready,
            None
        ),
        Err(FleetGenerateError::SeedTopology(_))
    ));
    std::fs::remove_file(path).unwrap();
}
