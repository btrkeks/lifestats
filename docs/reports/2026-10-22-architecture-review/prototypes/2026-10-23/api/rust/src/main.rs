use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use serde_json::{json, Value as Json};
use std::{error::Error, fs};
use utoipa::{OpenApi, ToSchema};

const SOURCE: &str = "607978998c72b1e14a95518d32fbfeafe2f7fed6";
const SAFE: i64 = 9_007_199_254_740_991;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "i64", into = "i64")]
struct SafeNumber(i64);
impl utoipa::PartialSchema for SafeNumber {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        utoipa::openapi::schema::ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::Integer)
            .minimum(Some(-SAFE)).maximum(Some(SAFE)).into()
    }
}
impl ToSchema for SafeNumber {}
impl TryFrom<i64> for SafeNumber {
    type Error = &'static str;
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if (-SAFE..=SAFE).contains(&value) { Ok(Self(value)) } else { Err("out_of_range: expected safe JSON integer") }
    }
}
impl From<SafeNumber> for i64 { fn from(v: SafeNumber) -> Self { v.0 } }

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "String", into = "String")]
#[schema(value_type = String, pattern = r"^(0|[1-9][0-9]*|-[1-9][0-9]*)$")]
struct DecimalI64(i64);
impl TryFrom<String> for DecimalI64 {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let digits = value.strip_prefix('-').unwrap_or(&value);
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) || (digits.starts_with('0') && digits != "0") || value == "-0" {
            return Err("invalid_decimal: canonical decimal required");
        }
        value.parse::<i64>().map(Self).map_err(|_| "out_of_range: expected i64 decimal")
    }
}
impl From<DecimalI64> for String { fn from(v: DecimalI64) -> Self { v.0.to_string() } }

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "String", into = "String")]
#[schema(value_type = String, pattern = r"^(0|[1-9][0-9]*)$")]
struct DecimalRevision(i64);
impl TryFrom<String> for DecimalRevision {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let parsed = DecimalI64::try_from(value)?;
        if parsed.0 < 0 { Err("out_of_range: revision must be nonnegative") } else { Ok(Self(parsed.0)) }
    }
}
impl From<DecimalRevision> for String { fn from(v: DecimalRevision) -> Self { v.0.to_string() } }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
struct OpaqueRevision(String);
impl utoipa::PartialSchema for OpaqueRevision {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        utoipa::openapi::schema::ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::String).min_length(Some(1)).into()
    }
}
impl ToSchema for OpaqueRevision {}
impl TryFrom<String> for OpaqueRevision {
    type Error = &'static str;
    fn try_from(v: String) -> Result<Self, Self::Error> {
        if v.is_empty() { Err("invalid_revision: nonempty token required") } else { Ok(Self(v)) }
    }
}
impl From<OpaqueRevision> for String { fn from(v: OpaqueRevision) -> Self { v.0 } }

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
struct RawI64Number(i64);

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum MetricValue {
    Amount { value: SafeNumber },
    TimeSpan { start: DateTime<FixedOffset>, end: DateTime<FixedOffset> },
    Habit { completed: bool },
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Goal {
    Inherited,
    Explicit { value: MetricValue },
    Suppressed,
}

fn nullable<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> { Option::<String>::deserialize(d) }

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct Contract {
    activity_date: NaiveDate,
    observed_at: DateTime<FixedOffset>,
    value: MetricValue,
    goal: Goal,
    #[serde(deserialize_with = "nullable")]
    #[schema(required = true)]
    note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum DecimalAmount { Amount { value: DecimalI64 } }

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum ErrorCode { InvalidInput, OutOfRange, InvalidDecimal, Stale, OperationPayloadMismatch }
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct ErrorDetail {
    code: ErrorCode,
    #[serde(deserialize_with = "nullable")]
    #[schema(required = true)]
    field: Option<String>,
    message: String,
    retryable: bool,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct ErrorEnvelope { error: ErrorDetail }

#[utoipa::path(post, path = "/prototype", request_body = Contract,
    responses((status = 200, body = Contract), (status = 400, body = ErrorEnvelope)))]
#[allow(dead_code)]
fn prototype() {}

#[derive(OpenApi)]
#[openapi(paths(prototype), components(schemas(Contract, MetricValue, Goal, SafeNumber, DecimalI64, DecimalAmount, RawI64Number, DecimalRevision, OpaqueRevision, ErrorCode, ErrorDetail, ErrorEnvelope)))]
struct Api;

fn parse<T: DeserializeOwned + Serialize>(raw: &str) -> Json {
    let mut de = serde_json::Deserializer::from_str(raw);
    match serde_path_to_error::deserialize::<_, T>(&mut de) {
        Ok(value) => match de.end() {
            Ok(()) => json!({"accepted":true,"round_trip_raw":serde_json::to_string(&value).unwrap()}),
            Err(error) => json!({"accepted":false,"error":{"code":"invalid_input","field":null,"message":error.to_string(),"retryable":false}}),
        },
        Err(error) => {
            let message = error.inner().to_string();
            let code = if message.contains("out_of_range:") { "out_of_range" } else if message.contains("invalid_decimal:") { "invalid_decimal" } else { "invalid_input" };
            json!({"accepted":false,"error":{"code":code,"field":error.path().to_string(),"message":message,"retryable":false}})
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    fs::create_dir_all("fixtures")?;
    let schema = Api::openapi();
    let compiler = std::process::Command::new("rustc").arg("--version").output()?;
    if !compiler.status.success() { return Err("rustc version command failed".into()); }
    let compiler_version = String::from_utf8(compiler.stdout)?.trim().to_owned();
    println!("Source {}. {}. serde 1.0.228, serde_json 1.0.145, utoipa 5.5.0, chrono 0.4.45, serde_path_to_error 0.1.20. Command: env RUSTC_WRAPPER= CARGO_TARGET_DIR=\"$PWD/rust/target\" cargo run --locked --manifest-path rust/Cargo.toml", SOURCE, compiler_version);
    fs::write("openapi.json", schema.to_pretty_json()?)?;
    let base = r#"{"activity_date":"2026-10-23","observed_at":"2026-10-23T08:00:00+02:00","value":{"kind":"amount","value":5},"goal":{"kind":"inherited"},"note":null}"#;
    let mut cases: Vec<(String, &str, String)> = vec![];
    let mut add = |id: &str, root: &'static str, raw: String| cases.push((id.into(), root, raw));
    add("amount_inherited", "Contract", base.into());
    add("habit_suppressed", "Contract", base.replace(r#"{"kind":"amount","value":5}"#, r#"{"kind":"habit","completed":true}"#).replace(r#"{"kind":"inherited"}"#, r#"{"kind":"suppressed"}"#));
    add("time_span_explicit", "Contract", base.replace(r#"{"kind":"amount","value":5}"#, r#"{"kind":"time_span","start":"2026-10-23T08:00:00+02:00","end":"2026-10-23T08:30:00+02:00"}"#).replace(r#"{"kind":"inherited"}"#, r#"{"kind":"explicit","value":{"kind":"amount","value":10}}"#));
    add("missing_note", "Contract", base.replace(",\"note\":null", ""));
    add("note_text", "Contract", base.replace("\"note\":null", "\"note\":\"hello\""));
    add("optional_label_null", "Contract", base.replace("\"note\":null", "\"note\":null,\"label\":null"));
    add("optional_label_text", "Contract", base.replace("\"note\":null", "\"note\":null,\"label\":\"x\""));
    add("unknown_top", "Contract", base.replace("\"note\":null", "\"note\":null,\"unexpected\":1"));
    add("unknown_value", "Contract", base.replace("\"value\":5", "\"value\":5,\"unexpected\":1"));
    add("suppression_with_value", "Contract", base.replace(r#"{"kind":"inherited"}"#, r#"{"kind":"suppressed","value":{"kind":"amount","value":1}}"#));
    add("goal_explicit_missing_value", "Contract", base.replace("\"inherited\"", "\"explicit\""));
    add("missing_value_tag", "Contract", base.replace("\"kind\":\"amount\",", ""));
    add("wrong_value_tag", "Contract", base.replace("\"amount\"", "\"duration\""));
    add("wrong_tag_type", "Contract", base.replace("\"kind\":\"amount\"", "\"kind\":3"));
    add("wrong_goal_tag", "Contract", base.replace("\"inherited\"", "\"unset\""));
    add("missing_goal_tag", "Contract", base.replace(r#"{"kind":"inherited"}"#, "{}"));
    add("amount_wrong_type", "Contract", base.replace("\"value\":5", "\"value\":\"5\""));
    add("date_impossible", "Contract", base.replace("2026-10-23\"", "2026-02-30\""));
    add("date_not_padded", "Contract", base.replace("2026-10-23\"", "2026-1-2\""));
    add("timestamp_missing_offset", "Contract", base.replace("08:00:00+02:00", "08:00:00"));
    add("timestamp_bad_offset", "Contract", base.replace("+02:00", "+25:00"));
    add("timestamp_utc", "Contract", base.replace("+02:00", "Z"));
    add("duplicate_value", "Contract", base.replace("\"value\":5", "\"value\":5,\"value\":6"));
    add("duplicate_note", "Contract", base.replace("\"note\":null", "\"note\":null,\"note\":\"last\""));
    add("duplicate_tag", "Contract", base.replace("\"kind\":\"amount\"", "\"kind\":\"habit\",\"kind\":\"amount\""));
    add("time_reversed", "Contract", base.replace(r#"{"kind":"amount","value":5}"#, r#"{"kind":"time_span","start":"2026-10-23T08:30:00+02:00","end":"2026-10-23T08:00:00+02:00"}"#));
    for (id, raw) in [
        ("zero","0"),("negative_zero","-0"),("positive_safe","9007199254740991"),("negative_safe","-9007199254740991"),
        ("positive_unsafe","9007199254740992"),("positive_rounded","9007199254740993"),("negative_unsafe","-9007199254740992"),("negative_rounded","-9007199254740993"),
        ("i64_max","9223372036854775807"),("i64_min","-9223372036854775808"),("above_i64","9223372036854775808"),("below_i64","-9223372036854775809"),
        ("float_lexical","1.0"),("exponent_lexical","1e0"),("fractional","1.5"),("float_near_safe","9007199254740990.5")
    ] {
        add(&format!("safe_{id}"), "SafeNumber", raw.into());
        add(&format!("raw_i64_{id}"), "RawI64Number", raw.into());
        add(&format!("amount_{id}"), "Contract", base.replace("\"value\":5", &format!("\"value\":{raw}")));
    }
    for (id, raw) in [
        ("zero",r#""0""#),("one",r#""1""#),("negative",r#""-5""#),
        ("safe",r#""9007199254740991""#),("beyond_safe",r#""9007199254740993""#),
        ("i64_max",r#""9223372036854775807""#),("i64_min",r#""-9223372036854775808""#),
        ("above_i64",r#""9223372036854775808""#),("below_i64",r#""-9223372036854775809""#),
        ("huge",r#""9999999999999999999999999999999999999999999999999999""#),
        ("negative_zero",r#""-0""#),("leading_zero",r#""01""#),("plus",r#""+1""#),("space",r#"" 1""#),("trailing_newline","\"1\\n\""),
        ("exponent",r#""1e3""#),("decimal",r#""1.0""#),("empty",r#""""#),("unicode",r#""١""#),("number","5")
    ] {
        add(&format!("decimal_{id}"), "DecimalI64", raw.into());
        add(&format!("decimal_amount_{id}"), "DecimalAmount", format!(r#"{{"kind":"amount","value":{raw}}}"#));
        add(&format!("revision_{id}"), "DecimalRevision", raw.into());
        add(&format!("opaque_{id}"), "OpaqueRevision", raw.into());
    }
    add("error_valid", "ErrorEnvelope", r#"{"error":{"code":"out_of_range","field":"value.value","message":"too large","retryable":false}}"#.into());
    add("error_null_field", "ErrorEnvelope", r#"{"error":{"code":"stale","field":null,"message":"old revision","retryable":false}}"#.into());
    add("error_missing_field", "ErrorEnvelope", r#"{"error":{"code":"stale","message":"old revision","retryable":false}}"#.into());
    add("error_bad_code", "ErrorEnvelope", r#"{"error":{"code":"surprise","field":null,"message":"bad","retryable":false}}"#.into());
    let mut results = vec![];
    for (id, root, raw) in cases {
        fs::write(format!("fixtures/{id}.json"), &raw)?;
        let outcome = match root {
            "Contract" => parse::<Contract>(&raw),
            "SafeNumber" => parse::<SafeNumber>(&raw),
            "RawI64Number" => parse::<RawI64Number>(&raw),
            "DecimalI64" => parse::<DecimalI64>(&raw),
            "DecimalAmount" => parse::<DecimalAmount>(&raw),
            "DecimalRevision" => parse::<DecimalRevision>(&raw),
            "OpaqueRevision" => parse::<OpaqueRevision>(&raw),
            "ErrorEnvelope" => parse::<ErrorEnvelope>(&raw),
            _ => unreachable!(),
        };
        results.push(json!({"id":id,"root":root,"raw":raw,"rust":outcome}));
    }
    let accepted = results.iter().filter(|r| r["rust"]["accepted"] == true).count();
    fs::write("rust-results.json", serde_json::to_string_pretty(&json!({"source_revision":SOURCE,"rustc":compiler_version,"direct_versions":{"serde":"1.0.228","serde_json":"1.0.145","utoipa":"5.5.0","chrono":"0.4.45","serde_path_to_error":"0.1.20"},"command":"env RUSTC_WRAPPER= CARGO_TARGET_DIR=$PWD/rust/target cargo run --locked --manifest-path rust/Cargo.toml","cases":results}))?)?;
    println!("Rust/serde observed {} fixtures, {} accepted and {} rejected. OpenAPI {} emitted with utoipa 5.5.0.", results.len(), accepted, results.len()-accepted, serde_json::to_string(&schema.openapi)?);
    Ok(())
}
