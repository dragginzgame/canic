//! Exercise the operator review/apply entrypoint on a genuinely completed current Fleet.

use super::*;
use canic_host::{
    fleet_ensure::{
        dto::capacity_import::CapacityImportReviewRequest,
        ops::capacity_import::{
            admission::{
                CapacityImportDeclaration, CapacityImportDeclarations,
                CapacityImportDispositionKind,
            },
            publication,
        },
        workflow::capacity_import::review,
    },
    icp::IcpCli,
};

pub(super) fn qualify(input: &ReinstallJourney<'_>, desired: &DesiredFleet, icp: &IcpCli) {
    let span = Span::start("completed_fleet_capacity_import");
    let pic = input.pic;
    let operator = Principal::from_text(&desired.operator).unwrap();
    let source = pic.create_canister_on_subnet(None, None, pic.get_subnet(input.root).unwrap());
    pic.add_cycles(source, 20_000_000_000_000);
    pic.set_controllers(source, None, vec![operator]).unwrap();
    let before = pic.canister_status(source, Some(operator)).unwrap();
    let declarations = CapacityImportDeclarations {
        schema_version: 1,
        operator: operator.to_text(),
        network_root_key_sha256: sha256_hex(&pic.root_key().unwrap()),
        canisters: vec![CapacityImportDeclaration {
            canister: source.to_text(),
            subnet: pic.get_subnet(source).unwrap().to_text(),
            controllers: vec![operator.to_text()],
            module_sha256: "empty".into(),
            canister_version: before.version,
            disposition: CapacityImportDispositionKind::Absence,
            no_external_obligations: true,
            no_other_fleet_ownership: true,
            evidence: "New empty test canister, funded explicitly and owned only by the operator"
                .into(),
        }],
    };
    std::fs::write(
        input.adapter_root.join("capacity-import.toml"),
        toml::to_string(&declarations).unwrap(),
    )
    .unwrap();
    let request = CapacityImportReviewRequest {
        environment: "local".into(),
        fleet: desired.fleet.clone(),
        canisters: vec![source],
        root: Some(input.root),
        declarations: "capacity-import.toml".into(),
        policy: "fleet-policy.toml".into(),
        seed: "fleet-seed.toml".into(),
        maximum_source_debit_cycles: 1_000_000_000_000,
        maximum_root_debit_cycles: 2_000_000_000_000,
        maximum_root_paid_calls: 64,
    };
    let planned = review::plan(input.adapter_root, &request, icp)
        .expect("review supplied capacity from the completed current Fleet");
    assert!(!planned.approved);
    assert_eq!(
        pic.canister_status(source, Some(operator)).unwrap().version,
        before.version
    );
    let repeated =
        review::plan(input.adapter_root, &request, icp).expect("reopen the original saved survey");
    assert_eq!(
        repeated, planned,
        "review repeat must preserve balances and approval identity"
    );
    let digest = planned.operation.as_ref().unwrap().review.review_sha256;
    let completed = review::apply(input.adapter_root, "local", &desired.fleet, digest, icp)
        .expect("apply the exact CLI-owned review");
    assert!(publication::completed(&completed));
    let after = pic.canister_status(source, Some(input.root)).unwrap();
    assert!(after.module_hash.is_none());
    assert_eq!(after.settings.controllers, planned.plan.final_controllers);
    let paths =
        canic_host::fleet_ensure::EnsurePaths::under(input.adapter_root, "local", &desired.fleet);
    let before_journal = std::fs::read(paths.plan.with_file_name("capacity-import.json")).unwrap();
    let unavailable = IcpCli::new("/no/such/import-icp", Some("local".into()));
    assert_eq!(
        review::apply(
            input.adapter_root,
            "local",
            &desired.fleet,
            digest,
            &unavailable
        )
        .unwrap(),
        completed
    );
    assert_eq!(
        std::fs::read(paths.plan.with_file_name("capacity-import.json")).unwrap(),
        before_journal
    );
    assert_eq!(
        pic.canister_status(source, Some(input.root))
            .unwrap()
            .version,
        after.version
    );
    span.finish();
}
