use axum::http::HeaderMap;
use axum::response::Response;
use sdkwork_web_core::{
    problem_response, ProblemCorrelation, WebFrameworkError, WebFrameworkErrorKind,
};
use serde::Serialize;

pub(crate) const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";
pub(crate) const REQUEST_HASH_HEADER: &str = "Sdkwork-Request-Hash";
pub(crate) const REQUEST_NO_HEADER: &str = "Sdkwork-Request-No";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppWriteCommandHeaders {
    pub idempotency_key: String,
    pub request_hash: String,
    pub request_no: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WriteCommandHeaderError {
    MissingHeader(&'static str),
    InvalidHeader(&'static str),
}

pub(crate) fn stable_command_request_hash(scope: &str, parts: &[&str]) -> String {
    let mut normalized = vec![scope];
    normalized.extend(parts);
    normalized
        .iter()
        .map(|part| {
            part.chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                        character
                    } else {
                        '-'
                    }
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("-")
}

pub(crate) fn stable_json_request_hash(
    scope: &str,
    value: &impl Serialize,
) -> Result<String, WriteCommandHeaderError> {
    let value = serde_json::to_value(value).map_err(|_| {
        WriteCommandHeaderError::InvalidHeader(
            "request body could not be canonicalized for request hash validation",
        )
    })?;
    Ok(stable_canonical_json_request_hash(scope, &value))
}

pub(crate) fn stable_canonical_json_request_hash(scope: &str, value: &serde_json::Value) -> String {
    stable_command_request_hash(scope, &[&canonical_json_string(value)])
}

fn canonical_json_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => {
            serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned())
        }
        serde_json::Value::Array(values) => {
            let items = values
                .iter()
                .map(canonical_json_string)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{items}]")
        }
        serde_json::Value::Object(values) => {
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            let items = keys
                .into_iter()
                .filter(|key| !values[*key].is_null())
                .map(|key| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap_or_else(|_| "\"\"".to_owned()),
                        canonical_json_string(&values[key])
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{items}}}")
        }
    }
}

#[allow(clippy::result_large_err)]
pub(crate) fn validate_write_payload(
    headers: &HeaderMap,
    scope: &str,
    body: &impl Serialize,
    fallback_request_no: impl FnOnce(&str) -> String,
) -> Result<AppWriteCommandHeaders, WriteCommandHeaderError> {
    let write_headers = parse_required_write_command_headers(headers, fallback_request_no)?;
    let expected_hash = stable_json_request_hash(scope, body)?;
    if expected_hash.trim() != write_headers.request_hash.trim() {
        return Err(WriteCommandHeaderError::InvalidHeader(
            "Sdkwork-Request-Hash does not match the command payload",
        ));
    }
    Ok(write_headers)
}

#[allow(clippy::result_large_err)]
pub(crate) fn validate_app_write_payload(
    headers: &HeaderMap,
    scope: &str,
    body: &impl Serialize,
    fallback_request_no: impl FnOnce(&str) -> String,
) -> Result<AppWriteCommandHeaders, Response> {
    validate_write_payload(headers, scope, body, fallback_request_no)
        .map_err(write_command_header_error_to_app_response)
}

pub(crate) fn write_payload_with_route_param(
    route_param_key: &str,
    route_param_value: &str,
    body: &impl Serialize,
) -> serde_json::Value {
    let mut payload = serde_json::to_value(body).expect("write payload must serialize");
    if let serde_json::Value::Object(ref mut fields) = payload {
        fields.insert(
            route_param_key.to_string(),
            serde_json::Value::String(route_param_value.to_string()),
        );
    }
    payload
}

#[allow(clippy::result_large_err)]
pub(crate) fn parse_required_write_command_headers(
    headers: &HeaderMap,
    fallback_request_no: impl FnOnce(&str) -> String,
) -> Result<AppWriteCommandHeaders, WriteCommandHeaderError> {
    let idempotency_key = required_text_header(headers, IDEMPOTENCY_KEY_HEADER)
        .map_err(|_| WriteCommandHeaderError::MissingHeader(IDEMPOTENCY_KEY_HEADER))?;
    let request_hash = required_text_header(headers, REQUEST_HASH_HEADER)
        .map_err(|_| WriteCommandHeaderError::MissingHeader(REQUEST_HASH_HEADER))?;
    let request_no = optional_text_header(headers, REQUEST_NO_HEADER)
        .unwrap_or_else(|| fallback_request_no(&idempotency_key));
    Ok(AppWriteCommandHeaders {
        idempotency_key,
        request_hash,
        request_no,
    })
}

fn write_command_header_error_to_app_response(error: WriteCommandHeaderError) -> Response {
    match error {
        WriteCommandHeaderError::MissingHeader(name) => {
            command_header_error_response(format!("{name} header is required"))
        }
        WriteCommandHeaderError::InvalidHeader(message) => validation_response(message),
    }
}

#[allow(clippy::result_large_err)]
fn required_text_header(headers: &HeaderMap, name: &'static str) -> Result<String, Response> {
    let value = headers
        .get(name)
        .ok_or_else(|| command_header_error_response(format!("{name} header is required")))?
        .to_str()
        .map(str::trim)
        .map_err(|_| command_header_error_response(format!("{name} header value is invalid")))?;
    if value.is_empty() {
        return Err(command_header_error_response(format!(
            "{name} header is required"
        )));
    }
    Ok(value.to_owned())
}

fn optional_text_header(headers: &HeaderMap, name: &'static str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn command_header_error_response(message: impl Into<String>) -> Response {
    problem_detail_response(message)
}

fn validation_response(message: impl Into<String>) -> Response {
    problem_detail_response(message)
}

fn problem_detail_response(message: impl Into<String>) -> Response {
    let trace_id = uuid::Uuid::new_v4().to_string();
    problem_response(
        &WebFrameworkError {
            kind: WebFrameworkErrorKind::BadRequest,
            message: message.into(),
            retry_after_seconds: None,
            auth_profile: None,
            failed_stage: None,
            reason: None,
        },
        ProblemCorrelation::from(Some(trace_id.as_str())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{header::CONTENT_TYPE, StatusCode};

    #[test]
    fn stable_command_request_hash_is_deterministic() {
        let first = stable_command_request_hash("scope", &["100001", "request-1"]);
        let second = stable_command_request_hash("scope", &["100001", "request-1"]);
        assert_eq!(first, second);
        assert!(!first.is_empty());
    }

    #[test]
    fn stable_json_request_hash_matches_struct_and_value_payloads() {
        use serde::Deserialize;

        let body_json = r#"{"methodKey":"wechat_pay","displayName":"WeChat Pay","providerCode":"wechat_pay","status":"active"}"#;
        let value: serde_json::Value = serde_json::from_str(body_json).expect("json");
        let from_value = stable_canonical_json_request_hash("payment-method-upsert", &value);

        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct UpsertPaymentMethodBody {
            method_key: Option<String>,
            display_name: Option<String>,
            provider_code: Option<String>,
            status: Option<String>,
            sort_order: Option<i64>,
        }

        let body: UpsertPaymentMethodBody = serde_json::from_str(body_json).expect("body");
        let from_struct = stable_json_request_hash("payment-method-upsert", &body).expect("hash");

        assert_eq!(from_value, from_struct);
    }

    #[test]
    fn missing_command_header_returns_bad_request_problem_detail() {
        let response = validate_app_write_payload(
            &HeaderMap::new(),
            "invoices.create",
            &serde_json::json!({ "title": "Example" }),
            |_| "request-1".to_owned(),
        )
        .expect_err("missing headers must fail");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("application/problem+json")
        );
    }
}
