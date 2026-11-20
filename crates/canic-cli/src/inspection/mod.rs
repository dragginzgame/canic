//! Read-only Canic inspection. Applications retain query/write and exposure policy.

#[cfg(test)]
mod tests;

use canic::db::{RPC_LIMITS, inspection as protocol};
use canic_host::{Error, InspectionClient};
use clap::{Args, Subcommand};
use ic_host_text::{bordered_table, escape};
use ic_wire::{Direction, Encoding};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Subcommand)]
pub(super) enum DbCommand {
    /// Read the exposed catalogue or one entity's schema and optional count.
    Inspect(Options),
}

#[derive(Args)]
pub(super) struct Options {
    #[command(flatten)]
    network: crate::args::NetworkArgs,
    /// Explicit target canister principal.
    #[arg(long)]
    canister: String,
    /// Make the query as the anonymous principal.
    #[arg(
        long,
        required_unless_present = "identity",
        conflicts_with = "identity"
    )]
    anonymous: bool,
    /// Ed25519 or secp256k1 PEM identity.
    #[arg(long, value_name = "PEM")]
    identity: Option<PathBuf>,
    /// Whole deadline per verified query, in seconds.
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..=300))]
    timeout_secs: u64,
    /// Selected logical entity; omit for the complete exposed catalogue.
    #[arg(long)]
    entity: Option<String>,
    /// Request maintained cardinality, subject to application permission and admission.
    #[arg(long, requires = "entity")]
    count: bool,
}

fn decode(bytes: &[u8], request: &protocol::Request) -> Result<protocol::Response, String> {
    let response = ic_wire::decode::<protocol::Reply>(
        bytes,
        Encoding::Frame {
            method: protocol::INSPECT,
            direction: Direction::Response,
        },
        RPC_LIMITS,
    )
    .map_err(|e| format!("invalid Canic inspection v1 reply: {e}"))?
    .map_err(|e| format!("inspection refused: {e:?}"))?;
    match (request, &response.selection) {
        (protocol::Request::Catalogue, protocol::Selection::Catalogue(_)) => {}
        (
            protocol::Request::Entity {
                schema,
                name,
                count,
            },
            protocol::Selection::Entity(entity),
        ) if *schema == response.schema && *name == entity.name => {
            if matches!(entity.cardinality, protocol::Cardinality::Available(_))
                && (!count || !matches!(response.admission, protocol::Admission::Readable { .. }))
            {
                return Err("unexpected cardinality without readable admission and request".into());
            }
        }
        _ => {
            return Err(
                "inspection reply does not match the requested schema/entity selection".into(),
            );
        }
    }

    Ok(response)
}

async fn call(
    client: &InspectionClient,
    request: &protocol::Request,
    timeout: Duration,
    stats: &mut Vec<String>,
) -> canic_host::Result<protocol::Response> {
    let payload = ic_wire::encode(
        request,
        Encoding::Frame {
            method: protocol::INSPECT,
            direction: Direction::Request,
        },
        RPC_LIMITS,
    )
    .map_err(|e| Error::Invalid(e.to_string()))?;
    let start = Instant::now();
    let reply = client.inspect(&payload, timeout).await;
    stats.push(format!(
        "query call (db_inspect): {:.2} sec, cycles unavailable",
        start.elapsed().as_secs_f64()
    ));
    decode(&reply?, request).map_err(Error::Invalid)
}

pub(super) async fn run(options: &Options, json: bool) -> canic_host::Result<crate::Output> {
    let network = options.network.clone().into_network()?;
    let client = InspectionClient::new(&network, &options.canister, options.identity.as_deref())?;
    let name = options
        .entity
        .as_ref()
        .map(|n| {
            protocol::Name::new(n.clone())
                .map_err(|_| Error::Invalid("entity name exceeds inspection bounds".into()))
        })
        .transpose()?;
    let timeout = Duration::from_secs(options.timeout_secs);
    let mut stats = Vec::new();
    let outcome = async {
        let catalogue = call(&client, &protocol::Request::Catalogue, timeout, &mut stats).await?;
        let Some(name) = name else {
            return Ok(catalogue);
        };
        let protocol::Selection::Catalogue(names) = &catalogue.selection else {
            unreachable!()
        };
        if !names.as_slice().contains(&name) {
            return Err(Error::Invalid(
                "entity is not in the exposed catalogue".into(),
            ));
        }
        let selected = call(
            &client,
            &protocol::Request::Entity {
                schema: catalogue.schema,
                name,
                count: options.count,
            },
            timeout,
            &mut stats,
        )
        .await?;
        if selected.admission != catalogue.admission {
            return Err(Error::Invalid(
                "database admission changed during inspection; repeat the command".into(),
            ));
        }
        Ok(selected)
    }
    .await;
    if json || outcome.is_err() {
        for line in &stats {
            eprintln!("{line}");
        }
    }
    let response = outcome?;
    let provider = match &network {
        canic_host::Network::Mainnet => "https://icp-api.io/",
        canic_host::Network::Local { url, .. } => url,
    };
    if json {
        Ok(crate::Output::Json(
            serde_json::json!({"provider":provider, "canister":options.canister, "caller":client.caller().to_text(), "inspection":response}),
        ))
    } else {
        Ok(crate::Output::Text(format!(
            "APPLICATION INSPECTION\n{}\n{}{}\n",
            bordered_table(
                &["Target", "Value"],
                &[
                    vec!["Provider".into(), escape(provider).into_owned()],
                    vec!["Canister".into(), options.canister.clone()],
                    vec!["Caller".into(), client.caller().to_text()]
                ]
            ),
            render(&response),
            stats.join("\n")
        )))
    }
}

fn kind(kind: &protocol::Kind) -> (String, String) {
    use protocol::Kind as K;

    match kind {
        K::Unsigned(bits) => (format!("Nat{bits}"), String::new()),
        K::Signed(bits) => (format!("Int{bits}"), String::new()),
        K::Float(bits) => (format!("Float{bits}"), String::new()),
        K::Text(bytes) => ("Text".into(), format!("max {bytes} UTF-8 bytes")),
        K::Blob(bytes) => ("Blob".into(), format!("max {bytes} bytes")),
        K::Decimal(scale) => ("Decimal".into(), format!("scale {scale}")),
        K::Named(name) => (escape(name.as_str()).into_owned(), String::new()),
        K::List { item, max_items } => {
            let (name, info) = self::kind(item);

            (
                format!("List<{name}>"),
                format!("max {max_items} items; {info}"),
            )
        }
        K::Bool
        | K::Principal
        | K::Account
        | K::Subaccount
        | K::Ulid
        | K::Date
        | K::Timestamp
        | K::Duration => (format!("{kind:?}"), String::new()),
    }
}

fn fields(fields: &[protocol::Field]) -> String {
    bordered_table(
        &[
            "Field",
            "Type",
            "Optional",
            "Info",
            "Reference target",
            "Reference index",
        ],
        &fields
            .iter()
            .map(|f| {
                let (name, info) = kind(&f.value.kind);

                vec![
                    escape(f.name.as_str()).into_owned(),
                    name,
                    f.value.optional.to_string(),
                    info,
                    f.reference
                        .as_ref()
                        .map_or_else(String::new, |r| escape(r.target.as_str()).into_owned()),
                    f.reference
                        .as_ref()
                        .map_or_else(String::new, |r| escape(r.index.as_str()).into_owned()),
                ]
            })
            .collect::<Vec<_>>(),
    )
}

fn render(response: &protocol::Response) -> String {
    let (admission, incarnation) = match response.admission {
        protocol::Admission::Readable { incarnation } => {
            ("Readable", incarnation.map(|v| format!("{v:02x}")).concat())
        }
        protocol::Admission::Maintenance => ("Maintenance", "—".into()),
        protocol::Admission::Incompatible => ("Incompatible", "—".into()),
        protocol::Admission::Unavailable => ("Unavailable", "—".into()),
    };
    let mut out = format!(
        "{}\n",
        bordered_table(
            &["Property", "Value"],
            &[
                vec![
                    "Executable schema".into(),
                    response.schema.map(|v| format!("{v:02x}")).concat()
                ],
                vec!["Stored admission".into(), admission.into()],
                vec!["Incarnation".into(), incarnation],
            ]
        )
    );
    match &response.selection {
        protocol::Selection::Catalogue(names) => out.push_str(&bordered_table(
            &["Entity"],
            &names
                .as_slice()
                .iter()
                .map(|n| vec![escape(n.as_str()).into_owned()])
                .collect::<Vec<_>>(),
        )),
        protocol::Selection::Entity(entity) => {
            let cardinality = match entity.cardinality {
                protocol::Cardinality::Available(n) => n.to_string(),
                protocol::Cardinality::NotExposed => "Not exposed".into(),
                protocol::Cardinality::Unavailable => "Unavailable".into(),
            };
            out.push_str(&format!(
                "{}\n{}\n",
                bordered_table(
                    &["Entity", "Primary key", "Maintained count"],
                    &[vec![
                        escape(entity.name.as_str()).into_owned(),
                        escape(entity.primary_key.as_str()).into_owned(),
                        cardinality
                    ]]
                ),
                fields(entity.fields.as_slice())
            ));
            out.push_str(&bordered_table(
                &["Index", "Fields", "Unique", "Present only"],
                &entity
                    .indexes
                    .as_slice()
                    .iter()
                    .map(|i| {
                        vec![
                            escape(i.name.as_str()).into_owned(),
                            i.fields
                                .as_slice()
                                .iter()
                                .map(|f| escape(f.as_str()).into_owned())
                                .collect::<Vec<_>>()
                                .join(", "),
                            i.unique.to_string(),
                            i.present_only.to_string(),
                        ]
                    })
                    .collect::<Vec<_>>(),
            ));
            for definition in entity.types.as_slice().iter() {
                out.push_str(&format!(
                    "\n{}\n",
                    escape(definition.name.as_str()).into_owned()
                ));

                match &definition.body {
                    protocol::Body::Record(items) => out.push_str(&fields(items.as_slice())),
                    protocol::Body::Enum(items) => out.push_str(&bordered_table(
                        &["Variant", "Payload", "Optional", "Info"],
                        &items
                            .as_slice()
                            .iter()
                            .map(|v| {
                                let (name, info) = v
                                    .payload
                                    .as_ref()
                                    .map_or_else(|| ("—".into(), String::new()), |p| kind(&p.kind));

                                vec![
                                    escape(v.name.as_str()).into_owned(),
                                    name,
                                    v.payload.as_ref().is_some_and(|p| p.optional).to_string(),
                                    info,
                                ]
                            })
                            .collect::<Vec<_>>(),
                    )),
                    protocol::Body::Set { item, max_items } => {
                        let (name, info) = kind(item);
                        out.push_str(&bordered_table(
                            &["Item", "Max items", "Info"],
                            &[vec![name, max_items.to_string(), info]],
                        ));
                    }
                    protocol::Body::Map {
                        key,
                        value,
                        max_items,
                    } => {
                        let (k, ki) = kind(key);
                        let (v, vi) = kind(&value.kind);
                        out.push_str(&bordered_table(
                            &["Key", "Value", "Optional value", "Max items", "Info"],
                            &[vec![
                                k,
                                v,
                                value.optional.to_string(),
                                max_items.to_string(),
                                format!("key: {ki}; value: {vi}"),
                            ]],
                        ));
                    }
                }
            }
        }
    }
    out.push('\n');

    out
}
