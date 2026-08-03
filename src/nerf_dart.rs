//! Nerf-darting: mapping a registry URL to a credential identifier.
//!
//! Rust port of `lib/nerf-dart.js` from @npmcli/config.

use url::Url;

/// Convert a registry URL to nerf-dart format for credential lookup.
///
/// Nerf-darting strips the protocol and normalizes the path to prevent
/// credentials from leaking across registries. This mirrors the upstream
/// implementation, which resolves `new URL('.', from)` against the
/// protocol/host/pathname of the input URL.
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
