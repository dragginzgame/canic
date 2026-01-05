use crate::{dto::env::EnvView, workflow};

///
/// EnvApi
///

pub struct EnvApi;

impl EnvApi {
    #[must_use]
    pub fn view() -> EnvView {
        workflow::env::query::EnvQuery::view()
    }
}
