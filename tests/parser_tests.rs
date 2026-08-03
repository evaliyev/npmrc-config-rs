//! INI parsing tests for `src/parser.rs`.
//!
//! Upstream parses `.npmrc` with the `ini` package and then coerces values in
//! `lib/parse-field.js`. This crate exposes string values only, so the cases
//! here cover the parsing half (quoting, comments, odd keys) plus the one
//! coercion the public API keeps: `parse_bool`.

use npmrc_config_rs::{parse_bool, LoadOptions, NpmrcConfig};
use std::fs;
use tempfile::TempDir;

/// Load a project-level .npmrc with the given content.
fn load_npmrc(content: &str) -> (TempDir, NpmrcConfig) {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("package.json"), "{}").unwrap();
    fs::write(temp.path().join(".npmrc"), content).unwrap();

    let config = NpmrcConfig::load_with_options(LoadOptions {
        cwd: Some(temp.path().to_path_buf()),
        skip_user: true,
        skip_global: true,
        ..Default::default()
    })
    .unwrap();

    (temp, config)
}

// =============================================================================
// Comments and whitespace
// =============================================================================

#[test]
fn test_parse_comments() {
    let (_temp, config) = load_npmrc(
        r#"
# This is a comment
; This is also a comment
key = value
# Another comment
"#,
    );

    assert_eq!(config.get("key"), Some("value"));
}

#[test]
fn test_parse_empty_lines() {
    let (_temp, config) = load_npmrc(
        r#"

key1 = value1

key2 = value2

"#,
    );

    assert_eq!(config.get("key1"), Some("value1"));
    assert_eq!(config.get("key2"), Some("value2"));
}

#[test]
fn test_parse_no_spaces_around_equals() {
    let (_temp, config) = load_npmrc("key=value");
    assert_eq!(config.get("key"), Some("value"));
}

#[test]
fn test_parse_value_with_equals() {
    let (_temp, config) = load_npmrc("key = value=with=equals");
    assert_eq!(config.get("key"), Some("value=with=equals"));
}

#[test]
fn test_parse_whitespace_in_value() {
    let (_temp, config) = load_npmrc("key =   value with spaces   ");
    // Value should be trimmed
    assert_eq!(config.get("key"), Some("value with spaces"));
}

#[test]
fn test_parse_inline_comment_is_stripped() {
    let (_temp, config) =
        load_npmrc("key = value # trailing comment\nother = value ; also a comment");
    assert_eq!(config.get("key"), Some("value"));
    assert_eq!(config.get("other"), Some("value"));
}

#[test]
fn test_parse_escaped_comment_char_is_kept() {
    let (_temp, config) = load_npmrc(r"key = value\#notacomment");
    assert_eq!(config.get("key"), Some("value#notacomment"));
}

// =============================================================================
// Quoting
// =============================================================================

#[test]
fn test_parse_double_quoted_value() {
    let (_temp, config) = load_npmrc(r#"key = "  spaced # value  ""#);
    assert_eq!(config.get("key"), Some("  spaced # value  "));
}

#[test]
fn test_parse_single_quoted_value() {
    let (_temp, config) = load_npmrc("key = 'quoted value'");
    assert_eq!(config.get("key"), Some("quoted value"));
}

// =============================================================================
// Key shapes specific to .npmrc
// =============================================================================

#[test]
fn test_parse_key_without_value_is_true() {
    // Upstream: `parseField('', 'global')` -> true (boolean flag)
    let (_temp, config) = load_npmrc("global");
    assert_eq!(config.get("global"), Some("true"));
}

#[test]
fn test_parse_scoped_registry_key() {
    let (_temp, config) = load_npmrc("@myorg:registry = https://registry.myorg.com/");
    assert_eq!(
        config.get("@myorg:registry"),
        Some("https://registry.myorg.com/")
    );
}

#[test]
fn test_parse_nerf_darted_key() {
    let (_temp, config) = load_npmrc("//registry.npmjs.org/:_authToken = secret");
    assert_eq!(
        config.get("//registry.npmjs.org/:_authToken"),
        Some("secret")
    );
}

#[test]
fn test_parse_section_header_is_ignored() {
    // .npmrc has no sections; a stray header must not become a key
    let (_temp, config) = load_npmrc("[section]\nkey = value");
    assert_eq!(config.get("key"), Some("value"));
    assert_eq!(config.get("[section]"), None);
}

#[test]
fn test_parse_keys_are_case_sensitive() {
    let (_temp, config) = load_npmrc("Registry = https://upper.example.com/");
    assert_eq!(config.get("Registry"), Some("https://upper.example.com/"));
    assert_eq!(config.get("registry"), None);
}

// =============================================================================
// Value coercion
// =============================================================================

#[test]
fn test_parse_bool_values() {
    assert_eq!(parse_bool("true"), Some(true));
    assert_eq!(parse_bool("false"), Some(false));
    assert_eq!(parse_bool("TRUE"), Some(true));
    assert_eq!(parse_bool("False"), Some(false));
    assert_eq!(parse_bool("blerg"), None);
    assert_eq!(parse_bool(""), None);
}
