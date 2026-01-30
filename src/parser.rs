//! INI parsing for .npmrc files.
//!
//! This module handles parsing .npmrc files which use a simplified INI format
//! with support for environment variable expansion.
//!
//! Note: We use a custom parser instead of standard INI libraries because
//! .npmrc files have special key formats (like `//registry.npmjs.org/:_authToken`)
//! that standard INI parsers may treat incorrectly as sections or comments.

use crate::error::Result;
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

/// Regex for matching environment variable references: `${VAR}` or `${VAR?}`
/// The `?` modifier makes undefined variables expand to empty string instead of keeping the literal.
/// Supports escaping with backslashes.
static ENV_EXPR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?P<esc>\\*)\$\{(?P<name>[^${}?]+)(?P<mod>\?)?\}").unwrap());

/// Parse .npmrc INI content into key-value pairs.
///
/// The parser handles:
/// - Standard INI key=value pairs
/// - Comments starting with `#` or `;`
/// - Scoped registry keys like `@myorg:registry`
/// - Nerf-darted auth keys like `//registry.npmjs.org/:_authToken`
///
/// Unlike standard INI files, .npmrc files:
/// - Don't use sections (no `[section]` headers)
/// - Allow keys starting with special characters like `@` and `//`
pub fn parse_npmrc(content: &str, _path: &Path) -> Result<HashMap<String, String>> {
    let mut result = HashMap::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // Skip comments (lines starting with # or ;)
        if line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        // Parse key=value or key = value
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim();
            let value = line[eq_pos + 1..].trim();

            // Skip empty keys
            if key.is_empty() {
                continue;
            }

            let expanded = expand_env_vars(value);
            result.insert(key.to_string(), expanded);
        }
        // Lines without = are ignored (npm's ini parser also ignores them)
    }

    Ok(result)
}

/// Expand `${VAR}` environment variable references in a value.
///
/// - `${VAR}` - Expands to the value of VAR, or keeps `${VAR}` literal if undefined
/// - `${VAR?}` - Expands to the value of VAR, or empty string if undefined
/// - `\\${VAR}` - Escaped, keeps the literal (with one less backslash)
pub fn expand_env_vars(value: &str) -> String {
    ENV_EXPR
        .replace_all(value, |caps: &regex::Captures| {
            let esc = caps.name("esc").map_or("", |m| m.as_str());
            let name = caps.name("name").map_or("", |m| m.as_str());
            let modifier = caps.name("mod").map_or("", |m| m.as_str());

            // Handle escape sequences
            let esc_len = esc.len();
            if esc_len % 2 == 1 {
                // Odd number of backslashes means the $ is escaped
                // Return half the backslashes (rounded down) plus the literal variable syntax
                let kept_esc = &esc[..(esc_len / 2)];
                // Preserve the original literal including modifier
                let literal = format!("${{{}{}}}", name, modifier);
                return format!("{}{}", kept_esc, literal);
            }

            // Even number of backslashes (including 0) - expand the variable
            let kept_esc = &esc[..(esc_len / 2)];
            let val = match std::env::var(name) {
                Ok(v) => v,
                Err(_) => {
                    if modifier == "?" {
                        String::new()
                    } else {
                        format!("${{{}}}", name)
                    }
                }
            };

            format!("{}{}", kept_esc, val)
        })
        .into_owned()
}

/// Parse a boolean value from a string.
///
/// Returns `Some(true)` for "true", `Some(false)` for "false", and `None` for other values.
pub fn parse_bool(value: &str) -> Option<bool> {
    match value.to_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_ini() {
        let content = r#"
registry = https://registry.npmjs.org/
strict-ssl = true
"#;
        let result = parse_npmrc(content, Path::new("test")).unwrap();
        assert_eq!(
            result.get("registry"),
            Some(&"https://registry.npmjs.org/".to_string())
        );
        assert_eq!(result.get("strict-ssl"), Some(&"true".to_string()));
    }

    #[test]
    fn test_parse_scoped_registry() {
        let content = r#"
@myorg:registry = https://registry.mycorp.com/
"#;
        let result = parse_npmrc(content, Path::new("test")).unwrap();
        assert_eq!(
            result.get("@myorg:registry"),
            Some(&"https://registry.mycorp.com/".to_string())
        );
    }

    #[test]
    fn test_parse_nerf_darted_auth() {
        let content = r#"
//registry.npmjs.org/:_authToken = token123
//registry.mycorp.com/:username = myuser
//registry.mycorp.com/:_password = cGFzc3dvcmQ=
"#;
        let result = parse_npmrc(content, Path::new("test")).unwrap();
        assert_eq!(
            result.get("//registry.npmjs.org/:_authToken"),
            Some(&"token123".to_string())
        );
        assert_eq!(
            result.get("//registry.mycorp.com/:username"),
            Some(&"myuser".to_string())
        );
        assert_eq!(
            result.get("//registry.mycorp.com/:_password"),
            Some(&"cGFzc3dvcmQ=".to_string())
        );
    }

    #[test]
    fn test_parse_comments() {
        let content = r#"
# This is a comment
; This is also a comment
registry = https://registry.npmjs.org/
"#;
        let result = parse_npmrc(content, Path::new("test")).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result.get("registry"),
            Some(&"https://registry.npmjs.org/".to_string())
        );
    }

    #[test]
    fn test_parse_no_spaces() {
        let content = "registry=https://registry.npmjs.org/";
        let result = parse_npmrc(content, Path::new("test")).unwrap();
        assert_eq!(
            result.get("registry"),
            Some(&"https://registry.npmjs.org/".to_string())
        );
    }

    #[test]
    fn test_parse_value_with_equals() {
        let content = "key = value=with=equals";
        let result = parse_npmrc(content, Path::new("test")).unwrap();
        assert_eq!(result.get("key"), Some(&"value=with=equals".to_string()));
    }

    #[test]
    fn test_expand_env_vars() {
        std::env::set_var("TEST_VAR", "test_value");

        assert_eq!(expand_env_vars("${TEST_VAR}"), "test_value");
        assert_eq!(
            expand_env_vars("prefix_${TEST_VAR}_suffix"),
            "prefix_test_value_suffix"
        );

        std::env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_expand_env_vars_undefined() {
        std::env::remove_var("UNDEFINED_VAR");

        // Without modifier - keeps literal
        assert_eq!(expand_env_vars("${UNDEFINED_VAR}"), "${UNDEFINED_VAR}");

        // With ? modifier - expands to empty
        assert_eq!(expand_env_vars("${UNDEFINED_VAR?}"), "");
    }

    #[test]
    fn test_expand_env_vars_escaped() {
        std::env::set_var("TEST_VAR2", "value");

        // Single backslash escapes
        assert_eq!(expand_env_vars("\\${TEST_VAR2}"), "${TEST_VAR2}");

        // Double backslash - one backslash kept, var expanded
        assert_eq!(expand_env_vars("\\\\${TEST_VAR2}"), "\\value");

        std::env::remove_var("TEST_VAR2");
    }

    #[test]
    fn test_parse_bool() {
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(parse_bool("TRUE"), Some(true));
        assert_eq!(parse_bool("false"), Some(false));
        assert_eq!(parse_bool("FALSE"), Some(false));
        assert_eq!(parse_bool("yes"), None);
        assert_eq!(parse_bool("1"), None);
    }
}
