//! API key authentication for the Layer 1 gateway.
//!
//! Loads `api_keys.json` at startup. Each key has scoped permissions:
//! read-only, session-create, or full-access. All MCP, REST, and
//! WebSocket endpoints validate the `X-API-Key` header through this
//! module before any gRPC call is made.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Permissions scoped to an API key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyPermission {
    ReadOnly,
    SessionCreate,
    FullAccess,
}

impl KeyPermission {
    /// Returns true if this permission allows creating sessions.
    #[must_use] 
    pub const fn can_create_session(&self) -> bool {
        matches!(
            self,
            Self::SessionCreate | Self::FullAccess
        )
    }

    /// Returns true if this permission allows mutating state.
    #[must_use] 
    pub const fn can_write(&self) -> bool {
        matches!(
            self,
            Self::SessionCreate | Self::FullAccess
        )
    }

    /// Returns true if this permission allows reading state.
    #[must_use] 
    pub const fn can_read(&self) -> bool {
        true // all permissions allow reading
    }
}

/// A single API key entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyEntry {
    pub key: String,
    pub name: String,
    pub permission: KeyPermission,
}

/// The API key registry, loaded from disk at startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRegistry {
    keys: HashMap<String, ApiKeyEntry>,
}

impl KeyRegistry {
    /// Load keys from a JSON file on disk. Creates a default
    /// admin key if the file does not exist.
    pub fn load(path: Option<PathBuf>) -> Result<Self, AuthError> {
        let path = path.unwrap_or_else(|| {
            let home = dirs_next();
            home.join(".ans").join("api_keys.json")
        });

        match std::fs::read_to_string(&path) {
            Ok(contents) => {
                let registry: Self = serde_json::from_str(&contents)
                    .map_err(|e| AuthError::ParseError(e.to_string()))?;
                Ok(registry)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(AuthError::ConfigMissing(path))
            }
            Err(e) => Err(AuthError::IoError(e)),
        }
    }

    /// Validate an API key and return its entry. Uses constant-time comparison
    /// to mitigate timing side-channel attacks.
    #[must_use] 
    pub fn validate(&self, api_key: &str) -> Option<&ApiKeyEntry> {
        if api_key.is_empty() {
            return None;
        }
        self.keys
            .values()
            .find(|entry| constant_time_eq(api_key.as_bytes(), entry.key.as_bytes()))
    }
}

/// Cached in an Arc so all request handlers share the same registry.
pub type SharedKeyRegistry = Arc<KeyRegistry>;

/// Errors that can occur during authentication.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("missing API key")]
    MissingKey,

    #[error("invalid API key")]
    InvalidKey,

    #[error("insufficient permissions: required {required:?}, have {have:?}")]
    InsufficientPermissions {
        required: KeyPermission,
        have: KeyPermission,
    },

    #[error("api_keys.json not found at {0}")]
    ConfigMissing(PathBuf),

    #[error("failed to parse api_keys.json: {0}")]
    ParseError(String),

    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Extract an API key from an HTTP header or query parameter.
/// Returns `None` for empty keys.
pub fn extract_api_key(headers: &axum::http::HeaderMap, query: Option<&str>) -> Option<String> {
    let key = if let Some(value) = headers.get("x-api-key") {
        value.to_str().ok().map(String::from)
    } else if let Some(value) = headers.get("authorization") {
        value.to_str().ok().and_then(|auth| auth.strip_prefix("Bearer ").map(String::from))
    } else if let Some(q) = query {
        q.split('&')
            .find_map(|pair| pair.strip_prefix("api_key=").map(String::from))
    } else {
        None
    };
    // Reject empty keys
    key.filter(|k| !k.is_empty())
}

/// Axum extractor that validates API keys.
///
/// Usage: add `AuthLayer::new(registry)` to your axum router.
#[derive(Clone)]
pub struct AuthLayer {
    registry: SharedKeyRegistry,
}

impl AuthLayer {
    #[must_use] 
    pub const fn new(registry: SharedKeyRegistry) -> Self {
        Self { registry }
    }
}

impl<S> tower::Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            inner,
            registry: Arc::clone(&self.registry),
        }
    }
}

/// Tower service that validates the API key before each request.
#[derive(Clone)]
pub struct AuthMiddleware<S> {
    inner: S,
    registry: SharedKeyRegistry,
}

impl<S, B> tower::Service<axum::http::Request<B>> for AuthMiddleware<S>
where
    S: tower::Service<axum::http::Request<B>, Response = axum::response::Response>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = futures_util::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: axum::http::Request<B>) -> Self::Future {
        let registry = Arc::clone(&self.registry);
        let inner = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, inner);

        // Extract headers before the request is consumed
        let headers = req.headers().clone();
        let query = req.uri().query().map(String::from);

        Box::pin(async move {
            let api_key = extract_api_key(&headers, query.as_deref());
            // Look up the entry once for both validation and permission scoping
            let entry = api_key.as_ref().and_then(|k| registry.validate(k));
            match entry {
                Some(entry) => {
                    req.extensions_mut().insert(entry.permission.clone());
                    inner.call(req).await
                }
                None if api_key.is_some() => {
                    #[allow(clippy::unwrap_used)]
                    // SAFETY: Body::from(&str) is infallible — the builder only
                    // returns Err when a previously-set body is overwritten.
                    let response = axum::response::Response::builder()
                        .status(axum::http::StatusCode::UNAUTHORIZED)
                        .body(axum::body::Body::from(r#"{"error":"invalid API key"}"#))
                        .unwrap();
                    Ok(response)
                }
                None => {
                    #[allow(clippy::unwrap_used)]
                    // SAFETY: see above.
                    let response = axum::response::Response::builder()
                        .status(axum::http::StatusCode::UNAUTHORIZED)
                        .body(axum::body::Body::from(
                            r#"{"error":"missing X-API-Key header"}"#,
                        ))
                        .unwrap();
                    Ok(response)
                }
            }
        })
    }
}

/// Constant-time byte comparison to prevent timing side-channel attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut acc: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        acc |= x ^ y;
    }
    acc == 0
}

/// Helper to get the user's home directory.
fn dirs_next() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME")).map_or_else(|_| PathBuf::from("."), PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_validation() {
        let registry = KeyRegistry {
            keys: [(
                "admin".into(),
                ApiKeyEntry {
                    key: "secret-123".into(),
                    name: "Test Admin".into(),
                    permission: KeyPermission::FullAccess,
                },
            )]
            .into(),
        };

        assert!(registry.validate("secret-123").is_some());
        assert!(registry.validate("wrong-key").is_none());
    }

    #[test]
    fn test_permission_checks() {
        assert!(KeyPermission::FullAccess.can_create_session());
        assert!(KeyPermission::SessionCreate.can_create_session());
        assert!(!KeyPermission::ReadOnly.can_create_session());

        assert!(KeyPermission::FullAccess.can_write());
        assert!(KeyPermission::SessionCreate.can_write());
        assert!(!KeyPermission::ReadOnly.can_write());

        assert!(KeyPermission::ReadOnly.can_read());
    }

    #[test]
    fn test_extract_api_key_from_header() {
        use axum::http::{HeaderMap, HeaderValue};
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("my-key"));
        assert_eq!(extract_api_key(&headers, None), Some("my-key".to_string()));
    }
}
