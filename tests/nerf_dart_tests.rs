//! Nerf-dart URL transformation tests.
//!
//! Based on test cases from @npmcli/config test/nerf-dart.js

use npmrc_config_rs::nerf_dart;
use url::Url;

/// Helper to test nerf-dart transformation
fn assert_nerf_dart(input: &str, expected: &str) {
    let url = Url::parse(input).unwrap();
    assert_eq!(
        nerf_dart(&url),
        expected,
        "nerf_dart({}) should be {}",
        input,
        expected
    );
}

// =============================================================================
// Basic registry URLs (registry.npmjs.org)
// =============================================================================

#[test]
fn test_registry_npmjs_org_base() {
    assert_nerf_dart("https://registry.npmjs.org", "//registry.npmjs.org/");
}

#[test]
fn test_registry_npmjs_org_trailing_slash() {
    assert_nerf_dart("https://registry.npmjs.org/", "//registry.npmjs.org/");
}

#[test]
fn test_registry_npmjs_org_with_package() {
    assert_nerf_dart(
        "https://registry.npmjs.org/package-name",
        "//registry.npmjs.org/",
    );
}

#[test]
fn test_registry_npmjs_org_with_query() {
    assert_nerf_dart(
        "https://registry.npmjs.org/package-name?write=true",
        "//registry.npmjs.org/",
    );
}

#[test]
fn test_registry_npmjs_org_with_scoped_package() {
    assert_nerf_dart(
        "https://registry.npmjs.org/@scope%2fpackage-name",
        "//registry.npmjs.org/",
    );
}

#[test]
fn test_registry_npmjs_org_with_scoped_package_and_query() {
    assert_nerf_dart(
        "https://registry.npmjs.org/@scope%2fpackage-name?write=true",
        "//registry.npmjs.org/",
    );
}

#[test]
fn test_registry_npmjs_org_with_credentials_in_url() {
    assert_nerf_dart(
        "https://username:password@registry.npmjs.org/package-name?write=true",
        "//registry.npmjs.org/",
    );
}

#[test]
fn test_registry_npmjs_org_with_hash() {
    assert_nerf_dart("https://registry.npmjs.org/#hash", "//registry.npmjs.org/");
}

#[test]
fn test_registry_npmjs_org_with_query_and_hash() {
    assert_nerf_dart(
        "https://registry.npmjs.org/?write=true#hash",
        "//registry.npmjs.org/",
    );
}

#[test]
fn test_registry_npmjs_org_with_package_query_and_hash() {
    assert_nerf_dart(
        "https://registry.npmjs.org/package-name?write=true#hash",
        "//registry.npmjs.org/",
    );
}

#[test]
fn test_registry_npmjs_org_with_package_and_hash() {
    assert_nerf_dart(
        "https://registry.npmjs.org/package-name#hash",
        "//registry.npmjs.org/",
    );
}

#[test]
fn test_registry_npmjs_org_scoped_with_query_and_hash() {
    assert_nerf_dart(
        "https://registry.npmjs.org/@scope%2fpackage-name?write=true#hash",
        "//registry.npmjs.org/",
    );
}

#[test]
fn test_registry_npmjs_org_scoped_with_hash() {
    assert_nerf_dart(
        "https://registry.npmjs.org/@scope%2fpackage-name#hash",
        "//registry.npmjs.org/",
    );
}

// =============================================================================
// Custom registry with port and path (CouchDB-style)
// =============================================================================

#[test]
fn test_couch_registry_base() {
    assert_nerf_dart(
        "https://my-couch:5984/registry/_design/app/rewrite/",
        "//my-couch:5984/registry/_design/app/rewrite/",
    );
}

#[test]
fn test_couch_registry_with_package() {
    assert_nerf_dart(
        "https://my-couch:5984/registry/_design/app/rewrite/package-name",
        "//my-couch:5984/registry/_design/app/rewrite/",
    );
}

#[test]
fn test_couch_registry_with_query() {
    assert_nerf_dart(
        "https://my-couch:5984/registry/_design/app/rewrite/package-name?write=true",
        "//my-couch:5984/registry/_design/app/rewrite/",
    );
}

#[test]
fn test_couch_registry_with_scoped_package() {
    assert_nerf_dart(
        "https://my-couch:5984/registry/_design/app/rewrite/@scope%2fpackage-name",
        "//my-couch:5984/registry/_design/app/rewrite/",
    );
}

#[test]
fn test_couch_registry_with_scoped_package_and_query() {
    assert_nerf_dart(
        "https://my-couch:5984/registry/_design/app/rewrite/@scope%2fpackage-name?write=true",
        "//my-couch:5984/registry/_design/app/rewrite/",
    );
}

#[test]
fn test_couch_registry_with_credentials() {
    assert_nerf_dart(
        "https://username:password@my-couch:5984/registry/_design/app/rewrite/package-name?write=true",
        "//my-couch:5984/registry/_design/app/rewrite/",
    );
}

#[test]
fn test_couch_registry_with_hash() {
    assert_nerf_dart(
        "https://my-couch:5984/registry/_design/app/rewrite/#hash",
        "//my-couch:5984/registry/_design/app/rewrite/",
    );
}

#[test]
fn test_couch_registry_with_query_and_hash() {
    assert_nerf_dart(
        "https://my-couch:5984/registry/_design/app/rewrite/?write=true#hash",
        "//my-couch:5984/registry/_design/app/rewrite/",
    );
}

#[test]
fn test_couch_registry_with_package_query_and_hash() {
    assert_nerf_dart(
        "https://my-couch:5984/registry/_design/app/rewrite/package-name?write=true#hash",
        "//my-couch:5984/registry/_design/app/rewrite/",
    );
}

#[test]
fn test_couch_registry_with_package_and_hash() {
    assert_nerf_dart(
        "https://my-couch:5984/registry/_design/app/rewrite/package-name#hash",
        "//my-couch:5984/registry/_design/app/rewrite/",
    );
}

#[test]
fn test_couch_registry_scoped_with_query_and_hash() {
    assert_nerf_dart(
        "https://my-couch:5984/registry/_design/app/rewrite/@scope%2fpackage-name?write=true#hash",
        "//my-couch:5984/registry/_design/app/rewrite/",
    );
}

#[test]
fn test_couch_registry_scoped_with_hash() {
    assert_nerf_dart(
        "https://my-couch:5984/registry/_design/app/rewrite/@scope%2fpackage-name#hash",
        "//my-couch:5984/registry/_design/app/rewrite/",
    );
}

// =============================================================================
// Edge cases
// =============================================================================

#[test]
fn test_http_protocol() {
    assert_nerf_dart("http://registry.example.com/", "//registry.example.com/");
}

#[test]
fn test_custom_port() {
    assert_nerf_dart(
        "https://registry.example.com:8080/",
        "//registry.example.com:8080/",
    );
}

#[test]
fn test_simple_path() {
    assert_nerf_dart("https://example.com/npm/", "//example.com/npm/");
}

#[test]
fn test_path_without_trailing_slash() {
    assert_nerf_dart("https://example.com/npm", "//example.com/");
}

#[test]
fn test_deeply_nested_path() {
    assert_nerf_dart("https://example.com/a/b/c/d/", "//example.com/a/b/c/d/");
}

#[test]
fn test_root_path_only() {
    assert_nerf_dart("https://example.com", "//example.com/");
}
