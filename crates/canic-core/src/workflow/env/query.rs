use crate::{dto::env::EnvView, ops::runtime::env::EnvOps, workflow::env::data_to_view};

///
/// EnvQuery
///

pub struct EnvQuery;

impl EnvQuery {
    #[must_use]
    pub fn view() -> EnvView {
        let data = EnvOps::snapshot();
        data_to_view(data)
    }
}
