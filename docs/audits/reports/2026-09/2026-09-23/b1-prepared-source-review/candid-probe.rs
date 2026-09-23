use candid::{CandidType, Decode, Deserialize, Encode};

/// A documented named record.
#[derive(CandidType, Debug, Deserialize, PartialEq)]
#[candid_path("candid")]
struct Entry {
    /// A renamed value.
    #[serde(rename = "v")]
    value: u64,
}

/// A documented generic wrapper.
#[derive(CandidType, Debug, Deserialize, PartialEq)]
struct Page<T> {
    /// Elements of this page.
    entries: Vec<T>,
}

/// A documented enum with all supported payload shapes.
#[derive(CandidType, Debug, Deserialize, PartialEq)]
enum Choice {
    /// An empty variant.
    Empty,
    /// A tuple payload.
    Pair(Entry, u64),
    /// A named payload.
    Named {
        /// A nested page.
        page: Page<Entry>,
    },
}

fn main() {
    let enabled = match std::env::var("CANIC_AUDIT_CANDID_TYPE_DOCS").as_deref() {
        Ok("1") => true,
        Ok("0") => false,
        _ => panic!("explicit audit input required"),
    };
    for doc in [
        Entry::_ty_doc(),
        Page::<Entry>::_ty_doc(),
        Choice::_ty_doc(),
    ] {
        assert_eq!(!doc.docs.is_empty(), enabled);
        assert_eq!(!doc.fields.is_empty(), enabled);
        if !enabled {
            assert_eq!(doc, candid::types::TypeDoc::default());
        }
    }
    let values = vec![
        Choice::Empty,
        Choice::Pair(Entry { value: 7 }, 8),
        Choice::Named {
            page: Page {
                entries: vec![Entry { value: u64::MAX }],
            },
        },
    ];
    let encoded = Encode!(&values).unwrap();
    assert_eq!(Decode!(&encoded, Vec<Choice>).unwrap(), values);
    for byte in encoded {
        print!("{byte:02x}");
    }
    println!();
    println!("{}", Choice::_ty());
}
