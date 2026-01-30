//! Environment variable expansion tests.
//!
//! Based on test cases from @npmcli/config test/env-replace.js
//!
//! Note: Each test uses unique environment variable names to avoid
//! interference when tests run in parallel.

use npmrc_config_rs::expand_env_vars;

// =============================================================================
// Basic replacement
// =============================================================================

#[test]
fn test_replaces_defined_variable() {
    std::env::set_var("ENVTEST_FOO_1", "bar");
    assert_eq!(expand_env_vars("${ENVTEST_FOO_1}"), "bar");
    std::env::remove_var("ENVTEST_FOO_1");
}

#[test]
fn test_replaces_defined_variable_with_optional_modifier() {
    std::env::set_var("ENVTEST_FOO_2", "bar");
    assert_eq!(expand_env_vars("${ENVTEST_FOO_2?}"), "bar");
    std::env::remove_var("ENVTEST_FOO_2");
}

#[test]
fn test_replaces_multiple_defined_variables() {
    std::env::set_var("ENVTEST_FOO_3", "bar");
    std::env::set_var("ENVTEST_BAR_3", "baz");
    assert_eq!(
        expand_env_vars("${ENVTEST_FOO_3}${ENVTEST_BAR_3}"),
        "barbaz"
    );
    std::env::remove_var("ENVTEST_FOO_3");
    std::env::remove_var("ENVTEST_BAR_3");
}

#[test]
fn test_replaces_variable_with_surrounding_text() {
    std::env::set_var("ENVTEST_FOO_4", "bar");
    assert_eq!(
        expand_env_vars("prefix_${ENVTEST_FOO_4}_suffix"),
        "prefix_bar_suffix"
    );
    std::env::remove_var("ENVTEST_FOO_4");
}

// =============================================================================
// Undefined variables
// =============================================================================

#[test]
fn test_leaves_undefined_variable_unreplaced() {
    std::env::remove_var("ENVTEST_UNDEF_1");
    assert_eq!(expand_env_vars("${ENVTEST_UNDEF_1}"), "${ENVTEST_UNDEF_1}");
}

#[test]
fn test_undefined_variable_with_optional_modifier_becomes_empty() {
    std::env::remove_var("ENVTEST_UNDEF_2");
    assert_eq!(expand_env_vars("${ENVTEST_UNDEF_2?}"), "");
}

#[test]
fn test_mixed_defined_undefined_with_optional_modifier() {
    std::env::set_var("ENVTEST_FOO_5", "bar");
    std::env::remove_var("ENVTEST_BAZ_5");
    assert_eq!(expand_env_vars("${ENVTEST_FOO_5?}${ENVTEST_BAZ_5?}"), "bar");
    std::env::remove_var("ENVTEST_FOO_5");
}

// =============================================================================
// Escape sequences
// =============================================================================

#[test]
fn test_single_backslash_escapes_defined_variable() {
    std::env::set_var("ENVTEST_FOO_6", "bar");
    assert_eq!(expand_env_vars("\\${ENVTEST_FOO_6}"), "${ENVTEST_FOO_6}");
    std::env::remove_var("ENVTEST_FOO_6");
}

#[test]
fn test_double_backslash_allows_replacement() {
    std::env::set_var("ENVTEST_FOO_7", "bar");
    assert_eq!(expand_env_vars("\\\\${ENVTEST_FOO_7}"), "\\bar");
    std::env::remove_var("ENVTEST_FOO_7");
}

#[test]
fn test_triple_backslash_prevents_replacement() {
    std::env::set_var("ENVTEST_FOO_8", "bar");
    assert_eq!(
        expand_env_vars("\\\\\\${ENVTEST_FOO_8}"),
        "\\${ENVTEST_FOO_8}"
    );
    std::env::remove_var("ENVTEST_FOO_8");
}

#[test]
fn test_single_backslash_escapes_undefined_variable() {
    std::env::remove_var("ENVTEST_BAZ_9");
    assert_eq!(expand_env_vars("\\${ENVTEST_BAZ_9}"), "${ENVTEST_BAZ_9}");
}

#[test]
fn test_double_backslash_with_undefined_variable() {
    std::env::remove_var("ENVTEST_BAZ_10");
    assert_eq!(
        expand_env_vars("\\\\${ENVTEST_BAZ_10}"),
        "\\${ENVTEST_BAZ_10}"
    );
}

#[test]
fn test_single_backslash_escapes_optional_variable() {
    std::env::set_var("ENVTEST_FOO_11", "bar");
    assert_eq!(
        expand_env_vars("\\${ENVTEST_FOO_11?}"),
        "${ENVTEST_FOO_11?}"
    );
    std::env::remove_var("ENVTEST_FOO_11");
}

#[test]
fn test_double_backslash_allows_optional_replacement() {
    std::env::set_var("ENVTEST_FOO_12", "bar");
    assert_eq!(expand_env_vars("\\\\${ENVTEST_FOO_12?}"), "\\bar");
    std::env::remove_var("ENVTEST_FOO_12");
}

#[test]
fn test_single_backslash_escapes_undefined_optional_variable() {
    std::env::remove_var("ENVTEST_BAZ_13");
    assert_eq!(
        expand_env_vars("\\${ENVTEST_BAZ_13?}"),
        "${ENVTEST_BAZ_13?}"
    );
}

#[test]
fn test_double_backslash_with_undefined_optional_variable() {
    std::env::remove_var("ENVTEST_BAZ_14");
    assert_eq!(expand_env_vars("\\\\${ENVTEST_BAZ_14?}"), "\\");
}

// =============================================================================
// Complex cases
// =============================================================================

#[test]
fn test_multiple_variables_with_text() {
    std::env::set_var("ENVTEST_A_15", "hello");
    std::env::set_var("ENVTEST_B_15", "world");
    assert_eq!(
        expand_env_vars("${ENVTEST_A_15} ${ENVTEST_B_15}!"),
        "hello world!"
    );
    std::env::remove_var("ENVTEST_A_15");
    std::env::remove_var("ENVTEST_B_15");
}

#[test]
fn test_nested_braces_pattern() {
    std::env::set_var("ENVTEST_VAR_16", "value");
    assert_eq!(
        expand_env_vars("prefix${ENVTEST_VAR_16}suffix"),
        "prefixvaluesuffix"
    );
    std::env::remove_var("ENVTEST_VAR_16");
}

#[test]
fn test_empty_value() {
    std::env::set_var("ENVTEST_EMPTY_17", "");
    assert_eq!(expand_env_vars("${ENVTEST_EMPTY_17}"), "");
    std::env::remove_var("ENVTEST_EMPTY_17");
}

#[test]
fn test_value_with_special_characters() {
    std::env::set_var("ENVTEST_SPECIAL_18", "foo=bar&baz");
    assert_eq!(expand_env_vars("${ENVTEST_SPECIAL_18}"), "foo=bar&baz");
    std::env::remove_var("ENVTEST_SPECIAL_18");
}

#[test]
fn test_no_variables() {
    assert_eq!(expand_env_vars("no variables here"), "no variables here");
}

#[test]
fn test_incomplete_variable_syntax() {
    assert_eq!(expand_env_vars("${incomplete"), "${incomplete");
}

#[test]
fn test_dollar_without_braces() {
    assert_eq!(expand_env_vars("$ENVTEST_VAR"), "$ENVTEST_VAR");
}

// =============================================================================
// Real-world npmrc patterns
// =============================================================================

#[test]
fn test_auth_token_pattern() {
    std::env::set_var("ENVTEST_NPM_TOKEN_19", "npm_abc123");
    assert_eq!(
        expand_env_vars("//registry.npmjs.org/:_authToken=${ENVTEST_NPM_TOKEN_19}"),
        "//registry.npmjs.org/:_authToken=npm_abc123"
    );
    std::env::remove_var("ENVTEST_NPM_TOKEN_19");
}

#[test]
fn test_registry_url_pattern() {
    std::env::set_var("ENVTEST_REGISTRY_HOST_20", "npm.mycompany.com");
    assert_eq!(
        expand_env_vars("registry=https://${ENVTEST_REGISTRY_HOST_20}/"),
        "registry=https://npm.mycompany.com/"
    );
    std::env::remove_var("ENVTEST_REGISTRY_HOST_20");
}

#[test]
fn test_optional_token_when_not_set() {
    std::env::remove_var("ENVTEST_NPM_TOKEN_21");
    assert_eq!(
        expand_env_vars("//registry.npmjs.org/:_authToken=${ENVTEST_NPM_TOKEN_21?}"),
        "//registry.npmjs.org/:_authToken="
    );
}
