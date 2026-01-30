//! Registry URL resolution for npm packages.
//!
//! This module handles resolving the correct registry URL for packages,
//! including support for scoped registries.

use url::Url;

/// The default npm registry URL.
pub const DEFAULT_REGISTRY: &str = "https://registry.npmjs.org/";

/// Extract the scope from a package name if present.
///
/// # Examples
///
/// ```
/// use npmrc_config_rs::registry::extract_scope;
///
/// assert_eq!(extract_scope("@myorg/package"), Some("@myorg"));
/// assert_eq!(extract_scope("package"), None);
/// assert_eq!(extract_scope("@scope/nested/path"), Some("@scope"));
/// ```
pub fn extract_scope(package: &str) -> Option<&str> {
    if package.starts_with('@') {
        // Find the end of the scope (either '/' or end of string)
        let end = package.find('/').unwrap_or(package.len());
        Some(&package[..end])
    } else {
        None
    }
}

/// Build the config key for a scoped registry.
///
/// # Examples
///
/// ```
/// use npmrc_config_rs::registry::scope_registry_key;
///
/// assert_eq!(scope_registry_key("@myorg"), "@myorg:registry");
/// ```
pub fn scope_registry_key(scope: &str) -> String {
    format!("{}:registry", scope)
}

/// Parse a registry URL, ensuring it has a trailing slash.
pub fn parse_registry_url(url: &str) -> Result<Url, url::ParseError> {
    let normalized = if url.ends_with('/') {
        url.to_string()
    } else {
        format!("{}/", url)
    };
    Url::parse(&normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_scope() {
        assert_eq!(extract_scope("@myorg/package"), Some("@myorg"));
        assert_eq!(extract_scope("@another/pkg"), Some("@another"));
        assert_eq!(extract_scope("regular-package"), None);
        assert_eq!(extract_scope("@scope"), Some("@scope"));
    }

    #[test]
    fn test_scope_registry_key() {
        assert_eq!(scope_registry_key("@myorg"), "@myorg:registry");
        assert_eq!(scope_registry_key("@acme"), "@acme:registry");
    }

    #[test]
    fn test_parse_registry_url() {
        let url = parse_registry_url("https://registry.npmjs.org").unwrap();
        assert_eq!(url.as_str(), "https://registry.npmjs.org/");

        let url = parse_registry_url("https://registry.npmjs.org/").unwrap();
        assert_eq!(url.as_str(), "https://registry.npmjs.org/");
    }
}
