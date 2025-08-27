use http::HeaderMap;

#[derive(Clone, Copy, Debug)]
pub enum AllowedOrigins {
    LocalhostOnly,
    LocalhostAll, // 127.0.0.1, localhost, [::1]
    List(&'static [&'static str]), // exact origins
    PortRangeLocalhost { start: u16, end: u16 },
}

pub fn is_origin_allowed(h: &HeaderMap, allow: AllowedOrigins) -> bool {
    match allow {
        AllowedOrigins::LocalhostOnly => {
            match h.get(http::header::ORIGIN).and_then(|v| v.to_str().ok()) {
                None => true, // no Origin is fine for local tools
                Some(origin) => origin.starts_with("http://127.0.0.1:") || origin == "http://127.0.0.1",
            }
        }
        AllowedOrigins::LocalhostAll => {
            match h.get(http::header::ORIGIN).and_then(|v| v.to_str().ok()) {
                None => true,
                Some(origin) => origin.starts_with("http://127.0.0.1:")
                    || origin == "http://127.0.0.1"
                    || origin.starts_with("http://localhost:")
                    || origin == "http://localhost"
                    || origin.starts_with("http://[::1]:")
                    || origin == "http://[::1]",
            }
        }
        AllowedOrigins::List(list) => {
            match h.get(http::header::ORIGIN).and_then(|v| v.to_str().ok()) {
                None => false,
                Some(origin) => list.iter().any(|o| *o == origin),
            }
        }
        AllowedOrigins::PortRangeLocalhost { start, end } => {
            match h.get(http::header::ORIGIN).and_then(|v| v.to_str().ok()) {
                None => true,
                Some(origin) => {
                    let prefix = "http://127.0.0.1:";
                    if let Some(port_str) = origin.strip_prefix(prefix) {
                        if let Ok(port) = port_str.parse::<u16>() {
                            return port >= start && port <= end;
                        }
                    }
                    false
                }
            }
        }
    }
}

pub const REQUIRED_PROTOCOL_VERSION: &str = "2025-06-18";
pub const FALLBACK_PROTOCOL_VERSION: &str = "2025-03-26";

#[derive(Clone, Copy)]
pub enum VersionPolicy {
    Strict(&'static str),
    AllowFallback { required: &'static str, fallback: &'static str },
}

pub fn has_valid_protocol_version_with(h: &HeaderMap, policy: &VersionPolicy) -> bool {
    let header = h.get("MCP-Protocol-Version").and_then(|v| v.to_str().ok());
    match policy {
        VersionPolicy::Strict(req) => header.map(|v| v == *req).unwrap_or(false),
        VersionPolicy::AllowFallback { required, fallback } => match header {
            Some(v) => v == *required || v == *fallback,
            None => true, // allow missing: treat as fallback
        }
    }
}

#[derive(Clone, Debug)]
pub enum Auth {
    None,
    Bearer { token: String },
}

pub fn is_authorized(h: &HeaderMap, auth: &Auth) -> bool {
    match auth {
        Auth::None => true,
        Auth::Bearer { token } => h
            .get(http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .map(|s| s == format!("Bearer {}", token))
            .unwrap_or(false),
    }
}
