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
    let original = format!(
        "# frozen source comment\n{}\n",
        std::fs::read_to_string(request.seed).unwrap()
    );
    assert_eq!(
        seed_projection(&supplied, &original, None).unwrap(),
        original
    );
    let coordinator = supplied
        .canisters
        .iter()
        .find(|entry| entry.kind == DesiredCanisterKind::Coordinator)
        .unwrap();
    let id = candid::Principal::from_text(coordinator.principal.as_ref().unwrap()).unwrap();
    assert_eq!(
        seed_projection(&supplied, &original, Some(id)).unwrap(),
        original
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
    let creating = std::fs::read_to_string(&path).unwrap();
    let projected = seed_projection(&created, &creating, Some(id)).unwrap();
    assert_ne!(projected, creating);
    let physical: EstateSeed = toml::from_str(&projected).unwrap();
    assert_eq!(physical.coordinator, id.to_text());
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
