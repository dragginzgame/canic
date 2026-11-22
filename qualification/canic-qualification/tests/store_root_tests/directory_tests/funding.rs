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
        root.update::<_, Result<ChildFundingStatus, String>>(
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
    let closed = root
        .update::<_, Result<ChildFundingStatus, String>>(
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
