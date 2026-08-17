//! Upstream parity tests driven by case tables generated from @npmcli/config.
//!
//! These assertions are not hand-authored: `upstream/sync.mjs` extracts them
//! from the upstream test suite and tap snapshots, so upstream is the source of
//! truth. `just upstream-check` fails if the tables drift from the pinned
//! version. The hand-written `*_tests.rs` files stay as readable named cases and
//! cover behaviour with no upstream counterpart.

use npmrc_config_rs::{expand_env_vars, nerf_dart, Credentials, LoadOptions, NpmrcConfig};
use std::fs;
use tempfile::TempDir;
use url::Url;

mod generated {
    pub mod nerf_dart {
        include!("generated/nerf_dart_cases.rs");
    }
    pub mod env_replace {
        include!("generated/env_replace_cases.rs");
    }
    pub mod credentials {
        include!("generated/credentials_cases.rs");
    }
}

// =============================================================================
// test/nerf-dart.js
// =============================================================================

/// A broken extractor that emits empty tables would make every loop below pass
/// vacuously, which is the one failure this harness must not have.
#[test]
fn test_generated_tables_are_populated() {
    assert!(
        generated::nerf_dart::NERF_DART_CASES.len() >= 24,
        "upstream nerf-dart table shrank; check upstream/sync.mjs extraction"
    );
    assert!(
        generated::env_replace::ENV_REPLACE_CASES.len() >= 15,
        "upstream env-replace table shrank; check upstream/sync.mjs extraction"
    );
    assert!(
        generated::credentials::CREDENTIAL_CASES.len() >= 15,
        "upstream credentials table shrank; check upstream/sync.mjs extraction"
    );
}

#[test]
fn test_upstream_nerf_dart_cases() {
    for (url, expected) in generated::nerf_dart::NERF_DART_CASES {
        let parsed =
            Url::parse(url).unwrap_or_else(|e| panic!("upstream url {url} is unparseable: {e}"));
        assert_eq!(
            nerf_dart(&parsed),
            *expected,
            "nerf_dart({url}) should be {expected}"
        );
    }
}

#[test]
fn test_upstream_nerf_dart_rejects_invalid_url() {
    let invalid = generated::nerf_dart::NERF_DART_INVALID;
    assert!(
        Url::parse(invalid).is_err(),
        "upstream expects {invalid} to be rejected"
    );
}

// =============================================================================
// test/env-replace.js
// =============================================================================

#[test]
fn test_upstream_env_replace_cases() {
    // Single test fn so the env mutations below are sequential. The generated
    // names carry a prefix no other test touches.
    for (name, value) in generated::env_replace::ENV_REPLACE_VARS {
        std::env::set_var(name, value);
    }
    for name in generated::env_replace::ENV_REPLACE_UNSET {
        std::env::remove_var(name);
    }

    let mut failures = Vec::new();
    for (input, expected, note) in generated::env_replace::ENV_REPLACE_CASES {
        let got = expand_env_vars(input);
        if got != *expected {
            failures.push(format!(
                "  {note}\n    input:    {input:?}\n    expected: {expected:?}\n    got:      {got:?}"
            ));
        }
    }

    for (name, _) in generated::env_replace::ENV_REPLACE_VARS {
        std::env::remove_var(name);
    }

    assert!(
        failures.is_empty(),
        "{} of {} upstream env-replace cases failed:\n{}",
        failures.len(),
        generated::env_replace::ENV_REPLACE_CASES.len(),
        failures.join("\n")
    );
}

// =============================================================================
// test/index.js - credentials management
// =============================================================================

/// Flatten this crate's `Credentials` into the field set upstream's snapshot
/// records, so the two can be compared directly.
#[derive(Default, Debug, PartialEq)]
struct Flat {
    token: Option<String>,
    username: Option<String>,
    password: Option<String>,
    auth: Option<String>,
    certfile: Option<String>,
    keyfile: Option<String>,
    email: Option<String>,
}

fn flatten(creds: Option<Credentials>, email: Option<&str>) -> Flat {
    let mut flat = Flat {
        email: email.map(str::to_owned),
        ..Default::default()
    };
    let Some(creds) = creds else {
        return flat;
    };
    // Upstream reports `auth` as base64(username:password) whenever it has a
    // user/pass pair, whatever the source key was.
    flat.auth = creds.basic_auth_header();
    match creds {
        Credentials::Token { token, cert } => {
            flat.token = Some(token.to_owned());
            flat.auth = None;
            if let Some(cert) = cert {
                flat.certfile = Some(cert.certfile.display().to_string());
                flat.keyfile = Some(cert.keyfile.display().to_string());
            }
        }
        Credentials::BasicAuth {
            username,
            password,
            cert,
        }
        | Credentials::LegacyAuth {
            username,
            password,
            cert,
            ..
        } => {
            flat.username = Some(username.to_owned());
            flat.password = Some(password.to_owned());
            if let Some(cert) = cert {
                flat.certfile = Some(cert.certfile.display().to_string());
                flat.keyfile = Some(cert.keyfile.display().to_string());
            }
        }
        Credentials::ClientCertOnly(cert) => {
            flat.auth = None;
            flat.certfile = Some(cert.certfile.display().to_string());
            flat.keyfile = Some(cert.keyfile.display().to_string());
        }
    }
    flat
}

fn load_fixture(npmrc: Option<&str>) -> (TempDir, NpmrcConfig) {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("package.json"), "{}").unwrap();
    if let Some(body) = npmrc {
        fs::write(temp.path().join(".npmrc"), body).unwrap();
    }
    let config = NpmrcConfig::load_with_options(LoadOptions {
        cwd: Some(temp.path().to_path_buf()),
        skip_user: true,
        skip_global: true,
        ..Default::default()
    })
    .unwrap();
    (temp, config)
}

#[test]
fn test_upstream_credential_fixtures() {
    use generated::credentials::{Expect, CREDENTIAL_CASES};

    let default_registry = Url::parse("https://registry.example/").unwrap();
    let other_registry = Url::parse("https://other.registry/").unwrap();

    for case in CREDENTIAL_CASES {
        // `def_authEnv` interpolates `${PATH}`; the fixture body is used as-is
        // because expansion is the crate's own behaviour under test.
        let (_temp, config) = load_fixture(case.npmrc);
        let got = flatten(
            config.credentials_for(&default_registry),
            config.email_for(&default_registry),
        );

        match &case.expect {
            Expect::SkipNeedsRepair => continue,
            Expect::NoCredentials | Expect::UnknownUpstream => {
                assert_eq!(
                    got,
                    Flat::default(),
                    "{}: expected no credentials for the default registry",
                    case.name
                );
            }
            Expect::Credentials {
                token,
                username,
                password,
                auth,
                certfile,
                keyfile,
                email,
            } => {
                let want = Flat {
                    token: token.map(str::to_owned),
                    username: username.map(str::to_owned),
                    password: password.map(str::to_owned),
                    auth: auth.map(str::to_owned),
                    certfile: certfile.map(str::to_owned),
                    keyfile: keyfile.map(str::to_owned),
                    email: email.map(str::to_owned),
                };
                assert_eq!(got, want, "{}: default registry credentials", case.name);
            }
        }

        if case.other_registry_empty {
            assert_eq!(
                flatten(
                    config.credentials_for(&other_registry),
                    config.email_for(&other_registry),
                ),
                Flat::default(),
                "{}: credentials leaked to an unrelated registry",
                case.name
            );
        }
    }
}
