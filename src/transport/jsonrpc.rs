use serde::Serialize;
use serde_json::{Map, Value};

use crate::{Error, RemoteError, RequestId, Result};

/// A text request waiting to be emitted by the writer task.
#[derive(Debug)]
pub(crate) struct OutboundRequest {
    pub(crate) text: String,
}

/// A server-to-client JSON-RPC message.
#[derive(Debug)]
pub(crate) enum Inbound {
    Response {
        id: RequestId,
        result: Result<Value>,
    },
    Notification {
        method: String,
        params: Option<Value>,
    },
}

/// Serializes a JSON-RPC 2.0 request.
pub(crate) fn serialize_request<P>(id: RequestId, method: &str, params: Option<P>) -> Result<String>
where
    P: Serialize,
{
    if method.is_empty() {
        return Err(Error::configuration("JSON-RPC method must not be empty"));
    }

    let mut request = Map::new();
    request.insert("jsonrpc".into(), Value::String("2.0".into()));
    request.insert("id".into(), Value::Number(id.get().into()));
    request.insert("method".into(), Value::String(method.into()));
    if let Some(params) = params {
        request.insert(
            "params".into(),
            serde_json::to_value(params)
                .map_err(|error| Error::Serialization(error.to_string()))?,
        );
    }

    serde_json::to_string(&Value::Object(request))
        .map_err(|error| Error::Serialization(error.to_string()))
}

/// Parses and validates an inbound JSON-RPC 2.0 message.
pub(crate) fn parse_inbound(text: &str) -> Result<Inbound> {
    let value: Value = serde_json::from_str(text)
        .map_err(|error| Error::protocol(format!("invalid JSON received from peer: {error}")))?;
    let object = value
        .as_object()
        .ok_or_else(|| Error::protocol("inbound JSON-RPC message must be an object"))?;

    validate_jsonrpc_version(object)?;

    let has_id = object.contains_key("id");
    let has_method = object.contains_key("method");

    match (has_id, has_method) {
        (true, false) => parse_response(object),
        (false, true) => parse_notification(object),
        (true, true) => Err(Error::protocol(
            "server requests are unsupported; expected a response or notification",
        )),
        (false, false) => Err(Error::protocol(
            "inbound JSON-RPC object contains neither `id` nor `method`",
        )),
    }
}

fn validate_jsonrpc_version(object: &Map<String, Value>) -> Result<()> {
    let version = object
        .get("jsonrpc")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::protocol("missing string `jsonrpc` member"))?;
    if version != "2.0" {
        return Err(Error::protocol(format!(
            "unsupported JSON-RPC version `{version}`; expected `2.0`"
        )));
    }
    Ok(())
}

fn parse_response(object: &Map<String, Value>) -> Result<Inbound> {
    let id = parse_request_id(
        object
            .get("id")
            .expect("response path is selected only when id exists"),
    )?;

    match (object.get("result"), object.get("error")) {
        (Some(result), None) => Ok(Inbound::Response {
            id,
            result: Ok(result.clone()),
        }),
        (None, Some(error)) => Ok(Inbound::Response {
            id,
            result: Err(Error::Remote(parse_remote_error(error)?)),
        }),
        (Some(_), Some(_)) => Err(Error::protocol(
            "JSON-RPC response must contain exactly one of `result` or `error`",
        )),
        (None, None) => Err(Error::protocol(
            "JSON-RPC response contains neither `result` nor `error`",
        )),
    }
}

fn parse_notification(object: &Map<String, Value>) -> Result<Inbound> {
    let method = object
        .get("method")
        .and_then(Value::as_str)
        .filter(|method| !method.is_empty())
        .ok_or_else(|| Error::protocol("notification `method` must be a non-empty string"))?;

    if let Some(params) = object.get("params")
        && !params.is_array()
        && !params.is_object()
        && !params.is_null()
    {
        return Err(Error::protocol(
            "JSON-RPC notification `params` must be an object, array, or null",
        ));
    }

    Ok(Inbound::Notification {
        method: method.to_owned(),
        params: object.get("params").cloned(),
    })
}

fn parse_request_id(value: &Value) -> Result<RequestId> {
    let id = value
        .as_u64()
        .filter(|id| *id != 0)
        .ok_or_else(|| Error::protocol("response `id` must be a non-zero unsigned integer"))?;
    Ok(RequestId::new(id))
}

fn parse_remote_error(value: &Value) -> Result<RemoteError> {
    let object = value
        .as_object()
        .ok_or_else(|| Error::protocol("JSON-RPC `error` member must be an object"))?;
    let code = object
        .get("code")
        .and_then(Value::as_i64)
        .ok_or_else(|| Error::protocol("JSON-RPC error `code` must be an integer"))?;
    let message = object
        .get("message")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::protocol("JSON-RPC error `message` must be a string"))?;

    Ok(RemoteError {
        code,
        message: message.to_owned(),
        data: object.get("data").cloned(),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn serializes_request_without_params() {
        let text = serialize_request(
            RequestId::new(7),
            "minecraft:server/status",
            Option::<Value>::None,
        )
        .expect("request should serialize");
        assert_eq!(
            serde_json::from_str::<Value>(&text).unwrap(),
            json!({"jsonrpc":"2.0", "id":7, "method":"minecraft:server/status"})
        );
    }

    #[test]
    fn parses_remote_error_response() {
        let inbound = parse_inbound(
            r#"{"jsonrpc":"2.0","id":9,"error":{"code":-32601,"message":"not found","data":{"method":"x"}}}"#,
        )
        .expect("response should parse");

        match inbound {
            Inbound::Response {
                id,
                result: Err(Error::Remote(error)),
            } => {
                assert_eq!(id.get(), 9);
                assert_eq!(error.code, -32601);
                assert_eq!(error.message, "not found");
            }
            _ => panic!("unexpected inbound message"),
        }
    }

    #[test]
    fn rejects_response_with_result_and_error() {
        let error = parse_inbound(
            r#"{"jsonrpc":"2.0","id":1,"result":{},"error":{"code":1,"message":"bad"}}"#,
        )
        .expect_err("ambiguous response must be rejected");
        assert!(matches!(error, Error::Protocol(_)));
    }

    #[test]
    fn parses_jsonrpc_fixtures() {
        let success = parse_inbound(include_str!("../../tests/fixtures/jsonrpc/success.json"));
        assert!(matches!(
            success,
            Ok(Inbound::Response { result: Ok(_), .. })
        ));

        let remote_error = parse_inbound(include_str!(
            "../../tests/fixtures/jsonrpc/remote_error.json"
        ));
        assert!(matches!(
            remote_error,
            Ok(Inbound::Response {
                result: Err(Error::Remote(_)),
                ..
            })
        ));

        let malformed = parse_inbound(include_str!(
            "../../tests/fixtures/jsonrpc/malformed_response.json"
        ));
        assert!(matches!(malformed, Err(Error::Protocol(_))));
    }

    #[test]
    fn arbitrary_inbound_text_never_panics() {
        const SEEDS: &[&str] = &[
            "",
            "null",
            "[]",
            "{",
            r#"{"jsonrpc":"2.0"}"#,
            r#"{"jsonrpc":"2.0","id":0,"result":null}"#,
            r#"{"jsonrpc":"2.0","method":5}"#,
            r#"{"jsonrpc":"1.0","id":1,"result":{}}"#,
        ];

        for seed in SEEDS {
            let outcome = std::panic::catch_unwind(|| parse_inbound(seed));
            assert!(outcome.is_ok(), "parser panicked for seed {seed:?}");
        }

        let mut state = 0x9e37_79b9_u32;
        for length in 0..=256 {
            let mut bytes = Vec::with_capacity(length);
            for _ in 0..length {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                bytes.push((state >> 24) as u8);
            }
            let text = String::from_utf8_lossy(&bytes);
            let outcome = std::panic::catch_unwind(|| parse_inbound(&text));
            assert!(
                outcome.is_ok(),
                "parser panicked for generated length {length}"
            );
        }
    }
}
