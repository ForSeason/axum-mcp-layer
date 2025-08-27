use std::task::{Context, Poll};
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, Response, Method, StatusCode};
use tower::{Layer, Service};

use crate::http::{handle_post, handle_sse_get};
use crate::registry::ToolRegistry;
use crate::security::{AllowedOrigins, Auth, VersionPolicy, REQUIRED_PROTOCOL_VERSION, FALLBACK_PROTOCOL_VERSION};

#[derive(Clone)]
pub struct McpLayerConfig {
    pub path: &'static str,
    pub require_version: bool,
    pub allowed_origins: AllowedOrigins,
    pub enable_sse: bool,
    pub auth: Auth,
    pub version_policy: VersionPolicy,
}

impl Default for McpLayerConfig {
    fn default() -> Self {
        Self { path: "/mcp", require_version: true, allowed_origins: AllowedOrigins::LocalhostOnly, enable_sse: false, auth: Auth::None, version_policy: VersionPolicy::AllowFallback { required: REQUIRED_PROTOCOL_VERSION, fallback: FALLBACK_PROTOCOL_VERSION } }
    }
}

#[derive(Clone)]
pub struct McpLayer {
    registry: Arc<ToolRegistry>,
    config: McpLayerConfig,
}

impl McpLayer {
    pub fn new(registry: Arc<ToolRegistry>, config: McpLayerConfig) -> Self { Self { registry, config } }
}

impl<S> Layer<S> for McpLayer {
    type Service = McpService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        McpService { inner, registry: self.registry.clone(), config: self.config.clone(), path: self.config.path }
    }
}

#[derive(Clone)]
pub struct McpService<S> {
    inner: S,
    registry: Arc<ToolRegistry>,
    config: McpLayerConfig,
    path: &'static str,
}

impl<S> Service<Request<Body>> for McpService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> { self.inner.poll_ready(cx) }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let path = req.uri().path().to_string();
        let method = req.method().clone();
        let mut inner = self.inner.clone();
        let registry = self.registry.clone();
        let allowed = self.config.allowed_origins;
        let intercept_post = path == self.path && method == Method::POST;
        let intercept_get = path == self.path && method == Method::GET && self.config.enable_sse;
        let auth = self.config.auth.clone();
        let policy = self.config.version_policy;
        Box::pin(async move {
            if intercept_post {
                let resp = handle_post(req, &registry, allowed, auth.clone(), policy).await;
                Ok(resp)
            } else if intercept_get {
                let resp = handle_sse_get(&registry, allowed, auth).await;
                Ok(resp)
            } else {
                inner.call(req).await
            }
        })
    }
}
