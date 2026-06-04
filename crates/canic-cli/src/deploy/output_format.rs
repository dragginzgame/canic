///
/// AuthorityOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AuthorityOutputFormat {
    Json,
    Text,
}

///
/// CheckOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CheckOutputFormat {
    Json,
    EnvelopeJson,
}

///
/// CatalogOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CatalogOutputFormat {
    Text,
    Json,
}

///
/// ExternalOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ExternalOutputFormat {
    Json,
    Text,
}

///
/// PromotionOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PromotionOutputFormat {
    Json,
    Text,
}

///
/// CompareOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CompareOutputFormat {
    Json,
    Text,
}

///
/// RootOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RootOutputFormat {
    Json,
    Text,
}

pub(super) fn parse_promotion_output_format(value: &str) -> Result<PromotionOutputFormat, String> {
    parse_json_text_output_format(
        value,
        "promotion",
        PromotionOutputFormat::Json,
        PromotionOutputFormat::Text,
    )
}

pub(super) fn parse_check_output_format(value: &str) -> Result<CheckOutputFormat, String> {
    match value {
        "json" => Ok(CheckOutputFormat::Json),
        "envelope-json" => Ok(CheckOutputFormat::EnvelopeJson),
        other => invalid_output_format("deployment check", other),
    }
}

pub(super) fn parse_catalog_output_format(value: &str) -> Result<CatalogOutputFormat, String> {
    parse_json_text_output_format(
        value,
        "deployment catalog",
        CatalogOutputFormat::Json,
        CatalogOutputFormat::Text,
    )
}

pub(super) fn parse_authority_output_format(value: &str) -> Result<AuthorityOutputFormat, String> {
    parse_json_text_output_format(
        value,
        "authority",
        AuthorityOutputFormat::Json,
        AuthorityOutputFormat::Text,
    )
}

pub(super) fn parse_external_output_format(value: &str) -> Result<ExternalOutputFormat, String> {
    parse_json_text_output_format(
        value,
        "external lifecycle",
        ExternalOutputFormat::Json,
        ExternalOutputFormat::Text,
    )
}

pub(super) fn parse_compare_output_format(value: &str) -> Result<CompareOutputFormat, String> {
    parse_json_text_output_format(
        value,
        "deployment comparison",
        CompareOutputFormat::Json,
        CompareOutputFormat::Text,
    )
}

pub(super) fn parse_root_output_format(value: &str) -> Result<RootOutputFormat, String> {
    parse_json_text_output_format(
        value,
        "deployment root",
        RootOutputFormat::Json,
        RootOutputFormat::Text,
    )
}

fn parse_json_text_output_format<T>(
    value: &str,
    context: &str,
    json: T,
    text: T,
) -> Result<T, String> {
    match value {
        "json" => Ok(json),
        "text" => Ok(text),
        other => invalid_output_format(context, other),
    }
}

fn invalid_output_format<T>(context: &str, value: &str) -> Result<T, String> {
    Err(format!("invalid {context} output format: {value}"))
}
