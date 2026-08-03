//! Registry URL resolution tests for `src/registry.rs`.
//!
//! Upstream keeps this logic in `lib/definitions/definitions.js` (the
//! `registry` / `@scope:registry` handling); here it is a standalone module.

use npmrc_config_rs::registry::{extract_scope, parse_registry_url, scope_registry_key};

// =============================================================================
// Scope extraction
// =============================================================================

#[test]
fn test_extract_scope() {
    assert_eq!(extract_scope("@myorg/package"), Some("@myorg"));
    assert_eq!(extract_scope("@another/pkg"), Some("@another"));
    assert_eq!(extract_scope("@scope"), Some("@scope"));
    assert_eq!(extract_scope("@scope/nested/path"), Some("@scope"));
}

#[test]
fn test_extract_scope_unscoped_package() {
    assert_eq!(extract_scope("regular-package"), None);
    assert_eq!(extract_scope(""), None);
}

#[test]
fn test_scope_registry_key() {
    assert_eq!(scope_registry_key("@myorg"), "@myorg:registry");
    assert_eq!(scope_registry_key("@acme"), "@acme:registry");
}

// =============================================================================
// Registry URL normalization
// =============================================================================

#[test]
fn test_parse_registry_url_adds_trailing_slash() {
    let url = parse_registry_url("https://registry.npmjs.org").unwrap();
    assert_eq!(url.as_str(), "https://registry.npmjs.org/");
}

#[test]
fn test_parse_registry_url_keeps_trailing_slash() {
    let url = parse_registry_url("https://registry.npmjs.org/").unwrap();
    assert_eq!(url.as_str(), "https://registry.npmjs.org/");
}

#[test]
fn test_parse_registry_url_with_path_and_port() {
    let url = parse_registry_url("https://my-couch:5984/registry/_design/app/rewrite").unwrap();
    assert_eq!(
        url.as_str(),
        "https://my-couch:5984/registry/_design/app/rewrite/"
    );
}

#[test]
fn test_parse_registry_url_rejects_invalid_url() {
    // Upstream equivalent: `t.throws(() => nerfDart('not a valid url'))`
    assert!(parse_registry_url("not a valid url").is_err());
    assert!(parse_registry_url("").is_err());
}
