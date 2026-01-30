//! Configuration loading and priority tests.
//!
//! Tests the loading of .npmrc files from different locations and
//! the priority/override behavior.

use npmrc_config_rs::{LoadOptions, NpmrcConfig};
use std::fs;
use tempfile::TempDir;

/// Create a complete test environment with global, user, and project configs
fn setup_full_environment(
    global_content: Option<&str>,
    user_content: Option<&str>,
    project_content: Option<&str>,
) -> (TempDir, LoadOptions) {
    let temp = TempDir::new().unwrap();

    let global_dir = temp.path().join("global");
    let global_etc = global_dir.join("etc");
    let user_dir = temp.path().join("user");
    let project_dir = temp.path().join("project");

    fs::create_dir_all(&global_etc).unwrap();
    fs::create_dir_all(&user_dir).unwrap();
    fs::create_dir_all(&project_dir).unwrap();

    // Create package.json to mark project root
    fs::write(project_dir.join("package.json"), "{}").unwrap();

    if let Some(content) = global_content {
        fs::write(global_etc.join("npmrc"), content).unwrap();
    }
    if let Some(content) = user_content {
        fs::write(user_dir.join(".npmrc"), content).unwrap();
    }
    if let Some(content) = project_content {
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
// Basic loading
// =============================================================================

#[test]
fn test_load_project_config_only() {
    let temp = TempDir::new().unwrap();
    let project_dir = temp.path();

    fs::write(project_dir.join("package.json"), "{}").unwrap();
    fs::write(project_dir.join(".npmrc"), "project-key = project-value").unwrap();

    let config = NpmrcConfig::load_with_options(LoadOptions {
        cwd: Some(project_dir.to_path_buf()),
        skip_user: true,
        skip_global: true,
        ..Default::default()
    })
    .unwrap();

    assert!(config.has_project_config());
    assert!(!config.has_user_config());
    assert!(!config.has_global_config());
    assert_eq!(config.get("project-key"), Some("project-value"));
}

#[test]
fn test_load_user_config_only() {
    let temp = TempDir::new().unwrap();
    let user_dir = temp.path().join("user");
    let project_dir = temp.path().join("project");

    fs::create_dir_all(&user_dir).unwrap();
    fs::create_dir_all(&project_dir).unwrap();
    fs::write(project_dir.join("package.json"), "{}").unwrap();
    fs::write(user_dir.join(".npmrc"), "user-key = user-value").unwrap();

    let config = NpmrcConfig::load_with_options(LoadOptions {
        cwd: Some(project_dir),
        user_config: Some(user_dir.join(".npmrc")),
        skip_project: true,
        skip_global: true,
        ..Default::default()
    })
    .unwrap();

    assert!(!config.has_project_config());
    assert!(config.has_user_config());
    assert!(!config.has_global_config());
    assert_eq!(config.get("user-key"), Some("user-value"));
}

#[test]
fn test_load_global_config_only() {
    let temp = TempDir::new().unwrap();
    let global_dir = temp.path().join("global");
    let global_etc = global_dir.join("etc");
    let project_dir = temp.path().join("project");

    fs::create_dir_all(&global_etc).unwrap();
    fs::create_dir_all(&project_dir).unwrap();
    fs::write(project_dir.join("package.json"), "{}").unwrap();
    fs::write(global_etc.join("npmrc"), "global-key = global-value").unwrap();

    let config = NpmrcConfig::load_with_options(LoadOptions {
        cwd: Some(project_dir),
        global_prefix: Some(global_dir),
        skip_project: true,
        skip_user: true,
        skip_global: false,
        ..Default::default()
    })
    .unwrap();

    assert!(!config.has_project_config());
    assert!(!config.has_user_config());
    assert!(config.has_global_config());
    assert_eq!(config.get("global-key"), Some("global-value"));
}

#[test]
fn test_load_all_config_levels() {
    let (_temp, opts) = setup_full_environment(
        Some("global-only = from-global\nshared = from-global"),
        Some("user-only = from-user\nshared = from-user"),
        Some("project-only = from-project\nshared = from-project"),
    );

    let config = NpmrcConfig::load_with_options(opts).unwrap();

    assert!(config.has_project_config());
    assert!(config.has_user_config());
    assert!(config.has_global_config());

    // Each level's unique key should be accessible
    assert_eq!(config.get("global-only"), Some("from-global"));
    assert_eq!(config.get("user-only"), Some("from-user"));
    assert_eq!(config.get("project-only"), Some("from-project"));
}

// =============================================================================
// Priority/override behavior
// =============================================================================

#[test]
fn test_project_overrides_user() {
    let (_temp, opts) = setup_full_environment(
        None,
        Some("override-key = from-user"),
        Some("override-key = from-project"),
    );

    let config = NpmrcConfig::load_with_options(opts).unwrap();
    assert_eq!(config.get("override-key"), Some("from-project"));
}

#[test]
fn test_user_overrides_global() {
    let (_temp, opts) = setup_full_environment(
        Some("override-key = from-global"),
        Some("override-key = from-user"),
        None,
    );

    let config = NpmrcConfig::load_with_options(opts).unwrap();
    assert_eq!(config.get("override-key"), Some("from-user"));
}

#[test]
fn test_project_overrides_global() {
    let (_temp, opts) = setup_full_environment(
        Some("override-key = from-global"),
        None,
        Some("override-key = from-project"),
    );

    let config = NpmrcConfig::load_with_options(opts).unwrap();
    assert_eq!(config.get("override-key"), Some("from-project"));
}

#[test]
fn test_full_override_chain() {
    let (_temp, opts) = setup_full_environment(
        Some("key = from-global"),
        Some("key = from-user"),
        Some("key = from-project"),
    );

    let config = NpmrcConfig::load_with_options(opts).unwrap();
    // Project (highest priority) should win
    assert_eq!(config.get("key"), Some("from-project"));
}

#[test]
fn test_fallback_to_lower_priority() {
    let (_temp, opts) = setup_full_environment(
        Some("global-key = global-value"),
        Some("user-key = user-value"),
        Some("project-key = project-value"),
    );

    let config = NpmrcConfig::load_with_options(opts).unwrap();

    // Keys only in lower-priority configs should still be accessible
    assert_eq!(config.get("global-key"), Some("global-value"));
    assert_eq!(config.get("user-key"), Some("user-value"));
}

// =============================================================================
// Missing files
// =============================================================================

#[test]
fn test_missing_project_config() {
    let temp = TempDir::new().unwrap();
    let project_dir = temp.path();
    fs::write(project_dir.join("package.json"), "{}").unwrap();
    // No .npmrc file

    let config = NpmrcConfig::load_with_options(LoadOptions {
        cwd: Some(project_dir.to_path_buf()),
        skip_user: true,
        skip_global: true,
        ..Default::default()
    })
    .unwrap();

    assert!(!config.has_project_config());
    assert!(config.get("any-key").is_none());
}

#[test]
fn test_missing_all_configs() {
    let temp = TempDir::new().unwrap();
    let project_dir = temp.path().join("empty");
    fs::create_dir_all(&project_dir).unwrap();
    fs::write(project_dir.join("package.json"), "{}").unwrap();

    let config = NpmrcConfig::load_with_options(LoadOptions {
        cwd: Some(project_dir),
        skip_user: true,
        skip_global: true,
        ..Default::default()
    })
    .unwrap();

    // Should still work, just with defaults
    assert!(!config.has_project_config());
    assert_eq!(
        config.default_registry().as_str(),
        "https://registry.npmjs.org/"
    );
}

// =============================================================================
// Skip options
// =============================================================================

#[test]
fn test_skip_project() {
    let (_temp, mut opts) =
        setup_full_environment(None, Some("key = from-user"), Some("key = from-project"));
    opts.skip_project = true;

    let config = NpmrcConfig::load_with_options(opts).unwrap();

    assert!(!config.has_project_config());
    // Should get user value since project is skipped
    assert_eq!(config.get("key"), Some("from-user"));
}

#[test]
fn test_skip_user() {
    let (_temp, mut opts) =
        setup_full_environment(Some("key = from-global"), Some("key = from-user"), None);
    opts.skip_user = true;
    opts.skip_project = true;

    let config = NpmrcConfig::load_with_options(opts).unwrap();

    assert!(!config.has_user_config());
    // Should get global value since user is skipped
    assert_eq!(config.get("key"), Some("from-global"));
}

#[test]
fn test_skip_global() {
    let (_temp, mut opts) = setup_full_environment(
        Some("global-only = from-global"),
        Some("key = from-user"),
        None,
    );
    opts.skip_global = true;
    opts.skip_project = true;

    let config = NpmrcConfig::load_with_options(opts).unwrap();

    assert!(!config.has_global_config());
    // Global-only key should not be accessible
    assert!(config.get("global-only").is_none());
}

// =============================================================================
// Registry configuration
// =============================================================================

#[test]
fn test_default_registry() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("package.json"), "{}").unwrap();

    let config = NpmrcConfig::load_with_options(LoadOptions {
        cwd: Some(temp.path().to_path_buf()),
        skip_user: true,
        skip_global: true,
        ..Default::default()
    })
    .unwrap();

    assert_eq!(
        config.default_registry().as_str(),
        "https://registry.npmjs.org/"
    );
}

#[test]
fn test_custom_default_registry() {
    let (_temp, opts) =
        setup_full_environment(None, None, Some("registry = https://custom.registry.com/"));

    let config = NpmrcConfig::load_with_options(opts).unwrap();
    assert_eq!(
        config.default_registry().as_str(),
        "https://custom.registry.com/"
    );
}

#[test]
fn test_registry_url_normalization() {
    let (_temp, opts) = setup_full_environment(
        None,
        None,
        Some("registry = https://custom.registry.com"), // No trailing slash
    );

    let config = NpmrcConfig::load_with_options(opts).unwrap();
    // Should add trailing slash
    assert_eq!(
        config.default_registry().as_str(),
        "https://custom.registry.com/"
    );
}

// =============================================================================
// Scoped registries
// =============================================================================

#[test]
fn test_scoped_registry() {
    let (_temp, opts) =
        setup_full_environment(None, None, Some("@myorg:registry = https://npm.myorg.com/"));

    let config = NpmrcConfig::load_with_options(opts).unwrap();

    assert_eq!(
        config.registry_for("@myorg/package").as_str(),
        "https://npm.myorg.com/"
    );
    // Non-scoped packages use default
    assert_eq!(
        config.registry_for("lodash").as_str(),
        "https://registry.npmjs.org/"
    );
}

#[test]
fn test_multiple_scoped_registries() {
    let (_temp, opts) = setup_full_environment(
        None,
        None,
        Some(
            r#"
@foo:registry = https://foo.example.com/
@bar:registry = https://bar.example.com/
@baz:registry = https://baz.example.com/
"#,
        ),
    );

    let config = NpmrcConfig::load_with_options(opts).unwrap();

    assert_eq!(
        config.registry_for("@foo/pkg").as_str(),
        "https://foo.example.com/"
    );
    assert_eq!(
        config.registry_for("@bar/pkg").as_str(),
        "https://bar.example.com/"
    );
    assert_eq!(
        config.registry_for("@baz/pkg").as_str(),
        "https://baz.example.com/"
    );
}

#[test]
fn test_scoped_registries_collection() {
    let (_temp, opts) = setup_full_environment(
        Some("@global:registry = https://global.example.com/"),
        Some("@user:registry = https://user.example.com/"),
        Some("@project:registry = https://project.example.com/"),
    );

    let config = NpmrcConfig::load_with_options(opts).unwrap();
    let scoped = config.scoped_registries();

    assert_eq!(scoped.len(), 3);
    assert!(scoped.contains_key("@global"));
    assert!(scoped.contains_key("@user"));
    assert!(scoped.contains_key("@project"));
}

#[test]
fn test_scoped_registry_override() {
    let (_temp, opts) = setup_full_environment(
        Some("@myorg:registry = https://global.example.com/"),
        Some("@myorg:registry = https://user.example.com/"),
        Some("@myorg:registry = https://project.example.com/"),
    );

    let config = NpmrcConfig::load_with_options(opts).unwrap();

    // Project should override
    assert_eq!(
        config.registry_for("@myorg/package").as_str(),
        "https://project.example.com/"
    );
}

// =============================================================================
// Config path accessors
// =============================================================================

#[test]
fn test_config_path_accessors() {
    let (_temp, opts) = setup_full_environment(
        Some("key = value"),
        Some("key = value"),
        Some("key = value"),
    );

    let config = NpmrcConfig::load_with_options(opts).unwrap();

    assert!(config.project_config_path().is_some());
    assert!(config.user_config_path().is_some());
    assert!(config.global_config_path().is_some());

    // Paths should end with expected filenames
    assert!(config.project_config_path().unwrap().ends_with(".npmrc"));
    assert!(config.user_config_path().unwrap().ends_with(".npmrc"));
    assert!(config.global_config_path().unwrap().ends_with("npmrc"));
}

// =============================================================================
// INI parsing edge cases
// =============================================================================

#[test]
fn test_parse_comments() {
    let (_temp, opts) = setup_full_environment(
        None,
        None,
        Some(
            r#"
# This is a comment
; This is also a comment
key = value
# Another comment
"#,
        ),
    );

    let config = NpmrcConfig::load_with_options(opts).unwrap();
    assert_eq!(config.get("key"), Some("value"));
}

#[test]
fn test_parse_empty_lines() {
    let (_temp, opts) = setup_full_environment(
        None,
        None,
        Some(
            r#"

key1 = value1

key2 = value2

"#,
        ),
    );

    let config = NpmrcConfig::load_with_options(opts).unwrap();
    assert_eq!(config.get("key1"), Some("value1"));
    assert_eq!(config.get("key2"), Some("value2"));
}

#[test]
fn test_parse_no_spaces_around_equals() {
    let (_temp, opts) = setup_full_environment(None, None, Some("key=value"));

    let config = NpmrcConfig::load_with_options(opts).unwrap();
    assert_eq!(config.get("key"), Some("value"));
}

#[test]
fn test_parse_value_with_equals() {
    let (_temp, opts) = setup_full_environment(None, None, Some("key = value=with=equals"));

    let config = NpmrcConfig::load_with_options(opts).unwrap();
    assert_eq!(config.get("key"), Some("value=with=equals"));
}

#[test]
fn test_parse_whitespace_in_value() {
    let (_temp, opts) = setup_full_environment(None, None, Some("key =   value with spaces   "));

    let config = NpmrcConfig::load_with_options(opts).unwrap();
    // Value should be trimmed
    assert_eq!(config.get("key"), Some("value with spaces"));
}
