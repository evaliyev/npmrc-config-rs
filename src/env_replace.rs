//! Environment variable expansion in config values.
//!
//! Rust port of `lib/env-replace.js` from @npmcli/config.

use regex::Regex;
use std::sync::LazyLock;

/// Regex for matching environment variable references: `${VAR}` or `${VAR?}`
/// The `?` modifier makes undefined variables expand to empty string instead of keeping the literal.
/// Supports escaping with backslashes.
static ENV_EXPR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?P<esc>\\*)\$\{(?P<name>[^${}?]+)(?P<mod>\?)?\}").unwrap());

/// Expand `${VAR}` environment variable references in a value.
///
/// - `${VAR}` - Expands to the value of VAR, or keeps `${VAR}` literal if undefined
/// - `${VAR?}` - Expands to the value of VAR, or empty string if undefined
/// - `\\${VAR}` - Escaped, keeps the literal (with one less backslash)
///
/// Unlike upstream, which is handed an env object, this reads the process
/// environment directly.
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
