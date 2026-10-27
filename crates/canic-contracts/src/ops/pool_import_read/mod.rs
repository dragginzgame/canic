//! Lossless import review conversion; canonical authority and stored plans are unchanged.
use crate::dto::{
    pool_import::{PoolImportBootstrap, PoolImportContext},
    pool_import_read::{Bootstrap, Context},
};
impl From<PoolImportBootstrap> for Bootstrap {
    fn from(v: PoolImportBootstrap) -> Self {
        Self {
            review_sha256: v.review_sha256,
            operator: v.operator.into(),
            sources: v.sources.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<Bootstrap> for PoolImportBootstrap {
    fn from(v: Bootstrap) -> Self {
        Self {
            review_sha256: v.review_sha256,
            operator: v.operator.into(),
            sources: v.sources.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<PoolImportContext> for Context {
    fn from(v: PoolImportContext) -> Self {
        Self {
            maximum_call_debit_cycles: v.maximum_call_debit_cycles,
            bootstrap: v.bootstrap.map(Into::into),
            binding: v.binding.into(),
            root_authority_sha256: v.root_authority_sha256,
            next_sequence: v.next_sequence,
            active_import: v.active_import,
        }
    }
}
impl From<Context> for PoolImportContext {
    fn from(v: Context) -> Self {
        Self {
            maximum_call_debit_cycles: v.maximum_call_debit_cycles,
            bootstrap: v.bootstrap.map(Into::into),
            binding: v.binding.into(),
            root_authority_sha256: v.root_authority_sha256,
            next_sequence: v.next_sequence,
            active_import: v.active_import,
        }
    }
}
