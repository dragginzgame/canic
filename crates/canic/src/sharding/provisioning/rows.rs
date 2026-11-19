//! Bounded lookups and atomic updates of application-owned provisioning rows.
use super::{Account, Binding, HubRoute, Id, Request, ShardRecord, Ticket};
use ic_db::{Database, Entity};

pub(super) fn find<E: Entity>(db: &Database, account: Account) -> Option<E> {
    assert!(
        E::SCHEMA.indexes.iter().any(|index| {
            index.unique && index.name == "issuer,subject" && index.fields == ["issuer", "subject"]
        }),
        "provisioning requires a unique issuer,subject index"
    );
    let row = db
        .index_prefix::<E>(
            "issuer,subject",
            &[account.issuer.into(), account.subject.into()],
            1,
        )
        .expect("lookup provisioned account")
        .pop()?;
    let encoded = row.encode().expect("encode provisioned account");
    assert!(encoded.len() <= 1024, "provisioning row exceeds 1 KiB");
    Some(row)
}

fn insert<E: Entity>(db: &mut Database, row: &E) {
    let mut batch = ic_db::Batch::new();
    batch.insert(row).expect("insert provisioned account");
    db.commit(batch).expect("commit provisioned account");
}

pub(super) fn ticket<E: HubRoute>(row: &E::Row) -> Ticket<E> {
    let (account, id) = E::identity(row);
    let (shard, data) = E::retained(row);
    Request {
        binding: Binding { id, account, shard },
        data,
    }
}

pub(super) fn reserve<E: HubRoute>(
    db: &mut Database,
    binding: &Binding<Id<E>>,
    initial: E::Initial,
) -> E::Row {
    find::<E::Row>(db, binding.account).unwrap_or_else(|| {
        let row = E::pending(binding.clone(), initial);
        assert!(
            ticket::<E>(&row).binding == *binding && !E::ready(&row),
            "invalid pending row mapping"
        );
        insert(db, &row);
        row
    })
}

pub(super) fn confirm<E: HubRoute>(db: &mut Database, expected: &Binding<Id<E>>) {
    let mut row = find::<E::Row>(db, expected.account).expect("missing retained reservation");
    assert!(
        ticket::<E>(&row).binding == *expected,
        "changed provisioning reservation"
    );
    if !E::ready(&row) {
        E::mark_ready(&mut row);
        assert!(
            E::ready(&row) && ticket::<E>(&row).binding == *expected,
            "invalid ready row mapping"
        );
        let mut batch = ic_db::Batch::new();
        batch.replace(&row).expect("confirm provisioned account");
        db.commit(batch).expect("commit confirmation");
    }
}

pub(super) fn accept<E: ShardRecord>(db: &mut Database, request: Ticket<E>) -> bool {
    let binding = request.binding.clone();
    let matches = |row: &E::Row| {
        let (account, id) = E::identity(row);
        account == binding.account && id == binding.id
    };
    // Validate the submitted payload even when acknowledging an earlier insert.
    let row = E::create(request);
    if !matches(&row) {
        return false;
    }
    if let Some(existing) = find::<E::Row>(db, binding.account) {
        return matches(&existing);
    }
    insert(db, &row);
    true
}
