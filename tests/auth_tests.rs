//! Credential tests for `src/auth.rs`.
//!
//! Mirrors the `credentials management` subtests of `test/index.js` in
//! @npmcli/config, which cover nerf-darted auth lookup and its fixtures.

use npmrc_config_rs::{Credentials, LoadOptions, NpmrcConfig};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use url::Url;

/// Helper to create a test environment with a specific .npmrc content
fn setup_config(npmrc_content: &str) -> (TempDir, NpmrcConfig) {
    let temp = TempDir::new().unwrap();
    let project_dir = temp.path();

    fs::write(project_dir.join("package.json"), "{}").unwrap();
    fs::write(project_dir.join(".npmrc"), npmrc_content).unwrap();

    let config = NpmrcConfig::load_with_options(LoadOptions {
        cwd: Some(project_dir.to_path_buf()),
        skip_user: true,
        skip_global: true,
        ..Default::default()
    })
    .unwrap();

    (temp, config)
}

// =============================================================================
// Token authentication (_authToken)
// =============================================================================

#[test]
fn test_token_auth_basic() {
    let (_temp, config) = setup_config("//registry.npmjs.org/:_authToken = npm_abc123xyz");

    let registry = Url::parse("https://registry.npmjs.org/").unwrap();
    let creds = config.credentials_for(&registry).unwrap();

    match creds {
        Credentials::Token { token, cert } => {
            assert_eq!(token, "npm_abc123xyz");
            assert!(cert.is_none());
        }
        _ => panic!("Expected Token credentials"),
    }
}

#[test]
fn test_token_auth_with_package_url() {
    let (_temp, config) = setup_config("//registry.npmjs.org/:_authToken = secret-token");

    // URL with package path should still find credentials
    let registry = Url::parse("https://registry.npmjs.org/some-package").unwrap();
    let creds = config.credentials_for(&registry).unwrap();

    assert!(matches!(creds, Credentials::Token { .. }));
}

#[test]
fn test_token_auth_custom_registry() {
    let (_temp, config) = setup_config("//npm.mycompany.com/:_authToken = corp-token-123");

    let registry = Url::parse("https://npm.mycompany.com/").unwrap();
    let creds = config.credentials_for(&registry).unwrap();

    match creds {
        Credentials::Token { token, .. } => {
            assert_eq!(token, "corp-token-123");
        }
        _ => panic!("Expected Token credentials"),
    }
}

#[test]
fn test_token_auth_with_port() {
    let (_temp, config) = setup_config("//registry.example.com:8080/:_authToken = port-token");

    let registry = Url::parse("https://registry.example.com:8080/").unwrap();
    let creds = config.credentials_for(&registry).unwrap();

    match creds {
        Credentials::Token { token, .. } => {
            assert_eq!(token, "port-token");
        }
        _ => panic!("Expected Token credentials"),
    }
}

#[test]
fn test_token_auth_with_path() {
    let (_temp, config) = setup_config("//registry.example.com/npm/:_authToken = path-token");

    let registry = Url::parse("https://registry.example.com/npm/").unwrap();
    let creds = config.credentials_for(&registry).unwrap();

    match creds {
        Credentials::Token { token, .. } => {
            assert_eq!(token, "path-token");
        }
        _ => panic!("Expected Token credentials"),
    }
}

// =============================================================================
// Username/password authentication
// =============================================================================

#[test]
fn test_basic_auth() {
    // "mypassword" base64 encoded
    let (_temp, config) = setup_config(
        r#"
//registry.example.com/:username = myuser
//registry.example.com/:_password = bXlwYXNzd29yZA==
"#,
    );

    let registry = Url::parse("https://registry.example.com/").unwrap();
    let creds = config.credentials_for(&registry).unwrap();

    match creds {
        Credentials::BasicAuth {
            username,
            password,
            cert,
        } => {
            assert_eq!(username, "myuser");
            assert_eq!(password, "mypassword");
            assert!(cert.is_none());
        }
        _ => panic!("Expected BasicAuth credentials"),
    }
}

#[test]
fn test_basic_auth_password_with_special_chars() {
    // "p@ss:word!" base64 encoded = "cEBzczp3b3JkIQ=="
    let (_temp, config) = setup_config(
        r#"
//registry.example.com/:username = admin
//registry.example.com/:_password = cEBzczp3b3JkIQ==
"#,
    );

    let registry = Url::parse("https://registry.example.com/").unwrap();
    let creds = config.credentials_for(&registry).unwrap();

    match creds {
        Credentials::BasicAuth {
            username, password, ..
        } => {
            assert_eq!(username, "admin");
            assert_eq!(password, "p@ss:word!");
        }
        _ => panic!("Expected BasicAuth credentials"),
    }
}

#[test]
fn test_basic_auth_missing_password() {
    let (_temp, config) = setup_config("//registry.example.com/:username = orphan-user");

    let registry = Url::parse("https://registry.example.com/").unwrap();
    // Should not return credentials if password is missing
    assert!(config.credentials_for(&registry).is_none());
}

#[test]
fn test_basic_auth_missing_username() {
    let (_temp, config) = setup_config("//registry.example.com/:_password = b3JwaGFuLXBhc3M=");

    let registry = Url::parse("https://registry.example.com/").unwrap();
    // Should not return credentials if username is missing
    assert!(config.credentials_for(&registry).is_none());
}

#[test]
fn test_top_level_auth_is_not_used_for_registry_credentials() {
    let registry = Url::parse("https://registry.example.com/").unwrap();

    for npmrc in [
        "_auth = dXNlcjpwYXNz",
        "_authToken = token",
        "username = user\n_password = cGFzcw==",
        "certfile = /path/to/cert\nkeyfile = /path/to/key",
    ] {
        let (_temp, config) = setup_config(npmrc);
        assert!(config.credentials_for(&registry).is_none());
    }
}

// =============================================================================
// Legacy _auth authentication
// =============================================================================

#[test]
fn test_legacy_auth() {
    // "user:password" base64 encoded = "dXNlcjpwYXNzd29yZA=="
    let (_temp, config) = setup_config("//registry.example.com/:_auth = dXNlcjpwYXNzd29yZA==");

    let registry = Url::parse("https://registry.example.com/").unwrap();
    let creds = config.credentials_for(&registry).unwrap();

    match creds {
        Credentials::LegacyAuth {
            auth,
            username,
            password,
            cert,
        } => {
            assert_eq!(auth, "dXNlcjpwYXNzd29yZA==");
            assert_eq!(username, "user");
            assert_eq!(password, "password");
            assert!(cert.is_none());
        }
        _ => panic!("Expected LegacyAuth credentials"),
    }
}

#[test]
fn test_legacy_auth_with_colon_in_password() {
    // "user:pass:word:colon" base64 encoded = "dXNlcjpwYXNzOndvcmQ6Y29sb24="
    let (_temp, config) =
        setup_config("//registry.example.com/:_auth = dXNlcjpwYXNzOndvcmQ6Y29sb24=");

    let registry = Url::parse("https://registry.example.com/").unwrap();
    let creds = config.credentials_for(&registry).unwrap();

    match creds {
        Credentials::LegacyAuth {
            username, password, ..
        } => {
            assert_eq!(username, "user");
            assert_eq!(password, "pass:word:colon");
        }
        _ => panic!("Expected LegacyAuth credentials"),
    }
}

// =============================================================================
// Client certificate authentication (mTLS)
// =============================================================================

#[test]
fn test_client_cert_only() {
    let (_temp, config) = setup_config(
        r#"
//mtls.example.com/:certfile = /path/to/client.crt
//mtls.example.com/:keyfile = /path/to/client.key
"#,
    );

    let registry = Url::parse("https://mtls.example.com/").unwrap();
    let creds = config.credentials_for(&registry).unwrap();

    match creds {
        Credentials::ClientCertOnly(cert) => {
            assert_eq!(cert.certfile.to_str().unwrap(), "/path/to/client.crt");
            assert_eq!(cert.keyfile.to_str().unwrap(), "/path/to/client.key");
        }
        _ => panic!("Expected ClientCertOnly credentials"),
    }
}

#[test]
fn test_client_cert_missing_keyfile() {
    let (_temp, config) = setup_config("//mtls.example.com/:certfile = /path/to/client.crt");

    let registry = Url::parse("https://mtls.example.com/").unwrap();
    // Should not return credentials if keyfile is missing
    assert!(config.credentials_for(&registry).is_none());
}

#[test]
fn test_client_cert_with_tilde_expansion() {
    let (_temp, config) = setup_config(
        r#"
//mtls.example.com/:certfile = ~/.ssl/client.crt
//mtls.example.com/:keyfile = ~/.ssl/client.key
"#,
    );

    let registry = Url::parse("https://mtls.example.com/").unwrap();
    let creds = config.credentials_for(&registry).unwrap();

    match creds {
        Credentials::ClientCertOnly(cert) => {
            // Should have expanded ~ to home directory
            assert!(!cert.certfile.to_str().unwrap().starts_with("~"));
            assert!(cert.certfile.to_str().unwrap().ends_with(".ssl/client.crt"));
        }
        _ => panic!("Expected ClientCertOnly credentials"),
    }
}

// =============================================================================
// Combined authentication (token + cert, basic + cert)
// =============================================================================

#[test]
fn test_token_with_client_cert() {
    let (_temp, config) = setup_config(
        r#"
//secure.example.com/:_authToken = secure-token
//secure.example.com/:certfile = /path/to/cert.pem
//secure.example.com/:keyfile = /path/to/key.pem
"#,
    );

    let registry = Url::parse("https://secure.example.com/").unwrap();
    let creds = config.credentials_for(&registry).unwrap();

    match creds {
        Credentials::Token { token, cert } => {
            assert_eq!(token, "secure-token");
            let cert = cert.expect("Should have cert");
            assert_eq!(cert.certfile.to_str().unwrap(), "/path/to/cert.pem");
            assert_eq!(cert.keyfile.to_str().unwrap(), "/path/to/key.pem");
        }
        _ => panic!("Expected Token credentials with cert"),
    }
}

#[test]
fn test_basic_auth_with_client_cert() {
    let (_temp, config) = setup_config(
        r#"
//secure.example.com/:username = mtls-user
//secure.example.com/:_password = cGFzc3dvcmQ=
//secure.example.com/:certfile = /path/to/cert.pem
//secure.example.com/:keyfile = /path/to/key.pem
"#,
    );

    let registry = Url::parse("https://secure.example.com/").unwrap();
    let creds = config.credentials_for(&registry).unwrap();

    match creds {
        Credentials::BasicAuth { username, cert, .. } => {
            assert_eq!(username, "mtls-user");
            assert!(cert.is_some());
        }
        _ => panic!("Expected BasicAuth credentials with cert"),
    }
}

// =============================================================================
// Authentication priority (token > basic > legacy)
// =============================================================================

#[test]
fn test_token_takes_priority_over_basic() {
    let (_temp, config) = setup_config(
        r#"
//registry.example.com/:_authToken = priority-token
//registry.example.com/:username = ignored-user
//registry.example.com/:_password = aWdub3JlZA==
"#,
    );

    let registry = Url::parse("https://registry.example.com/").unwrap();
    let creds = config.credentials_for(&registry).unwrap();

    // Token should win
    assert!(matches!(creds, Credentials::Token { .. }));
}

#[test]
fn test_token_takes_priority_over_legacy() {
    let (_temp, config) = setup_config(
        r#"
//registry.example.com/:_authToken = priority-token
//registry.example.com/:_auth = aWdub3JlZDppZ25vcmVk
"#,
    );

    let registry = Url::parse("https://registry.example.com/").unwrap();
    let creds = config.credentials_for(&registry).unwrap();

    // Token should win
    assert!(matches!(creds, Credentials::Token { .. }));
}

#[test]
fn test_basic_takes_priority_over_legacy() {
    let (_temp, config) = setup_config(
        r#"
//registry.example.com/:username = priority-user
//registry.example.com/:_password = cHJpb3JpdHk=
//registry.example.com/:_auth = aWdub3JlZDppZ25vcmVk
"#,
    );

    let registry = Url::parse("https://registry.example.com/").unwrap();
    let creds = config.credentials_for(&registry).unwrap();

    // BasicAuth should win over LegacyAuth
    match creds {
        Credentials::BasicAuth { username, .. } => {
            assert_eq!(username, "priority-user");
        }
        _ => panic!("Expected BasicAuth credentials"),
    }
}

// =============================================================================
// Multiple registries
// =============================================================================

#[test]
fn test_multiple_registries_different_auth() {
    let (_temp, config) = setup_config(
        r#"
//registry.npmjs.org/:_authToken = npm-token
//npm.mycompany.com/:username = corp-user
//npm.mycompany.com/:_password = Y29ycC1wYXNz
//private.example.com/:_auth = cHJpdmF0ZTphdXRo
"#,
    );

    // Check npm registry
    let npm = Url::parse("https://registry.npmjs.org/").unwrap();
    assert!(matches!(
        config.credentials_for(&npm),
        Some(Credentials::Token { .. })
    ));

    // Check company registry
    let corp = Url::parse("https://npm.mycompany.com/").unwrap();
    assert!(matches!(
        config.credentials_for(&corp),
        Some(Credentials::BasicAuth { .. })
    ));

    // Check private registry
    let private = Url::parse("https://private.example.com/").unwrap();
    assert!(matches!(
        config.credentials_for(&private),
        Some(Credentials::LegacyAuth { .. })
    ));
}

#[test]
fn test_no_credentials_for_unconfigured_registry() {
    let (_temp, config) = setup_config("//registry.npmjs.org/:_authToken = npm-token");

    let other = Url::parse("https://other.example.com/").unwrap();
    assert!(config.credentials_for(&other).is_none());
}

// =============================================================================
// Credential helper methods
// =============================================================================

#[test]
fn test_token_helper() {
    let creds = Credentials::Token {
        token: "my-token".to_string(),
        cert: None,
    };
    assert_eq!(creds.token(), Some("my-token"));
    assert!(creds.username_password().is_none());
    assert!(creds.basic_auth_header().is_none());
}

#[test]
fn test_basic_auth_helper() {
    let creds = Credentials::BasicAuth {
        username: "user".to_string(),
        password: "pass".to_string(),
        cert: None,
    };
    assert!(creds.token().is_none());
    assert_eq!(creds.username_password(), Some(("user", "pass")));
    // "user:pass" base64 = "dXNlcjpwYXNz"
    assert_eq!(creds.basic_auth_header(), Some("dXNlcjpwYXNz".to_string()));
}

#[test]
fn test_legacy_auth_helper() {
    let creds = Credentials::LegacyAuth {
        auth: "dXNlcjpwYXNz".to_string(),
        username: "user".to_string(),
        password: "pass".to_string(),
        cert: None,
    };
    assert!(creds.token().is_none());
    assert_eq!(creds.username_password(), Some(("user", "pass")));
    assert_eq!(creds.basic_auth_header(), Some("dXNlcjpwYXNz".to_string()));
}

// =============================================================================
// Upstream fixture matrix
//
// 1:1 port of the `credentials management` fixtures in
// @npmcli/config test/index.js. Fixture names are kept verbatim. Each case
// asserts the credentials for the default registry and that nothing leaks to
// another registry, matching the upstream "default registry" / "other
// registry" snapshots.
// =============================================================================

/// Base64 of `world`, as upstream writes it into `_password`.
const B64_WORLD: &str = "d29ybGQ=";
/// Base64 of `hello:world`, as upstream writes it into `_auth`.
const B64_HELLO_WORLD: &str = "aGVsbG86d29ybGQ=";

fn default_registry() -> Url {
    Url::parse("https://registry.example/").unwrap()
}

fn other_registry() -> Url {
    Url::parse("https://other.registry/").unwrap()
}

/// Assert a fixture yields no credentials for either registry.
fn assert_no_credentials(fixture: &str, npmrc: &str) {
    let (_temp, config) = setup_config(npmrc);
    assert!(
        config.credentials_for(&default_registry()).is_none(),
        "{fixture}: default registry should have no credentials"
    );
    assert!(
        config.credentials_for(&other_registry()).is_none(),
        "{fixture}: other registry should have no credentials"
    );
}

#[test]
fn test_fixture_nerfed_auth_token() {
    let (_temp, config) = setup_config("//registry.example/:_authToken = 0bad1de4");

    match config.credentials_for(&default_registry()).unwrap() {
        Credentials::Token { token, cert } => {
            assert_eq!(token, "0bad1de4");
            assert!(cert.is_none());
        }
        other => panic!("expected token, got {:?}", other),
    }
    assert!(config.credentials_for(&other_registry()).is_none());
}

#[test]
fn test_fixture_nerfed_userpass() {
    let (_temp, config) = setup_config(&format!(
        "//registry.example/:username = hello\n\
         //registry.example/:_password = {B64_WORLD}\n\
         //registry.example/:email = i@izs.me"
    ));

    match config.credentials_for(&default_registry()).unwrap() {
        Credentials::BasicAuth {
            username,
            password,
            cert,
        } => {
            assert_eq!(username, "hello");
            assert_eq!(password, "world");
            assert!(cert.is_none());
        }
        other => panic!("expected basic auth, got {:?}", other),
    }
    assert_eq!(config.email_for(&default_registry()), Some("i@izs.me"));
    assert!(config.credentials_for(&other_registry()).is_none());
    assert_eq!(config.email_for(&other_registry()), None);
}

#[test]
fn test_fixture_nerfed_auth() {
    let (_temp, config) = setup_config(&format!("//registry.example/:_auth = {B64_HELLO_WORLD}"));

    match config.credentials_for(&default_registry()).unwrap() {
        Credentials::LegacyAuth {
            auth,
            username,
            password,
            cert,
        } => {
            assert_eq!(auth, B64_HELLO_WORLD);
            assert_eq!(username, "hello");
            assert_eq!(password, "world");
            assert!(cert.is_none());
        }
        other => panic!("expected legacy auth, got {:?}", other),
    }
    assert!(config.credentials_for(&other_registry()).is_none());
}

#[test]
fn test_fixture_nerfed_mtls() {
    let (_temp, config) = setup_config(
        "//registry.example/:certfile = /path/to/cert\n\
         //registry.example/:keyfile = /path/to/key",
    );

    match config.credentials_for(&default_registry()).unwrap() {
        Credentials::ClientCertOnly(cert) => {
            assert_eq!(cert.certfile, PathBuf::from("/path/to/cert"));
            assert_eq!(cert.keyfile, PathBuf::from("/path/to/key"));
        }
        other => panic!("expected client cert only, got {:?}", other),
    }
    assert!(config.credentials_for(&other_registry()).is_none());
}

#[test]
fn test_fixture_nerfed_mtls_auth_token() {
    let (_temp, config) = setup_config(
        "//registry.example/:_authToken = 0bad1de4\n\
         //registry.example/:certfile = /path/to/cert\n\
         //registry.example/:keyfile = /path/to/key",
    );

    match config.credentials_for(&default_registry()).unwrap() {
        Credentials::Token { token, cert } => {
            assert_eq!(token, "0bad1de4");
            let cert = cert.expect("cert should be present");
            assert_eq!(cert.certfile, PathBuf::from("/path/to/cert"));
            assert_eq!(cert.keyfile, PathBuf::from("/path/to/key"));
        }
        other => panic!("expected token, got {:?}", other),
    }
    assert!(config.credentials_for(&other_registry()).is_none());
}

#[test]
fn test_fixture_nerfed_mtls_userpass() {
    let (_temp, config) = setup_config(&format!(
        "//registry.example/:username = hello\n\
         //registry.example/:_password = {B64_WORLD}\n\
         //registry.example/:email = i@izs.me\n\
         //registry.example/:certfile = /path/to/cert\n\
         //registry.example/:keyfile = /path/to/key"
    ));

    match config.credentials_for(&default_registry()).unwrap() {
        Credentials::BasicAuth {
            username,
            password,
            cert,
        } => {
            assert_eq!(username, "hello");
            assert_eq!(password, "world");
            let cert = cert.expect("cert should be present");
            assert_eq!(cert.certfile, PathBuf::from("/path/to/cert"));
            assert_eq!(cert.keyfile, PathBuf::from("/path/to/key"));
        }
        other => panic!("expected basic auth, got {:?}", other),
    }
    assert_eq!(config.email_for(&default_registry()), Some("i@izs.me"));
    assert!(config.credentials_for(&other_registry()).is_none());
}

#[test]
fn test_fixture_def_userpass() {
    // Top-level username/_password is legacy; upstream collects it as an
    // unknown config rather than using it for the registry.
    assert_no_credentials(
        "def_userpass",
        &format!(
            "username = hello\n\
             _password = {B64_WORLD}\n\
             email = i@izs.me\n\
             //registry.example/:always-auth = true\n"
        ),
    );
}

#[test]
fn test_fixture_def_user_no_pass() {
    assert_no_credentials(
        "def_userNoPass",
        "username = hello\nemail = i@izs.me\n//registry.example/:always-auth = true\n",
    );
}

#[test]
fn test_fixture_def_pass_no_user() {
    assert_no_credentials(
        "def_passNoUser",
        &format!(
            "_password = {B64_WORLD}\nemail = i@izs.me\n//registry.example/:always-auth = true\n"
        ),
    );
}

#[test]
fn test_fixture_def_auth() {
    assert_no_credentials(
        "def_auth",
        &format!("_auth = {B64_HELLO_WORLD}\nalways-auth = true"),
    );
}

#[test]
fn test_fixture_def_auth_env() {
    // Upstream fixture is `_auth = ${PATH}`: the value expands, but a
    // top-level _auth is still not registry credentials.
    std::env::set_var("AUTHTEST_DEF_AUTH_ENV", B64_HELLO_WORLD);
    assert_no_credentials("def_authEnv", "_auth = ${AUTHTEST_DEF_AUTH_ENV}");
    std::env::remove_var("AUTHTEST_DEF_AUTH_ENV");
}

#[test]
fn test_fixture_none_auth_token() {
    assert_no_credentials("none_authToken", "_authToken = 0bad1de4");
}

#[test]
fn test_fixture_none_lc_auth_token() {
    // Lowercase `_authtoken`: keys are case-sensitive, so this is not auth.
    assert_no_credentials("none_lcAuthToken", "_authtoken = 0bad1de4");
}

#[test]
fn test_fixture_none_empty_config() {
    assert_no_credentials("none_emptyConfig", "");
}

#[test]
fn test_fixture_none_no_config() {
    // Upstream `none_noConfig`: no .npmrc at all.
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("package.json"), "{}").unwrap();

    let config = NpmrcConfig::load_with_options(LoadOptions {
        cwd: Some(temp.path().to_path_buf()),
        skip_user: true,
        skip_global: true,
        ..Default::default()
    })
    .unwrap();

    assert!(config.credentials_for(&default_registry()).is_none());
    assert!(config.credentials_for(&other_registry()).is_none());
}

// =============================================================================
// Nerf-darted key handling
// =============================================================================

#[test]
fn test_nerfed_auth_token_key_is_case_sensitive() {
    let (_temp, config) = setup_config("//registry.example/:_authtoken = 0bad1de4");
    assert!(config.credentials_for(&default_registry()).is_none());
}

#[test]
fn test_credentials_do_not_leak_to_sibling_paths() {
    let (_temp, config) = setup_config("//registry.example/npm/:_authToken = 0bad1de4");

    let scoped = Url::parse("https://registry.example/npm/").unwrap();
    assert!(config.credentials_for(&scoped).is_some());
    assert!(config.credentials_for(&default_registry()).is_none());
}

// =============================================================================
// Malformed credential values
// =============================================================================

#[test]
fn test_invalid_base64_password_yields_no_credentials() {
    let (_temp, config) = setup_config(
        "//registry.example/:username = hello\n//registry.example/:_password = not!base64!",
    );
    assert!(config.credentials_for(&default_registry()).is_none());
}

#[test]
fn test_invalid_base64_password_falls_back_to_legacy_auth() {
    let (_temp, config) = setup_config(&format!(
        "//registry.example/:username = hello\n\
         //registry.example/:_password = not!base64!\n\
         //registry.example/:_auth = {B64_HELLO_WORLD}"
    ));

    match config.credentials_for(&default_registry()).unwrap() {
        Credentials::LegacyAuth { username, .. } => assert_eq!(username, "hello"),
        other => panic!("expected legacy auth, got {:?}", other),
    }
}

#[test]
fn test_invalid_base64_legacy_auth_yields_no_credentials() {
    let (_temp, config) = setup_config("//registry.example/:_auth = not!base64!");
    assert!(config.credentials_for(&default_registry()).is_none());
}

#[test]
fn test_invalid_base64_password_still_reports_client_cert() {
    let (_temp, config) = setup_config(
        "//registry.example/:username = hello\n\
         //registry.example/:_password = not!base64!\n\
         //registry.example/:certfile = /path/to/cert\n\
         //registry.example/:keyfile = /path/to/key",
    );

    match config.credentials_for(&default_registry()).unwrap() {
        Credentials::ClientCertOnly(cert) => {
            assert_eq!(cert.certfile, PathBuf::from("/path/to/cert"));
        }
        other => panic!("expected client cert only, got {:?}", other),
    }
}

// =============================================================================
// Email lookup (upstream returns this from getCredentialsByURI)
// =============================================================================

#[test]
fn test_email_requires_nerf_darted_key() {
    let (_temp, config) = setup_config("email = i@izs.me");
    assert_eq!(config.email_for(&default_registry()), None);
}

#[test]
fn test_email_without_other_credentials() {
    let (_temp, config) = setup_config("//registry.example/:email = i@izs.me");
    assert_eq!(config.email_for(&default_registry()), Some("i@izs.me"));
    assert!(config.credentials_for(&default_registry()).is_none());
}
