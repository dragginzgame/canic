//! Registry-wide approval exercised by two paying project canisters.
use super::*;
use canic::sharding::{Placement, payment::Payment};

pub(super) async fn exercise(
    root: &Client<'_>,
    project: CanisterId,
    source: &stored::Stage<management::Approval>,
    module: &str,
) -> CanisterId {
    let pic = root.pic;
    capacity::approve_projects(root, project, source, 2).await;
    let first = capacity::ready_project(root, project, 1).await;
    let second = capacity::ready_project(root, project, 2).await;
    assert_ne!(first, second);
    let grant = stored::SingletonGrant {
        registry: project.to_text(),
        role: "project_ledger".into(),
        maximum: 2,
        creation_budget: 2 * management::CREATION_CYCLES,
        creation_payer: stored::CreationPayer::Requester,
        expires_at_ns: source.approval.expires_at_ns,
        reference: source.reference.clone(),
        application: management::application::Arguments::new(vec![]).unwrap(),
    };
    let approve = async |g: &stored::SingletonGrant| {
        root.update::<_, Result<(), management::Refusal>>(
            "configure_singleton",
            stored::CONFIGURE_SINGLETON,
            g,
            stored::LIMITS,
        )
        .await
    };
    approve(&grant).await.unwrap();
    approve(&grant).await.unwrap();
    let mut changed = grant.clone();
    changed.creation_payer = stored::CreationPayer::Root;
    assert_eq!(approve(&changed).await, Err(management::Refusal::Occupied));
    let fund = async |parent, attached: u128| -> Result<Payment, String> {
        Client {
            pic,
            id: parent,
            operator: root.id,
        }
        .update(
            "fund_ledger",
            ic_wire::method_id("fund_ledger"),
            &(2_000_000_000_000_u128, attached),
            canic::sharding::LIMITS,
        )
        .await
    };
    assert!(fund(project, 0).await.is_err());
    for parent in [first, second] {
        pic.add_cycles(parent, 4_000_000_000_000).await;
    }
    let unapproved: Result<u64, String> = Client {
        pic,
        id: project,
        operator: first,
    }
    .update(
        "request_child_funding",
        ic_wire::method_id("request_child_funding"),
        &(1u64, 1u128),
        canic::method::LIMITS,
    )
    .await;
    assert!(unapproved.is_err());
    for parent in [first, second] {
        assert!(matches!(fund(parent, 0).await.unwrap(), Payment::Quoted(_)));
    }
    let install = async |parent| {
        let Payment::Quoted(quote) = fund(parent, 0).await.unwrap() else {
            panic!("expected project quote")
        };
        let charge = quote.creation_fee + quote.working_cycles;
        let before = pic.cycle_balance(parent).await;
        let Payment::Placed(Placement::Pending(original), receipt) =
            fund(parent, charge).await.unwrap()
        else {
            panic!("expected paid reservation")
        };
        assert_eq!(receipt, Some(quote.clone()));
        assert!(pic.cycle_balance(parent).await <= before - charge);
        settle(pic).await;
        let Payment::Placed(Placement::Assigned(recovered, id), receipt) =
            fund(parent, 0).await.unwrap()
        else {
            panic!("expected retained project ledger")
        };
        assert_eq!(original, recovered);
        assert_eq!(receipt, Some(quote));
        let ledger = CanisterId::from_text(id).unwrap();
        assert_eq!(pic.get_controllers(ledger).await, [root.id]);
        ledger
    };
    let a = install(first).await;
    let b = install(second).await;
    assert_ne!(a, b);
    for (parent, own, other) in [(first, a, b), (second, b, a)] {
        let status = async |child| -> (String, Option<Option<u64>>) {
            query_service(pic, parent, "topology_status", child).await
        };
        assert_eq!(status(own).await, ("project".into(), Some(Some(2))));
        assert_eq!(status(other).await, ("project".into(), None));
    }
    canic_qualification::upgrade_stopped(
        pic,
        root.id,
        std::fs::read(artifacts().join(module)).unwrap(),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
    for (parent, ledger) in [(first, a), (second, b)] {
        let Payment::Placed(Placement::Assigned(_, id), Some(_)) = fund(parent, 0).await.unwrap()
        else {
            panic!("Root must retain the paid singleton after reopen")
        };
        assert_eq!(id, ledger.to_text());
    }
    funding::application_parent(root, first, a, "project_ledger").await;
    first
}
