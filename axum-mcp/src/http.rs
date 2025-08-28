use axum::Json;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::{
    Response,
    sse::{Event, Sse},
};
use futures::stream;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;

use crate::registry::ToolRegistry;
use crate::security::{
    AllowedOrigins, Auth, VersionPolicy, has_valid_protocol_version_with, is_authorized,
    is_origin_allowed,
};

#[derive(Deserialize)]
struct RawOp {
    op: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    args: Value,
}

#[derive(Serialize)]
struct ToolMeta {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    desc: Option<&'static str>,
    input_schema: crate::schema::RootSchema,
    output_schema: crate::schema::RootSchema,
    #[serde(rename = "structuredContent")]
    structured: bool,
}

pub async fn handle_post(
    req: Request<Body>,
    registry: &ToolRegistry,
    allowed: AllowedOrigins,
    auth: Auth,
    version_policy: VersionPolicy,
) -> Response {
    // Security checks
    if !has_valid_protocol_version_with(req.headers(), &version_policy) {
        return axum::response::Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("missing/invalid MCP-Protocol-Version"))
            .unwrap();
    }
    if !is_origin_allowed(req.headers(), allowed) {
        return axum::response::Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from("forbidden origin"))
            .unwrap();
    }
    if !is_authorized(req.headers(), &auth) {
        return axum::response::Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::from("unauthorized"))
            .unwrap();
    }

    let (_parts, body) = req.into_parts();
    let bytes = match axum::body::to_bytes(body, 1 << 20).await {
        Ok(b) => b,
        Err(_) => {
            return axum::response::IntoResponse::into_response((
                StatusCode::BAD_REQUEST,
                "invalid body",
            ));
        }
    };
    let raw: RawOp = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(e) => {
            return axum::response::IntoResponse::into_response((
                StatusCode::BAD_REQUEST,
                format!("invalid json: {}", e),
            ));
        }
    };

    match raw.op.as_str() {
        "tools/list" => {
            let list = registry.list().await;
            let tools: Vec<_> = list
                .into_iter()
                .map(|(name, desc, i, o)| ToolMeta {
                    name,
                    desc,
                    input_schema: i,
                    output_schema: o,
                    structured: true,
                })
                .collect();
            axum::response::IntoResponse::into_response(Json(json!({"tools": tools})))
        }
        "tools/call" => {
            let name = match raw.name {
                Some(n) => n,
                None => {
                    return axum::response::IntoResponse::into_response((
                        StatusCode::BAD_REQUEST,
                        "missing name",
                    ));
                }
            };
            match registry.call(&name, raw.args).await {
                Ok(v) => axum::response::IntoResponse::into_response(Json(
                    json!({"ok": true, "result": v}),
                )),
                Err(e) => {
                    use crate::tool::ToolError::*;
                    let (code, status) = match &e {
                        NotFound(_) => ("tool_not_found", StatusCode::NOT_FOUND),
                        InvalidArgs(_) => ("invalid_args", StatusCode::BAD_REQUEST),
                        Internal(_) => ("internal", StatusCode::INTERNAL_SERVER_ERROR),
                    };
                    let body = json!({"ok": false, "code": code, "message": e.to_string()});
                    axum::response::Response::builder()
                        .status(status)
                        .header(axum::http::header::CONTENT_TYPE, "application/json")
                        .body(Body::from(serde_json::to_vec(&body).unwrap()))
                        .unwrap()
                }
            }
        }
        _ => axum::response::IntoResponse::into_response((StatusCode::NOT_FOUND, "unknown op")),
    }
}

pub async fn handle_sse_get(
    registry: &ToolRegistry,
    _allowed: AllowedOrigins,
    _auth: Auth,
) -> Response {
    // Simple one-shot SSE announcing readiness and available tool count
    let count = registry.list().await.len();
    let fut = async move {
        let ready = json!({"event":"ready","tool_count": count});
        let ev = Event::default().data(serde_json::to_string(&ready).unwrap());
        Ok::<_, std::convert::Infallible>(ev)
    };
    let stream = stream::once(fut);
    axum::response::IntoResponse::into_response(
        Sse::new(stream).keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("ping"),
        ),
    )
}
