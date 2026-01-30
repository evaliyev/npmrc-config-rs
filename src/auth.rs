//! Authentication handling for npm registries.
//!
//! This module implements "nerf-darting" - npm's mechanism for scoping
//! credentials to specific registries to prevent credential leakage.

use crate::error::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use std::fmt;
use std::path::PathBuf;
use url::Url;

/// Credentials for authenticating with an npm registry.
///
/// # Security Notes
///
/// - The `Debug` implementation redacts sensitive fields (tokens, passwords)
///   to prevent accidental credential leakage in logs or error messages.
/// - `PartialEq` is intentionally not implemented to prevent timing attacks
///   when comparing credentials.
#[derive(Clone)]
pub enum Credentials {
    /// Bearer token authentication (`_authToken`).
    /// This is the recommended authentication method.
    Token {
        token: String,
        /// Optional client certificate for mTLS.
        cert: Option<ClientCert>,
    },

    /// Username and password authentication.
    /// The password is decoded from base64 `_password` field.
    BasicAuth {
        username: String,
        password: String,
        /// Optional client certificate for mTLS.
        cert: Option<ClientCert>,
    },

    /// Legacy `_auth` field containing base64-encoded `username:password`.
    LegacyAuth {
        /// The raw base64-encoded auth string.
        auth: String,
        /// Decoded username.
        username: String,
        /// Decoded password.
        password: String,
        /// Optional client certificate for mTLS.
        cert: Option<ClientCert>,
    },

    /// Client certificate only (mTLS without token/password auth).
    ClientCertOnly(ClientCert),
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Credentials::Token { cert, .. } => f
                .debug_struct("Token")
                .field("token", &"[REDACTED]")
                .field("cert", cert)
                .finish(),
            Credentials::BasicAuth {
                username, cert, ..
            } => f
                .debug_struct("BasicAuth")
                .field("username", username)
                .field("password", &"[REDACTED]")
                .field("cert", cert)
                .finish(),
            Credentials::LegacyAuth {
                username, cert, ..
            } => f
                .debug_struct("LegacyAuth")
                .field("auth", &"[REDACTED]")
                .field("username", username)
                .field("password", &"[REDACTED]")
                .field("cert", cert)
                .finish(),
            Credentials::ClientCertOnly(cert) => {
                f.debug_tuple("ClientCertOnly").field(cert).finish()
            }
        }
    }
}

/// Client certificate for mTLS authentication.
#[derive(Debug, Clone, PartialEq)]
pub struct ClientCert {
    /// Path to the certificate file.
    pub certfile: PathBuf,
    /// Path to the private key file.
    pub keyfile: PathBuf,
}

/// Convert a registry URL to nerf-dart format for credential lookup.
///
/// Nerf-darting strips the protocol and normalizes the path to prevent
/// credentials from leaking across registries.
///
/// # Examples
///
/// ```
/// use url::Url;
/// use npmrc_config_rs::nerf_dart;
///
/// let url = Url::parse("https://registry.npmjs.org/").unwrap();
/// assert_eq!(nerf_dart(&url), "//registry.npmjs.org/");
///
/// let url = Url::parse("https://example.com/some/path/").unwrap();
/// assert_eq!(nerf_dart(&url), "//example.com/some/path/");
/// ```
pub fn nerf_dart(url: &Url) -> String {
    // Get host and path, normalizing the path to end with /
    let host = url.host_str().unwrap_or("");
    let port = url.port().map(|p| format!(":{}", p)).unwrap_or_default();

    // Normalize path: get parent directory and ensure trailing slash
    // This mimics `new URL('.', from)` in JavaScript which resolves to the directory
    let path = url.path();
    let normalized_path = if path.ends_with('/') {
        path.to_string()
    } else {
        // Get the "directory" part of the path (like `new URL('.', from)` in JS)
        // For "/some/path", we want "/some/"
        // For "/", we want "/"
        match path.rfind('/') {
            Some(idx) => {
                // Include everything up to and including the last /
                path[..=idx].to_string()
            }
            None => "/".to_string(),
        }
    };

    format!("//{}{}{}", host, port, normalized_path)
}

/// Decode a base64-encoded password.
pub fn decode_password(encoded: &str) -> Result<String> {
    let decoded = BASE64.decode(encoded)?;
    Ok(String::from_utf8(decoded)?)
}

/// Parse legacy `_auth` field (base64-encoded `username:password`).
pub fn parse_legacy_auth(auth: &str) -> Result<(String, String)> {
    let decoded = String::from_utf8(BASE64.decode(auth)?)?;
    let mut parts = decoded.splitn(2, ':');
    let username = parts.next().unwrap_or("").to_string();
    let password = parts.next().unwrap_or("").to_string();
    Ok((username, password))
}

impl Credentials {
    /// Get the client certificate if present.
    pub fn client_cert(&self) -> Option<&ClientCert> {
        match self {
            Credentials::Token { cert, .. } => cert.as_ref(),
            Credentials::BasicAuth { cert, .. } => cert.as_ref(),
            Credentials::LegacyAuth { cert, .. } => cert.as_ref(),
            Credentials::ClientCertOnly(cert) => Some(cert),
        }
    }

    /// Get the token if this is token-based auth.
    pub fn token(&self) -> Option<&str> {
        match self {
            Credentials::Token { token, .. } => Some(token),
            _ => None,
        }
    }

    /// Get username and password if available.
    pub fn username_password(&self) -> Option<(&str, &str)> {
        match self {
            Credentials::BasicAuth {
                username, password, ..
            } => Some((username, password)),
            Credentials::LegacyAuth {
                username, password, ..
            } => Some((username, password)),
            _ => None,
        }
    }

    /// Get the base64-encoded auth string for HTTP Basic auth.
    pub fn basic_auth_header(&self) -> Option<String> {
        match self {
            Credentials::BasicAuth {
                username, password, ..
            } => {
                let auth = format!("{}:{}", username, password);
                Some(BASE64.encode(auth.as_bytes()))
            }
            Credentials::LegacyAuth { auth, .. } => Some(auth.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nerf_dart_simple() {
        let url = Url::parse("https://registry.npmjs.org/").unwrap();
        assert_eq!(nerf_dart(&url), "//registry.npmjs.org/");
    }

    #[test]
    fn test_nerf_dart_with_path() {
        let url = Url::parse("https://example.com/some/path/").unwrap();
        assert_eq!(nerf_dart(&url), "//example.com/some/path/");
    }

    #[test]
    fn test_nerf_dart_normalizes_path() {
        // Without trailing slash
        let url = Url::parse("https://example.com/some/path").unwrap();
        assert_eq!(nerf_dart(&url), "//example.com/some/");

        // Package path gets normalized to registry root
        let url = Url::parse("https://registry.npmjs.org/package-name").unwrap();
        assert_eq!(nerf_dart(&url), "//registry.npmjs.org/");
    }

    #[test]
    fn test_nerf_dart_with_port() {
        let url = Url::parse("https://registry.example.com:8080/npm/").unwrap();
        assert_eq!(nerf_dart(&url), "//registry.example.com:8080/npm/");
    }

    #[test]
    fn test_decode_password() {
        // "password" in base64
        let encoded = "cGFzc3dvcmQ=";
        assert_eq!(decode_password(encoded).unwrap(), "password");
    }

    #[test]
    fn test_parse_legacy_auth() {
        // "user:password" in base64
        let auth = "dXNlcjpwYXNzd29yZA==";
        let (username, password) = parse_legacy_auth(auth).unwrap();
        assert_eq!(username, "user");
        assert_eq!(password, "password");
    }

    #[test]
    fn test_parse_legacy_auth_with_colon_in_password() {
        // "user:pass:word" in base64
        let auth = "dXNlcjpwYXNzOndvcmQ=";
        let (username, password) = parse_legacy_auth(auth).unwrap();
        assert_eq!(username, "user");
        assert_eq!(password, "pass:word");
    }

    #[test]
    fn test_credentials_basic_auth_header() {
        let creds = Credentials::BasicAuth {
            username: "user".to_string(),
            password: "password".to_string(),
            cert: None,
        };
        assert_eq!(
            creds.basic_auth_header(),
            Some("dXNlcjpwYXNzd29yZA==".to_string())
        );
    }

    #[test]
    fn test_credentials_token() {
        let creds = Credentials::Token {
            token: "my-token".to_string(),
            cert: None,
        };
        assert_eq!(creds.token(), Some("my-token"));
        assert_eq!(creds.username_password(), None);
    }

    #[test]
    fn test_debug_redacts_token() {
        let creds = Credentials::Token {
            token: "super-secret-token".to_string(),
            cert: None,
        };
        let debug_output = format!("{:?}", creds);
        assert!(
            !debug_output.contains("super-secret-token"),
            "Debug output should not contain the actual token"
        );
        assert!(
            debug_output.contains("[REDACTED]"),
            "Debug output should show [REDACTED]"
        );
    }

    #[test]
    fn test_debug_redacts_basic_auth_password() {
        let creds = Credentials::BasicAuth {
            username: "myuser".to_string(),
            password: "super-secret-password".to_string(),
            cert: None,
        };
        let debug_output = format!("{:?}", creds);
        assert!(
            !debug_output.contains("super-secret-password"),
            "Debug output should not contain the actual password"
        );
        assert!(
            debug_output.contains("myuser"),
            "Debug output should still show username"
        );
        assert!(
            debug_output.contains("[REDACTED]"),
            "Debug output should show [REDACTED]"
        );
    }

    #[test]
    fn test_debug_redacts_legacy_auth() {
        let creds = Credentials::LegacyAuth {
            auth: "c2VjcmV0LWF1dGgtc3RyaW5n".to_string(),
            username: "legacyuser".to_string(),
            password: "legacy-secret-password".to_string(),
            cert: None,
        };
        let debug_output = format!("{:?}", creds);
        assert!(
            !debug_output.contains("c2VjcmV0LWF1dGgtc3RyaW5n"),
            "Debug output should not contain the auth string"
        );
        assert!(
            !debug_output.contains("legacy-secret-password"),
            "Debug output should not contain the password"
        );
        assert!(
            debug_output.contains("legacyuser"),
            "Debug output should still show username"
        );
    }
}
