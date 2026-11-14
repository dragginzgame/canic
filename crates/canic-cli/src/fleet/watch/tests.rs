use super::{Page, refresh_while_serving, report};
use canic_host::{Error, Fleet};

#[tokio::test(flavor = "current_thread")]
async fn observation_failure_returns_to_the_page_instead_of_stopping_the_viewer() {
    let viewer = crate::view::Viewer::new(0, false, "watch recovery test").unwrap();
    let mut page = Page::new("<head></head><main>prior observation</main>".into());
    let result = refresh_while_serving(&viewer, &page.current, async {
        Err(Error::Blocked("provider unavailable".into()))
    })
    .await
    .unwrap();
    assert!(result.is_err());
    page.update(result);
    assert!(page.current.contains("Refresh failed"));
    assert!(page.current.contains("prior observation"));
    let result = refresh_while_serving(&viewer, &page.current, async {
        Ok("<head></head><main>recovered observation</main>".into())
    })
    .await
    .unwrap();
    page.update(result);
    assert!(!page.current.contains("Refresh failed"));
    assert!(page.current.contains("recovered observation"));
}

#[test]
fn failed_refresh_preserves_last_report_and_recovery_clears_warning() {
    let first = "<html><head></head><body><main>original observed data</main></body></html>";
    let mut page = Page::new(first.into());
    assert!(page.current.contains("http-equiv=refresh content=30"));
    page.update(Err(Error::Invalid(
        "<script>provider failure</script>".into(),
    )));
    assert!(page.current.contains("Refresh failed"));
    assert!(page.current.contains("last completed snapshot"));
    assert!(page.current.contains("original observed data"));
    assert!(
        page.current
            .contains("&lt;script&gt;provider failure&lt;/script&gt;")
    );
    assert!(!page.current.contains("<script>provider"));
    assert_eq!(page.last, first);
    page.update(Ok(
        first.replace("original observed data", "new observed data")
    ));
    assert!(!page.current.contains("Refresh failed"));
    assert!(!page.current.contains("original observed data"));
    assert!(page.current.contains("new observed data"));
    assert_eq!(page.revision, 3);
    assert_eq!(page.current.matches("http-equiv=refresh").count(), 1);
    assert_eq!(page.current.matches("id=watch-status").count(), 1);
}

#[test]
fn oversized_refresh_cannot_replace_the_current_page() {
    let snapshot = Fleet {
        started_at_ns: 0,
        finished_at_ns: 0,
        scope: "local journal",
        canisters: vec![],
        operations: vec![],
        managers: vec![],
        wallet: None,
        issues: vec!["&".repeat(crate::view::MAX_REPORT_BYTES)],
    };
    let mut page = Page::new("<head></head><main>last bounded result</main>".into());
    page.update(report(&snapshot, &[]));
    assert!(page.current.contains("last bounded result"));
    assert!(page.current.contains("Refresh failed"));
    assert!(page.current.len() < crate::view::MAX_REPORT_BYTES);
}
