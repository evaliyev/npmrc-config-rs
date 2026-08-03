//! INI parsing for .npmrc files.
//!
//! This module handles parsing .npmrc files which use a simplified INI format
//! with support for environment variable expansion.
//!
//! Note: We use a custom parser instead of standard INI libraries because
//! .npmrc files have special key formats (like `//registry.npmjs.org/:_authToken`)
//! that standard INI parsers may treat incorrectly as sections or comments.

use crate::env_replace::expand_env_vars;
use crate::error::Result;
use std::collections::HashMap;
use std::path::Path;

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
    // ponytail: Keep this scalar-only until the public API can represent arrays and sections.
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

        let (key, value) = match line.split_once('=') {
            Some((key, value)) => (parse_value(key), parse_value(value)),
            None if line.starts_with('[') => continue,
            None => (parse_value(line), "true".to_string()),
        };

        if !key.is_empty() {
            result.insert(key, expand_env_vars(&value));
        }
    }

    Ok(result)
}

/// Parse an npm INI scalar, removing quotes and unescaped inline comments.
fn parse_value(value: &str) -> String {
    let value = value.trim();

    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        return value[1..value.len() - 1].to_string();
    }

    let mut parsed = String::with_capacity(value.len());
    let mut escaped = false;

    for character in value.chars() {
        if escaped {
            if matches!(character, '\\' | ';' | '#') {
                parsed.push(character);
            } else {
                parsed.push('\\');
                parsed.push(character);
            }
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if matches!(character, ';' | '#') {
            break;
        } else {
            parsed.push(character);
        }
    }

    if escaped {
        parsed.push('\\');
    }

    parsed.trim().to_string()
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
    fn test_parse_quoted_values_and_inline_comments() {
        let content = r#"
quoted = "value ; with # markers"
single-quoted = 'another value'
commented = value ; comment
escaped = value\;still-value\#still-value
blank =
quoted-spaces = ' a '
mismatched-quote = "something'
flag
"quoted-key" = quoted-key-value
"#;
        let result = parse_npmrc(content, Path::new("test")).unwrap();

        assert_eq!(
            result.get("quoted"),
            Some(&"value ; with # markers".to_string())
        );
        assert_eq!(
            result.get("single-quoted"),
            Some(&"another value".to_string())
        );
        assert_eq!(result.get("commented"), Some(&"value".to_string()));
        assert_eq!(
            result.get("escaped"),
            Some(&"value;still-value#still-value".to_string())
        );
        assert_eq!(result.get("blank"), Some(&String::new()));
        assert_eq!(result.get("quoted-spaces"), Some(&" a ".to_string()));
        assert_eq!(
            result.get("mismatched-quote"),
            Some(&"\"something'".to_string())
        );
        assert_eq!(result.get("flag"), Some(&"true".to_string()));
        assert_eq!(
            result.get("quoted-key"),
            Some(&"quoted-key-value".to_string())
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
    fn test_parse_bool() {
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(parse_bool("TRUE"), Some(true));
        assert_eq!(parse_bool("false"), Some(false));
        assert_eq!(parse_bool("FALSE"), Some(false));
        assert_eq!(parse_bool("yes"), None);
        assert_eq!(parse_bool("1"), None);
    }
}
