use super::*;

// The Root reply frame and child reply frame must both fit at the page boundary.
#[test]
fn full_page_with_maximum_names_fits_both_wire_contracts() {
    let name = "x".repeat(128);
    let page = Page {
        measurements: (0..16)
            .map(|_| Measurement {
                endpoint: name.clone(),
                calls: u64::MAX,
                latest: u64::MAX,
                average: Some(u64::MAX),
                maximum: u64::MAX,
                overflowed: true,
            })
            .collect(),
        next: Some(name),
    };
    fn roundtrip<C: Contract<Output = Page>>(page: &Page) {
        assert_eq!(
            &C::decode_reply(&C::encode_reply(page).unwrap()).unwrap(),
            page
        );
    }
    EndpointMetrics::decode_request(&EndpointMetrics::encode_request(&page.next).unwrap()).unwrap();
    ChildMetrics::decode_request(&ChildMetrics::encode_request(&(0, page.next.clone())).unwrap())
        .unwrap();
    roundtrip::<EndpointMetrics>(&page);
    roundtrip::<ChildMetrics>(&page);
}
