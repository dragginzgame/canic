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
    use crate::{
        method::Contract,
        metrics::{ObserveChildMetrics, ReadMetrics, Report, Request},
    };
    let request =
        ObserveChildMetrics::encode_request(&(256, Request::Endpoints(page.next.clone()))).unwrap();
    ObserveChildMetrics::decode_request(&request).unwrap();
    let report = Report::Endpoints(page);
    let bytes = ReadMetrics::encode_reply(&report).unwrap();
    ReadMetrics::decode_reply(&bytes).unwrap();
    let bytes = ObserveChildMetrics::encode_reply(&report).unwrap();
    ObserveChildMetrics::decode_reply(&bytes).unwrap();
}
