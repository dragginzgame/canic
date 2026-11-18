//! Bounded lookups and atomic updates of application-owned provisioning rows.
use super::{Account, Binding, HubRoute, Id, Record, Request, ShardRecord, Ticket};
use ic_db::Database;

pub(super) fn find<E: Record>(db: &Database, account: Account) -> Option<E> {
    db.query::<E>()
        .using_index(
            "issuer,subject",
            &[account.issuer.into(), account.subject.into()],
        )
        .limit(1)
        .work_limit(1)
        .byte_limit(1024)
        .fetch()
        .expect("lookup provisioned account")
        .rows
        .pop()
}

fn insert<E: Record>(db: &mut Database, row: &E) {
    let mut batch = ic_db::Batch::new();
    batch.insert(row).expect("insert provisioned account");
    db.commit(batch).expect("commit provisioned account");
}

pub(super) fn ticket<E: HubRoute>(row: &E) -> Ticket<E> {
    let (account, id) = row.identity();
    let (shard, data) = row.retained();
    Request {
        binding: Binding { id, account, shard },
        data,
    }
}

pub(super) fn reserve<E: HubRoute>(
    db: &mut Database,
    binding: &Binding<Id<E>>,
    initial: E::Initial,
) -> E {
    find::<E>(db, binding.account).unwrap_or_else(|| {
        let row = E::pending(binding.clone(), initial);
        assert!(
            ticket(&row).binding == *binding && !row.ready(),
            "invalid pending row mapping"
        );
        insert(db, &row);
        row
    })
}

pub(super) fn confirm<E: HubRoute>(db: &mut Database, expected: &Binding<Id<E>>) {
    let mut row = find::<E>(db, expected.account).expect("missing retained reservation");
    assert!(
        ticket(&row).binding == *expected,
        "changed provisioning reservation"
    );
    if !row.ready() {
        row.mark_ready();
        assert!(
            row.ready() && ticket(&row).binding == *expected,
            "invalid ready row mapping"
        );
        let mut batch = ic_db::Batch::new();
        batch.replace(&row).expect("confirm provisioned account");
        db.commit(batch).expect("commit confirmation");
    }
}

pub(super) fn accept<E: ShardRecord>(db: &mut Database, request: Ticket<E>) -> bool {
    let binding = request.binding.clone();
    let matches = |row: &E| {
        let (account, id) = row.identity();
        account == binding.account && id == binding.id
    };
    if let Some(row) = find::<E>(db, binding.account) {
        return matches(&row);
    }
    let row = E::create(request);
    if !matches(&row) {
        return false;
    }
    insert(db, &row);
    true
}
