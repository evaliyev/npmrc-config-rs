//! Path resolution tests.
//!
//! Tests for finding global prefix, local prefix, and expanding paths.

use npmrc_config_rs::{
    expand_tilde, find_local_prefix, global_config_path, project_config_path, user_config_path,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// =============================================================================
// Local prefix discovery (project root)
// =============================================================================

#[test]
fn test_find_local_prefix_with_package_json() {
    let temp = TempDir::new().unwrap();
    let project_dir = temp.path().join("my-project");
    fs::create_dir_all(&project_dir).unwrap();
    fs::write(project_dir.join("package.json"), "{}").unwrap();

    let sub_dir = project_dir.join("src").join("lib").join("deep");
    fs::create_dir_all(&sub_dir).unwrap();

    let result = find_local_prefix(&sub_dir);
    assert_eq!(result, project_dir);
}

#[test]
fn test_find_local_prefix_with_node_modules() {
    let temp = TempDir::new().unwrap();
    let project_dir = temp.path().join("my-project");
    fs::create_dir_all(project_dir.join("node_modules")).unwrap();

    let sub_dir = project_dir.join("src");
    fs::create_dir_all(&sub_dir).unwrap();

    let result = find_local_prefix(&sub_dir);
    assert_eq!(result, project_dir);
}

#[test]
fn test_find_local_prefix_prefers_package_json() {
    let temp = TempDir::new().unwrap();

    // Create a nested structure where package.json is deeper
    let outer = temp.path().join("outer");
    let inner = outer.join("inner");

    fs::create_dir_all(outer.join("node_modules")).unwrap();
    fs::create_dir_all(&inner).unwrap();
    fs::write(inner.join("package.json"), "{}").unwrap();

    // Starting from inner, should find inner first (it has package.json)
    let result = find_local_prefix(&inner);
    assert_eq!(result, inner);
}

#[test]
fn test_find_local_prefix_fallback_to_cwd() {
    let temp = TempDir::new().unwrap();
    let empty_dir = temp.path().join("empty");
    fs::create_dir_all(&empty_dir).unwrap();

    let result = find_local_prefix(&empty_dir);
    assert_eq!(result, empty_dir);
}

#[test]
fn test_find_local_prefix_at_root() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("package.json"), "{}").unwrap();

    let result = find_local_prefix(temp.path());
    assert_eq!(result, temp.path());
}

#[test]
fn test_find_local_prefix_stops_at_first_match() {
    let temp = TempDir::new().unwrap();

    // Nested projects
    let outer = temp.path().join("outer");
    let inner = outer.join("inner");

    fs::create_dir_all(&inner).unwrap();
    fs::write(outer.join("package.json"), "{}").unwrap();
    fs::write(inner.join("package.json"), "{}").unwrap();

    // Starting from inner, should find inner
    let result = find_local_prefix(&inner);
    assert_eq!(result, inner);
}

// =============================================================================
// Config path builders
// =============================================================================

#[test]
fn test_global_config_path() {
    let prefix = PathBuf::from("/usr/local");
    assert_eq!(
        global_config_path(&prefix),
        PathBuf::from("/usr/local/etc/npmrc")
    );
}

#[test]
fn test_global_config_path_windows_style() {
    let prefix = PathBuf::from("C:\\Program Files\\nodejs");
    let result = global_config_path(&prefix);
    assert!(result.ends_with("npmrc"));
    assert!(result.to_str().unwrap().contains("etc"));
}

#[test]
fn test_project_config_path() {
    let prefix = PathBuf::from("/home/user/project");
    assert_eq!(
        project_config_path(&prefix),
        PathBuf::from("/home/user/project/.npmrc")
    );
}

#[test]
fn test_user_config_path() {
    let result = user_config_path();
    // Should return Some path ending with .npmrc
    assert!(result.is_some());
    let path = result.unwrap();
    assert!(path.ends_with(".npmrc"));
}

// =============================================================================
// Tilde expansion
// =============================================================================

#[test]
fn test_expand_tilde_basic() {
    let home = dirs::home_dir().unwrap();
    let result = expand_tilde("~/.npmrc");
    assert_eq!(result, home.join(".npmrc"));
}

#[test]
fn test_expand_tilde_nested_path() {
    let home = dirs::home_dir().unwrap();
    let result = expand_tilde("~/foo/bar/baz");
    assert_eq!(result, home.join("foo/bar/baz"));
}

#[test]
fn test_expand_tilde_only() {
    let home = dirs::home_dir().unwrap();
    let result = expand_tilde("~");
    assert_eq!(result, home);
}

#[test]
fn test_expand_tilde_no_tilde() {
    let result = expand_tilde("/absolute/path");
    assert_eq!(result, PathBuf::from("/absolute/path"));
}

#[test]
fn test_expand_tilde_relative_path() {
    let result = expand_tilde("relative/path");
    assert_eq!(result, PathBuf::from("relative/path"));
}

#[test]
fn test_expand_tilde_tilde_in_middle() {
    // Tilde not at start should not expand
    let result = expand_tilde("/path/~/file");
    assert_eq!(result, PathBuf::from("/path/~/file"));
}

#[test]
fn test_expand_tilde_tilde_user_not_supported() {
    // ~username format is not supported, should return as-is
    let result = expand_tilde("~otheruser/.npmrc");
    assert_eq!(result, PathBuf::from("~otheruser/.npmrc"));
}

// =============================================================================
// Edge cases
// =============================================================================

#[test]
fn test_empty_path() {
    let result = expand_tilde("");
    assert_eq!(result, PathBuf::from(""));
}

#[test]
fn test_tilde_with_backslash() {
    // On Unix, backslash is a valid filename character
    let result = expand_tilde("~\\.npmrc");
    // Should not expand because there's no / after ~
    assert_eq!(result, PathBuf::from("~\\.npmrc"));
}
