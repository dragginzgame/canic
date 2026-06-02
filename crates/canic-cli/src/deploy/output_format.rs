use super::DeployCommandError;

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

pub(super) fn parse_promotion_output_format(
    value: Option<&str>,
    usage: fn() -> String,
) -> Result<PromotionOutputFormat, DeployCommandError> {
    parse_json_text_output_format(
        value,
        "json",
        "promotion",
        PromotionOutputFormat::Json,
        PromotionOutputFormat::Text,
        usage,
    )
}

pub(super) fn parse_check_output_format(
    value: Option<&str>,
    usage: fn() -> String,
) -> Result<CheckOutputFormat, DeployCommandError> {
    match value.unwrap_or("json") {
        "json" => Ok(CheckOutputFormat::Json),
        "envelope-json" => Ok(CheckOutputFormat::EnvelopeJson),
        other => invalid_output_format("deployment check", other, usage),
    }
}

pub(super) fn parse_catalog_output_format(
    value: Option<&str>,
    usage: fn() -> String,
) -> Result<CatalogOutputFormat, DeployCommandError> {
    parse_json_text_output_format(
        value,
        "text",
        "deployment catalog",
        CatalogOutputFormat::Json,
        CatalogOutputFormat::Text,
        usage,
    )
}

pub(super) fn parse_authority_output_format(
    value: Option<&str>,
    usage: fn() -> String,
) -> Result<AuthorityOutputFormat, DeployCommandError> {
    parse_json_text_output_format(
        value,
        "json",
        "authority",
        AuthorityOutputFormat::Json,
        AuthorityOutputFormat::Text,
        usage,
    )
}

pub(super) fn parse_external_output_format(
    value: Option<&str>,
    usage: fn() -> String,
) -> Result<ExternalOutputFormat, DeployCommandError> {
    parse_json_text_output_format(
        value,
        "json",
        "external lifecycle",
        ExternalOutputFormat::Json,
        ExternalOutputFormat::Text,
        usage,
    )
}

pub(super) fn parse_compare_output_format(
    value: Option<&str>,
    usage: fn() -> String,
) -> Result<CompareOutputFormat, DeployCommandError> {
    parse_json_text_output_format(
        value,
        "json",
        "deployment comparison",
        CompareOutputFormat::Json,
        CompareOutputFormat::Text,
        usage,
    )
}

pub(super) fn parse_root_output_format(
    value: Option<&str>,
    usage: fn() -> String,
) -> Result<RootOutputFormat, DeployCommandError> {
    parse_json_text_output_format(
        value,
        "json",
        "deployment root",
        RootOutputFormat::Json,
        RootOutputFormat::Text,
        usage,
    )
}

fn parse_json_text_output_format<T>(
    value: Option<&str>,
    default: &str,
    context: &str,
    json: T,
    text: T,
    usage: fn() -> String,
) -> Result<T, DeployCommandError> {
    debug_assert!(matches!(default, "json" | "text"));
    match value.unwrap_or(default) {
        "json" => Ok(json),
        "text" => Ok(text),
        other => invalid_output_format(context, other, usage),
    }
}

fn invalid_output_format<T>(
    context: &str,
    value: &str,
    usage: fn() -> String,
) -> Result<T, DeployCommandError> {
    Err(DeployCommandError::Usage(format!(
        "invalid {context} output format: {value}\n\n{}",
        usage()
    )))
}
