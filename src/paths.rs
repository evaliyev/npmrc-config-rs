//! Path resolution for .npmrc configuration files.
//!
//! This module handles discovering the locations of various .npmrc files
//! following npm's resolution logic.

use std::path::{Path, PathBuf};

/// Find the global prefix by locating the node executable and deriving
/// the prefix from its location.
///
/// - **Unix**: Parent of parent of node executable (e.g., `/usr/local/bin/node` -> `/usr/local`)
/// - **Windows**: Parent of node executable (e.g., `c:\node\node.exe` -> `c:\node`)
///
/// Returns `None` if node cannot be found.
pub fn find_global_prefix() -> Option<PathBuf> {
    let node_path = which::which("node").ok()?;

    #[cfg(windows)]
    {
        // c:\node\node.exe --> prefix=c:\node\
        node_path.parent().map(|p| p.to_path_buf())
    }

    #[cfg(not(windows))]
    {
        // /usr/local/bin/node --> prefix=/usr/local
        node_path
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
    }
}

/// Walk up from the given directory looking for the first directory containing
/// either a `package.json` file or a `node_modules` directory.
///
/// Falls back to the starting directory if nothing is found.
pub fn find_local_prefix(cwd: &Path) -> PathBuf {
    let mut current = cwd.to_path_buf();

    loop {
        // Check for package.json
        if current.join("package.json").is_file() {
            return current;
        }

        // Check for node_modules directory
        if current.join("node_modules").is_dir() {
            return current;
        }

        // Move to parent directory
        match current.parent() {
            Some(parent) if parent != current => {
                current = parent.to_path_buf();
            }
            _ => break,
        }
    }

    // Fall back to cwd if nothing found
    cwd.to_path_buf()
}

/// Get the path to the user's .npmrc file (`~/.npmrc`).
///
/// Returns `None` if the home directory cannot be determined.
pub fn user_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".npmrc"))
}

/// Get the path to the global .npmrc file (`{globalPrefix}/etc/npmrc`).
pub fn global_config_path(prefix: &Path) -> PathBuf {
    prefix.join("etc").join("npmrc")
}

/// Get the path to the project .npmrc file (`{localPrefix}/.npmrc`).
pub fn project_config_path(prefix: &Path) -> PathBuf {
    prefix.join(".npmrc")
}

/// Expand `~` at the start of a path to the user's home directory.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    } else if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_local_prefix_with_package_json() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_dir = temp_dir.path().join("project");
        std::fs::create_dir_all(&project_dir).unwrap();
        std::fs::write(project_dir.join("package.json"), "{}").unwrap();

        let sub_dir = project_dir.join("src").join("lib");
        std::fs::create_dir_all(&sub_dir).unwrap();

        let result = find_local_prefix(&sub_dir);
        assert_eq!(result, project_dir);
    }

    #[test]
    fn test_find_local_prefix_with_node_modules() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_dir = temp_dir.path().join("project");
        std::fs::create_dir_all(project_dir.join("node_modules")).unwrap();

        let sub_dir = project_dir.join("src");
        std::fs::create_dir_all(&sub_dir).unwrap();

        let result = find_local_prefix(&sub_dir);
        assert_eq!(result, project_dir);
    }

    #[test]
    fn test_find_local_prefix_fallback() {
        let temp_dir = tempfile::tempdir().unwrap();
        let empty_dir = temp_dir.path().join("empty");
        std::fs::create_dir_all(&empty_dir).unwrap();

        let result = find_local_prefix(&empty_dir);
        assert_eq!(result, empty_dir);
    }

    #[test]
    fn test_expand_tilde() {
        let home = dirs::home_dir().unwrap();

        assert_eq!(expand_tilde("~/foo/bar"), home.join("foo/bar"));
        assert_eq!(expand_tilde("~"), home);
        assert_eq!(
            expand_tilde("/absolute/path"),
            PathBuf::from("/absolute/path")
        );
        assert_eq!(
            expand_tilde("relative/path"),
            PathBuf::from("relative/path")
        );
    }

    #[test]
    fn test_global_config_path() {
        let prefix = PathBuf::from("/usr/local");
        assert_eq!(
            global_config_path(&prefix),
            PathBuf::from("/usr/local/etc/npmrc")
        );
    }

    #[test]
    fn test_project_config_path() {
        let prefix = PathBuf::from("/home/user/project");
        assert_eq!(
            project_config_path(&prefix),
            PathBuf::from("/home/user/project/.npmrc")
        );
    }
}
