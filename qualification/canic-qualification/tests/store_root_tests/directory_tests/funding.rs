//! Direct service replenishment keeps one lifetime budget through retries.
use super::*;
use canic::method::Contract;
use stored::{ChildFunding, ChildFundingStatus, ConfigureChildFunding, StopChildFunding};

pub(super) async fn exercise(root: &Client<'_>, target: CanisterId, sibling: CanisterId) {
    let pic = root.pic;
    let amount = 100_000_000_000;
    let rule = ChildFunding {
        parent: root.id.to_text().parse().unwrap(),
        child: target.to_text().parse().unwrap(),
        component: "project_hub".into(),
        below_cycles: u64::MAX,
        amount_cycles: amount,
        budget_cycles: 2 * amount,
        expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 600_000_000_000,
    };
    let configure = async |r: &ChildFunding| {
        root.update::<_, Result<ChildFundingStatus, stored::FundingError>>(
            ConfigureChildFunding::NAME,
            ConfigureChildFunding::ID,
            r,
            ConfigureChildFunding::REPLY,
        )
        .await
    };
    assert!(
        pic.update_call(
            root.id,
            sibling,
            ConfigureChildFunding::NAME,
            ConfigureChildFunding::encode_request(&rule).unwrap()
        )
        .await
        .is_err()
    );
    let mut wrong = rule.clone();
    wrong.child = sibling.to_text().parse().unwrap();
    assert!(configure(&wrong).await.is_err());
    let before = pic.cycle_balance(target).await;
    let sibling_before = pic.cycle_balance(sibling).await;
    assert_eq!(
        configure(&rule).await.unwrap(),
        ChildFundingStatus::default()
    );
    for _ in 0..20 {
        pic.tick().await;
    }
    let first = configure(&rule).await.unwrap();
    assert_eq!(
        (first.attempts, first.charged_cycles, first.refund_cycles),
        (1, amount, Some(0))
    );
    assert!(pic.cycle_balance(target).await > before);
    assert!(pic.cycle_balance(sibling).await <= sibling_before);
    let mut changed = rule.clone();
    changed.budget_cycles *= 2;
    assert!(configure(&changed).await.is_err());
    for _ in 0..2 {
        pic.advance_time(Duration::from_secs(61)).await;
        for _ in 0..20 {
            pic.tick().await;
        }
    }
    let spent = configure(&rule).await.unwrap();
    assert_eq!(
        (spent.attempts, spent.charged_cycles),
        (2, rule.budget_cycles)
    );
    assert!(!spent.issued);
    canic_qualification::upgrade_stopped(
        pic,
        root.id,
        std::fs::read(artifacts().join("root.wasm")).unwrap(),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
    assert_eq!(configure(&rule).await.unwrap(), spent);
    let closed = root
        .update::<_, Result<ChildFundingStatus, stored::FundingError>>(
            StopChildFunding::NAME,
            StopChildFunding::ID,
            &rule.child,
            StopChildFunding::REPLY,
        )
        .await
        .unwrap();
    assert!(closed.closed);
    assert_eq!(configure(&rule).await.unwrap(), closed);
}

pub(super) async fn application_parent(
    root: &Client<'_>,
    parent: CanisterId,
    child: CanisterId,
    component: &str,
) {
    let pic = root.pic;
    let amount = 100_000_000_000;
    let rule = ChildFunding {
        parent: parent.to_text().parse().unwrap(),
        child: child.to_text().parse().unwrap(),
        component: component.into(),
        below_cycles: u64::try_from(pic.cycle_balance(child).await).unwrap() + 4 * amount,
        amount_cycles: amount,
        budget_cycles: amount * 3,
        expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 600_000_000_000,
    };
    let payer = Client {
        pic,
        id: parent,
        operator: root.operator,
    };
    let configure = async |r: &ChildFunding| {
        payer
            .update::<_, Result<ChildFundingStatus, stored::FundingError>>(
                ConfigureChildFunding::NAME,
                ConfigureChildFunding::ID,
                r,
                ConfigureChildFunding::REPLY,
            )
            .await
    };
    let mut wrong = rule.clone();
    wrong.child = root.id.to_text().parse().unwrap();
    assert!(configure(&wrong).await.is_err());
    let outsider = Client {
        pic,
        id: parent,
        operator: root.operator,
    };
    let request = async |client: &Client<'_>, seq: u64| {
        client
            .update::<_, Result<u64, stored::FundingError>>(
                "request_child_funding",
                ic_wire::method_id("request_child_funding"),
                &(seq, 1_u128),
                canic::method::LIMITS,
            )
            .await
    };
    assert_eq!(
        request(&outsider, 1).await,
        Err(stored::FundingError::UnknownChild)
    );
    let before = pic.cycle_balance(child).await;
    let parent_before = pic.cycle_balance(parent).await;
    assert_eq!(
        configure(&rule).await.unwrap(),
        ChildFundingStatus::default()
    );
    let unauthorized = Client {
        pic,
        id: parent,
        operator: root.id,
    };
    assert_eq!(
        unauthorized
            .update::<_, Result<ChildFundingStatus, stored::FundingError>>(
                ConfigureChildFunding::NAME,
                ConfigureChildFunding::ID,
                &rule,
                ConfigureChildFunding::REPLY
            )
            .await,
        Err(stored::FundingError::Unauthorized)
    );
    let advance = async || {
        pic.advance_time(Duration::from_secs(61)).await;
        for _ in 0..25 {
            pic.tick().await;
        }
    };
    advance().await;
    let paid = configure(&rule).await.unwrap();
    assert_eq!(
        (paid.attempts, paid.charged_cycles, paid.refund_cycles),
        (1, amount, Some(0))
    );
    assert!(
        pic.cycle_balance(child).await > before && pic.cycle_balance(parent).await < parent_before
    );
    let client = Client {
        pic,
        id: parent,
        operator: child,
    };
    assert_eq!(request(&client, 1).await, Ok(1));
    assert_eq!(configure(&rule).await.unwrap(), paid);
    let wasm = std::fs::read(artifacts().join("directory.wasm")).unwrap();
    canic_qualification::upgrade_stopped(pic, parent, wasm.clone(), vec![], Some(root.id))
        .await
        .unwrap();
    canic_qualification::upgrade_stopped(pic, child, wasm, vec![], Some(root.id))
        .await
        .unwrap();
    advance().await;
    let second = configure(&rule).await.unwrap();
    assert_eq!((second.attempts, second.charged_cycles), (2, 2 * amount));
    // External funding lifts the child's balance above its trigger.
    pic.add_cycles(child, u128::from(8 * amount)).await;
    advance().await;
    assert_eq!(configure(&rule).await.unwrap(), second);
    let closed = payer
        .update::<_, Result<ChildFundingStatus, stored::FundingError>>(
            StopChildFunding::NAME,
            StopChildFunding::ID,
            &rule.child,
            StopChildFunding::REPLY,
        )
        .await
        .unwrap();
    advance().await;
    assert!(closed.closed);
    assert_eq!(request(&client, 3).await, Err(stored::FundingError::Closed));
    assert_eq!(configure(&rule).await.unwrap(), closed);
    assert!(request(&client, 3).await.is_err());
}
