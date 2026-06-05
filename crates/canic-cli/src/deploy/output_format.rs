use clap::ValueEnum;

///
/// AuthorityOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum AuthorityOutputFormat {
    Json,
    Text,
}

///
/// CheckOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum CheckOutputFormat {
    Json,
    EnvelopeJson,
}

///
/// CatalogOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum CatalogOutputFormat {
    Text,
    Json,
}

///
/// ExternalOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum ExternalOutputFormat {
    Json,
    Text,
}

///
/// PromotionOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum PromotionOutputFormat {
    Json,
    Text,
}

///
/// CompareOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum CompareOutputFormat {
    Json,
    Text,
}

///
/// RootOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum RootOutputFormat {
    Json,
    Text,
}
