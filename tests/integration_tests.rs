//! End-to-end integration tests.
//!
//! These tests verify complete workflows combining multiple features.

use npmrc_config_rs::{Credentials, LoadOptions, NpmrcConfig};
use std::fs;
use tempfile::TempDir;
use url::Url;

/// Helper to create a test directory structure with config files.
fn setup_test_environment(
    global_npmrc: Option<&str>,
    user_npmrc: Option<&str>,
    project_npmrc: Option<&str>,
) -> (TempDir, LoadOptions) {
    let temp = TempDir::new().unwrap();

    let global_dir = temp.path().join("global");
    let global_etc = global_dir.join("etc");
    let user_dir = temp.path().join("user");
    let project_dir = temp.path().join("project");

    fs::create_dir_all(&global_etc).unwrap();
    fs::create_dir_all(&user_dir).unwrap();
    fs::create_dir_all(&project_dir).unwrap();

    fs::write(project_dir.join("package.json"), "{}").unwrap();

    if let Some(content) = global_npmrc {
        fs::write(global_etc.join("npmrc"), content).unwrap();
    }
    if let Some(content) = user_npmrc {
        fs::write(user_dir.join(".npmrc"), content).unwrap();
    }
    if let Some(content) = project_npmrc {
        fs::write(project_dir.join(".npmrc"), content).unwrap();
    }

    let opts = LoadOptions {
        cwd: Some(project_dir),
        global_prefix: Some(global_dir),
        user_config: Some(user_dir.join(".npmrc")),
        skip_project: false,
        skip_user: false,
        skip_global: false,
    };

    (temp, opts)
}

// =============================================================================
// Complete workflow: private registry with scoped packages
// =============================================================================

#[test]
fn test_workflow_private_registry_with_scoped_packages() {
    let (_temp, opts) = setup_test_environment(
        None,
        None,
        Some(
            r#"
# Company private registry for scoped packages
@mycompany:registry = https://npm.mycompany.com/

# Authentication for private registry
//npm.mycompany.com/:_authToken = ${NPM_TOKEN_COMPANY}

# Public packages from default registry
registry = https://registry.npmjs.org/
"#,
        ),
    );

    // Set environment variable for the test
    std::env::set_var("NPM_TOKEN_COMPANY", "secret-corp-token-123");

    let config = NpmrcConfig::load_with_options(opts).unwrap();

    // Scoped package should use company registry
    let company_registry = config.registry_for("@mycompany/internal-lib");
    assert_eq!(company_registry.as_str(), "https://npm.mycompany.com/");

    // Public package should use default registry
    let public_registry = config.registry_for("lodash");
    assert_eq!(public_registry.as_str(), "https://registry.npmjs.org/");

    // Should have credentials for company registry
    let creds = config.credentials_for(&company_registry).unwrap();
    match creds {
        Credentials::Token { token, .. } => {
            assert_eq!(token, "secret-corp-token-123");
        }
        _ => panic!("Expected token credentials"),
    }

    // Should NOT have credentials for public registry
    let public_creds = config.credentials_for(&public_registry);
    assert!(public_creds.is_none());

    std::env::remove_var("NPM_TOKEN_COMPANY");
}

// =============================================================================
// Complete workflow: multiple organizations
// =============================================================================

#[test]
fn test_workflow_multiple_organizations() {
    let (_temp, opts) = setup_test_environment(
        None,
        None,
        Some(
            r#"
# Different registries for different orgs
@acme:registry = https://npm.acme.com/
@bigcorp:registry = https://registry.bigcorp.io/
@opensource:registry = https://registry.npmjs.org/

# Auth for each
//npm.acme.com/:_authToken = acme-token
//registry.bigcorp.io/:username = bigcorp-user
//registry.bigcorp.io/:_password = YmlnY29ycC1wYXNz
"#,
        ),
    );

    let config = NpmrcConfig::load_with_options(opts).unwrap();

    // Verify each org gets the right registry
    assert_eq!(
        config.registry_for("@acme/lib").as_str(),
        "https://npm.acme.com/"
    );
    assert_eq!(
        config.registry_for("@bigcorp/sdk").as_str(),
        "https://registry.bigcorp.io/"
    );
    assert_eq!(
        config.registry_for("@opensource/tool").as_str(),
        "https://registry.npmjs.org/"
    );

    // Verify auth types
    let acme_reg = Url::parse("https://npm.acme.com/").unwrap();
    assert!(matches!(
        config.credentials_for(&acme_reg),
        Some(Credentials::Token { .. })
    ));

    let bigcorp_reg = Url::parse("https://registry.bigcorp.io/").unwrap();
    assert!(matches!(
        config.credentials_for(&bigcorp_reg),
        Some(Credentials::BasicAuth { .. })
    ));

    // Opensource scope uses public registry, no auth
    let npm_reg = Url::parse("https://registry.npmjs.org/").unwrap();
    assert!(config.credentials_for(&npm_reg).is_none());
}

// =============================================================================
// Complete workflow: config inheritance
// =============================================================================

#[test]
fn test_workflow_config_inheritance() {
    let (_temp, opts) = setup_test_environment(
        // Global: company-wide defaults
        Some(
            r#"
registry = https://npm.company.com/
//npm.company.com/:_authToken = global-fallback-token
strict-ssl = true
"#,
        ),
        // User: personal overrides
        Some(
            r#"
//npm.company.com/:_authToken = user-personal-token
email = developer@company.com
"#,
        ),
        // Project: project-specific
        Some(
            r#"
@project:registry = https://npm.project-specific.com/
//npm.project-specific.com/:_authToken = project-token
"#,
        ),
    );

    let config = NpmrcConfig::load_with_options(opts).unwrap();

    // Registry comes from global (not overridden)
    assert_eq!(
        config.default_registry().as_str(),
        "https://npm.company.com/"
    );

    // Token for company registry comes from user (overrides global)
    let company_reg = Url::parse("https://npm.company.com/").unwrap();
    let creds = config.credentials_for(&company_reg).unwrap();
    match creds {
        Credentials::Token { token, .. } => {
            assert_eq!(token, "user-personal-token"); // User overrides global
        }
        _ => panic!("Expected token"),
    }

    // Project-specific registry works
    assert_eq!(
        config.registry_for("@project/lib").as_str(),
        "https://npm.project-specific.com/"
    );

    // Values from all levels are accessible
    assert_eq!(config.get("strict-ssl"), Some("true")); // From global
    assert_eq!(config.get("email"), Some("developer@company.com")); // From user
}

// =============================================================================
// Complete workflow: mTLS with token
// =============================================================================

#[test]
fn test_workflow_mtls_with_token() {
    let (_temp, opts) = setup_test_environment(
        None,
        None,
        Some(
            r#"
registry = https://secure.company.com/

# Both token and mTLS required
//secure.company.com/:_authToken = bearer-token-123
//secure.company.com/:certfile = /etc/ssl/client.crt
//secure.company.com/:keyfile = /etc/ssl/client.key
"#,
        ),
    );

    let config = NpmrcConfig::load_with_options(opts).unwrap();
    let registry = config.default_registry();
    let creds = config.credentials_for(&registry).unwrap();

    match creds {
        Credentials::Token { token, cert } => {
            assert_eq!(token, "bearer-token-123");
            let cert = cert.expect("Should have cert");
            assert_eq!(cert.certfile.to_str().unwrap(), "/etc/ssl/client.crt");
            assert_eq!(cert.keyfile.to_str().unwrap(), "/etc/ssl/client.key");
        }
        _ => panic!("Expected Token with cert"),
    }
}

// =============================================================================
// Complete workflow: CI/CD with optional auth
// =============================================================================

#[test]
fn test_workflow_ci_cd_optional_auth() {
    // Simulate CI environment where NPM_TOKEN might not be set
    std::env::remove_var("CI_NPM_TOKEN");

    let (_temp, opts) = setup_test_environment(
        None,
        None,
        Some(
            r#"
registry = https://registry.npmjs.org/
//registry.npmjs.org/:_authToken = ${CI_NPM_TOKEN?}
"#,
        ),
    );

    let config = NpmrcConfig::load_with_options(opts).unwrap();

    // With optional modifier, missing env var results in empty token
    // This would still create a Token credential with empty string
    let raw_token = config.get("//registry.npmjs.org/:_authToken");
    assert_eq!(raw_token, Some("")); // Empty due to ${VAR?}
}

// =============================================================================
// Error handling: graceful degradation
// =============================================================================

#[test]
fn test_graceful_degradation_missing_files() {
    let temp = TempDir::new().unwrap();
    let project_dir = temp.path().join("project");
    fs::create_dir_all(&project_dir).unwrap();
    fs::write(project_dir.join("package.json"), "{}").unwrap();
    // No .npmrc files anywhere

    let config = NpmrcConfig::load_with_options(LoadOptions {
        cwd: Some(project_dir),
        global_prefix: Some(temp.path().join("nonexistent-global")),
        user_config: Some(temp.path().join("nonexistent-user/.npmrc")),
        skip_project: false,
        skip_user: false,
        skip_global: false,
    })
    .unwrap();

    // Should still work with defaults
    assert!(!config.has_project_config());
    assert!(!config.has_user_config());
    assert!(!config.has_global_config());
    assert_eq!(
        config.default_registry().as_str(),
        "https://registry.npmjs.org/"
    );
}

// =============================================================================
// Real-world scenario: monorepo
// =============================================================================

#[test]
fn test_scenario_monorepo_nested_packages() {
    let temp = TempDir::new().unwrap();

    // Monorepo structure
    let root = temp.path().join("monorepo");
    let packages_a = root.join("packages").join("package-a");

    fs::create_dir_all(&packages_a).unwrap();

    // Root has .npmrc and package.json
    fs::write(root.join("package.json"), r#"{"name": "monorepo"}"#).unwrap();
    fs::write(
        root.join(".npmrc"),
        r#"
@monorepo:registry = https://npm.monorepo.dev/
//npm.monorepo.dev/:_authToken = mono-token
"#,
    )
    .unwrap();

    // Nested package has package.json but no .npmrc
    fs::write(
        packages_a.join("package.json"),
        r#"{"name": "@monorepo/package-a"}"#,
    )
    .unwrap();

    // Load from nested package directory
    // Note: find_local_prefix will find packages_a first, but we manually
    // specify the root as cwd for this test
    let config = NpmrcConfig::load_with_options(LoadOptions {
        cwd: Some(root.clone()),
        skip_user: true,
        skip_global: true,
        ..Default::default()
    })
    .unwrap();

    // Should find root's .npmrc
    assert!(config.has_project_config());
    assert_eq!(
        config.registry_for("@monorepo/package-a").as_str(),
        "https://npm.monorepo.dev/"
    );
}
