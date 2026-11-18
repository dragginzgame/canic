use super::*;
use ic_db::Entity as _;

#[ic_db::enum_(
    variant(name = "End"),
    variant(name = "Next", value(item(indirect, is = "Self")))
)]
struct Link {}
#[ic_db::entity(
    pk(field = "id"),
    index(fields = ["next", "id"]),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "link", value(item(is = "Link"))),
        field(name = "next", value(opt, item(prim = "Nat64")), references(entity = "Self"))
    )
)]
struct Row {}

#[test]
fn recursive_types_stay_finite_and_count_requires_ordinary_admission() {
    let schema = Schema::new(&[Row::SCHEMA]).unwrap();
    let request = Request::Entity {
        schema: schema.fingerprint(),
        name: name("Row").unwrap(),
        count: true,
    };
    let reply = inspect(
        &schema,
        &request,
        Admission::Maintenance,
        Cardinality::Available(99),
    )
    .unwrap();
    let Selection::Entity(entity) = reply.selection else {
        panic!("entity");
    };
    assert_eq!(entity.cardinality, Cardinality::Unavailable);
    let next = entity
        .fields
        .as_slice()
        .iter()
        .find(|f| f.name.as_str() == "next")
        .unwrap();
    assert_eq!(next.value.kind, Kind::Unsigned(64));
    assert!(next.value.optional);
    assert_eq!(
        next.reference,
        Some(Reference {
            target: name("Row").unwrap(),
            index: name("next,id").unwrap()
        })
    );
    assert_eq!(entity.types.as_slice().len(), 1);
    let Body::Enum(variants) = &entity.types.as_slice()[0].body else {
        panic!("enum");
    };
    assert_eq!(
        variants.as_slice()[1].payload.as_ref().unwrap().kind,
        Kind::Named(name("Link").unwrap())
    );
    let wrong = Request::Entity {
        schema: [0; 32],
        name: name("Row").unwrap(),
        count: true,
    };
    assert_eq!(
        inspect(
            &schema,
            &wrong,
            Admission::Unavailable,
            Cardinality::Unavailable
        ),
        Err(Refusal::SchemaChanged)
    );
}

#[test]
fn bounded_names_and_unknown_entities_refuse_instead_of_truncating() {
    assert_eq!(name(&"x".repeat(65)), Err(Refusal::Limit));
    assert_eq!(
        list::<_, 64>(vec![name("x").unwrap(); 65]),
        Err(Refusal::Limit)
    );
    let schema = Schema::new(&[Row::SCHEMA]).unwrap();
    let unknown = Request::Entity {
        schema: schema.fingerprint(),
        name: name("Missing").unwrap(),
        count: false,
    };
    assert_eq!(
        inspect(
            &schema,
            &unknown,
            Admission::Unavailable,
            Cardinality::NotExposed
        ),
        Err(Refusal::UnknownEntity)
    );
}

#[test]
fn named_definition_capacity_counts_unique_names_including_cycles() {
    let mut projection = Projection {
        reached: BTreeMap::new(),
    };
    for id in 0..64 {
        let name = Box::leak(format!("Type{id}").into_boxed_str());
        let definition = Box::leak(Box::new(TypeSchema {
            name,
            body: TypeBody::Enum(&[]),
        }));
        projection.reached.insert(name, definition);
    }
    fn another() -> &'static TypeSchema {
        static VALUE: TypeSchema = TypeSchema {
            name: "Another",
            body: TypeBody::Enum(&[]),
        };
        &VALUE
    }
    let value = FieldType::Named(ic_db::TypeLink {
        name: "Another",
        definition: another,
    });
    assert_eq!(projection.kind(value), Err(Refusal::Limit));
    assert_eq!(projection.reached.len(), 64);
    projection.reached.remove("Type0");
    assert_eq!(
        projection.kind(value),
        Ok(Kind::Named(name("Another").unwrap()))
    );
    assert_eq!(
        projection.kind(value),
        Ok(Kind::Named(name("Another").unwrap()))
    );
    assert_eq!(projection.reached.len(), 64);
}

#[test]
fn inspection_v1_refuses_unrepresented_wrapped_types() {
    let mut projection = Projection {
        reached: BTreeMap::new(),
    };
    assert_eq!(
        projection.body(&TypeBody::Wrapped(FieldType::Bool)),
        Err(Refusal::Unsupported)
    );
}
