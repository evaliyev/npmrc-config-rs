//! Core configuration structures and loading logic.
//!
//! This module contains the main `NpmrcConfig` struct and related types
//! for loading and querying npm configuration.

use crate::auth::{decode_password, nerf_dart, parse_legacy_auth, ClientCert, Credentials};
use crate::error::{Error, Result};
use crate::parser::parse_npmrc;
use crate::paths::{
    expand_tilde, find_global_prefix, find_local_prefix, global_config_path, project_config_path,
    user_config_path,
};
use crate::registry::{extract_scope, parse_registry_url, scope_registry_key, DEFAULT_REGISTRY};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use url::Url;

/// Parsed configuration data from a single .npmrc file.
#[derive(Debug, Clone, Default)]
pub struct ConfigData {
    /// Path to the source file.
    pub source: PathBuf,
    /// Raw key-value pairs from the INI file.
    pub data: HashMap<String, String>,
}

impl ConfigData {
    /// Load configuration from a file path.
    ///
    /// Returns `Ok(None)` if the f doesn't exist.
    /// Returns `Err` if the file exists but can't be read or parsed.
    pub fn load(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(path).map_err(|e| Error::ReadFile {
            path: path.to_path_buf(),
            source: e,
        })?;

        let data = parse_npmrc(&content, path)?;

        Ok(Some(ConfigData {
            source: path.to_path_buf(),
            data,
        }))
    }

    /// Get a value from this config layer.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }
}

/// Options for loading npm configuration.
#[derive(Debug, Clone, Default)]
pub struct LoadOptions {
    /// Override current working directory for project config discovery.
    pub cwd: Option<PathBuf>,
    /// Override global prefix path.
    pub global_prefix: Option<PathBuf>,
    /// Override user config path (default: `~/.npmrc`).
    pub user_config: Option<PathBuf>,
    /// Skip loading project-level `.npmrc`.
    pub skip_project: bool,
    /// Skip loading user-level `~/.npmrc`.
    pub skip_user: bool,
    /// Skip loading global config.
    pub skip_global: bool,
}

/// npm configuration loaded from .npmrc files.
///
/// Configuration is loaded from multiple levels with the following priority
/// (highest to lowest):
/// 1. Project `.npmrc` (`{localPrefix}/.npmrc`)
/// 2. User `.npmrc` (`~/.npmrc`)
/// 3. Global `.npmrc` (`{globalPrefix}/etc/npmrc`)
///
/// # Examples
///
/// ```no_run
/// use npmrc_config_rs::NpmrcConfig;
///
/// // Load from standard locations
/// let config = NpmrcConfig::load().unwrap();
///
/// // Get the registry for a package
/// let registry = config.registry_for("lodash");
/// let scoped_registry = config.registry_for("@myorg/package");
///
/// // Get credentials for a registry
/// if let Some(creds) = config.credentials_for(&registry) {
///     // Use credentials...
/// }
/// ```
#[derive(Debug, Clone)]
pub struct NpmrcConfig {
    /// Global prefix path (e.g., `/usr/local`).
    pub global_prefix: Option<PathBuf>,
    /// Local/project prefix path.
    pub local_prefix: PathBuf,
    /// User's home directory.
    pub home: Option<PathBuf>,

    /// Global config (`{globalPrefix}/etc/npmrc`).
    global_config: Option<ConfigData>,
    /// User config (`~/.npmrc`).
    user_config: Option<ConfigData>,
    /// Project config (`{localPrefix}/.npmrc`).
    project_config: Option<ConfigData>,
}

impl NpmrcConfig {
    /// Load configuration from standard locations with auto-detected paths.
    pub fn load() -> Result<Self> {
        Self::load_with_options(LoadOptions::default())
    }

    /// Load configuration from a single file path.
    ///
    /// This loads only the specified file, bypassing the standard multi-layer
    /// config discovery (project, user, global).
    ///
    /// Returns `Err(Error::FileNotFound)` if the file doesn't exist.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use npmrc_config_rs::NpmrcConfig;
    /// use std::path::Path;
    ///
    /// let config = NpmrcConfig::load_from_file(Path::new("/path/to/custom.npmrc"))?;
    /// # Ok::<(), npmrc_config_rs::Error>(())
    /// ```
    pub fn load_from_file(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(Error::FileNotFound(path.to_path_buf()));
        }

        let content = std::fs::read_to_string(path).map_err(|e| Error::ReadFile {
            path: path.to_path_buf(),
            source: e,
        })?;

        let data = parse_npmrc(&content, path)?;

        let config = ConfigData {
            source: path.to_path_buf(),
            data,
        };

        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        Ok(NpmrcConfig {
            global_prefix: find_global_prefix(),
            local_prefix: find_local_prefix(&cwd),
            home: dirs::home_dir(),
            global_config: None,
            user_config: None,
            project_config: Some(config),
        })
    }

    /// Load configuration with custom options.
    pub fn load_with_options(opts: LoadOptions) -> Result<Self> {
        let cwd = opts
            .cwd
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let global_prefix = opts.global_prefix.or_else(find_global_prefix);
        let local_prefix = find_local_prefix(&cwd);
        let home = dirs::home_dir();

        // Load global config
        let global_config = if opts.skip_global {
            None
        } else if let Some(ref prefix) = global_prefix {
            let path = global_config_path(prefix);
            ConfigData::load(&path)?
        } else {
            None
        };

        // Load user config
        let user_config = if opts.skip_user {
            None
        } else {
            let path = opts.user_config.or_else(user_config_path);
            if let Some(path) = path {
                ConfigData::load(&path)?
            } else {
                None
            }
        };

        // Load project config
        let project_config = if opts.skip_project {
            None
        } else {
            let path = project_config_path(&local_prefix);
            ConfigData::load(&path)?
        };

        Ok(NpmrcConfig {
            global_prefix,
            local_prefix,
            home,
            global_config,
            user_config,
            project_config,
        })
    }

    /// Get a raw config value by key.
    ///
    /// Searches all config layers by priority (project > user > global).
    pub fn get(&self, key: &str) -> Option<&str> {
        // Check in priority order: project > user > global
        self.project_config
            .as_ref()
            .and_then(|c| c.get(key))
            .or_else(|| self.user_config.as_ref().and_then(|c| c.get(key)))
            .or_else(|| self.global_config.as_ref().and_then(|c| c.get(key)))
    }

    /// Get the default registry URL.
    pub fn default_registry(&self) -> Url {
        self.get("registry")
            .and_then(|r| parse_registry_url(r).ok())
            .unwrap_or_else(|| Url::parse(DEFAULT_REGISTRY).unwrap())
    }

    /// Get the registry URL for a specific package.
    ///
    /// For scoped packages (e.g., `@myorg/package`), looks up the scoped
    /// registry configuration (`@myorg:registry`). Falls back to the
    /// default registry if no scoped registry is configured.
    pub fn registry_for(&self, package: &str) -> Url {
        if let Some(scope) = extract_scope(package) {
            let key = scope_registry_key(scope);
            if let Some(url) = self.get(&key) {
                if let Ok(parsed) = parse_registry_url(url) {
                    return parsed;
                }
            }
        }
        self.default_registry()
    }

    /// Get all configured scoped registries.
    ///
    /// Returns a map from scope (e.g., `@myorg`) to registry URL.
    pub fn scoped_registries(&self) -> HashMap<String, Url> {
        let mut result = HashMap::new();

        // Collect from all config layers (lower priority first so higher overwrites)
        for config in [&self.global_config, &self.user_config, &self.project_config]
            .into_iter()
            .flatten()
        {
            for (key, value) in &config.data {
                if key.starts_with('@') && key.ends_with(":registry") {
                    let scope = &key[..key.len() - ":registry".len()];
                    if let Ok(url) = parse_registry_url(value) {
                        result.insert(scope.to_string(), url);
                    }
                }
            }
        }

        result
    }

    /// Get credentials for a registry URL.
    ///
    /// Looks up authentication configuration using nerf-darting to scope
    /// credentials to the specific registry.
    pub fn credentials_for(&self, registry: &Url) -> Option<Credentials> {
        let nerfed = nerf_dart(registry);

        // Check for client certificate (can be used with other auth types)
        let cert = self.get_client_cert(&nerfed);

        // Check for bearer token (_authToken) - highest priority
        let token_key = format!("{}:_authToken", nerfed);
        if let Some(token) = self.get(&token_key) {
            return Some(Credentials::Token {
                token: token.to_string(),
                cert,
            });
        }

        // Check for username/password
        let username_key = format!("{}:username", nerfed);
        let password_key = format!("{}:_password", nerfed);
        if let (Some(username), Some(encoded_password)) =
            (self.get(&username_key), self.get(&password_key))
        {
            if let Ok(password) = decode_password(encoded_password) {
                return Some(Credentials::BasicAuth {
                    username: username.to_string(),
                    password,
                    cert,
                });
            }
        }

        // Check for legacy _auth field
        let auth_key = format!("{}:_auth", nerfed);
        if let Some(auth) = self.get(&auth_key) {
            if let Ok((username, password)) = parse_legacy_auth(auth) {
                return Some(Credentials::LegacyAuth {
                    auth: auth.to_string(),
                    username,
                    password,
                    cert,
                });
            }
        }

        // Return client cert only if no other auth was found
        cert.map(Credentials::ClientCertOnly)
    }

    /// Get client certificate configuration for a nerf-darted key.
    fn get_client_cert(&self, nerfed: &str) -> Option<ClientCert> {
        let certfile_key = format!("{}:certfile", nerfed);
        let keyfile_key = format!("{}:keyfile", nerfed);

        match (self.get(&certfile_key), self.get(&keyfile_key)) {
            (Some(certfile), Some(keyfile)) => Some(ClientCert {
                certfile: expand_tilde(certfile),
                keyfile: expand_tilde(keyfile),
            }),
            _ => None,
        }
    }

    /// Check if a specific config file was loaded.
    pub fn has_project_config(&self) -> bool {
        self.project_config.is_some()
    }

    /// Check if user config was loaded.
    pub fn has_user_config(&self) -> bool {
        self.user_config.is_some()
    }

    /// Check if global config was loaded.
    pub fn has_global_config(&self) -> bool {
        self.global_config.is_some()
    }

    /// Get the path to the project config if loaded.
    pub fn project_config_path(&self) -> Option<&Path> {
        self.project_config.as_ref().map(|c| c.source.as_path())
    }

    /// Get the path to the user config if loaded.
    pub fn user_config_path(&self) -> Option<&Path> {
        self.user_config.as_ref().map(|c| c.source.as_path())
    }

    /// Get the path to the global config if loaded.
    pub fn global_config_path(&self) -> Option<&Path> {
        self.global_config.as_ref().map(|c| c.source.as_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_load_project_config() {
        let temp = setup_test_dir();
        let project_dir = temp.path();

        // Create package.json to mark as project root
        fs::write(project_dir.join("package.json"), "{}").unwrap();

        // Create .npmrc
        fs::write(
            project_dir.join(".npmrc"),
            "registry = https://custom.registry.com/\n",
        )
        .unwrap();

        let config = NpmrcConfig::load_with_options(LoadOptions {
            cwd: Some(project_dir.to_path_buf()),
            skip_user: true,
            skip_global: true,
            ..Default::default()
        })
        .unwrap();

        assert!(config.has_project_config());
        assert_eq!(config.get("registry"), Some("https://custom.registry.com/"));
    }

    #[test]
    fn test_config_priority() {
        let temp = setup_test_dir();
        let project_dir = temp.path().join("project");
        let user_dir = temp.path().join("user");

        fs::create_dir_all(&project_dir).unwrap();
        fs::create_dir_all(&user_dir).unwrap();

        // Create package.json
        fs::write(project_dir.join("package.json"), "{}").unwrap();

        // User config with registry
        fs::write(
            user_dir.join(".npmrc"),
            "registry = https://user.registry.com/\nuser-key = user-value\n",
        )
        .unwrap();

        // Project config with different registry
        fs::write(
            project_dir.join(".npmrc"),
            "registry = https://project.registry.com/\n",
        )
        .unwrap();

        let config = NpmrcConfig::load_with_options(LoadOptions {
            cwd: Some(project_dir.clone()),
            user_config: Some(user_dir.join(".npmrc")),
            skip_global: true,
            ..Default::default()
        })
        .unwrap();

        // Project should override user
        assert_eq!(
            config.get("registry"),
            Some("https://project.registry.com/")
        );

        // User-only key should still be accessible
        assert_eq!(config.get("user-key"), Some("user-value"));
    }

    #[test]
    fn test_scoped_registry() {
        let temp = setup_test_dir();
        let project_dir = temp.path();

        fs::write(project_dir.join("package.json"), "{}").unwrap();
        fs::write(
            project_dir.join(".npmrc"),
            "@myorg:registry = https://myorg.registry.com/\n\
             @another:registry = https://another.registry.com/\n",
        )
        .unwrap();

        let config = NpmrcConfig::load_with_options(LoadOptions {
            cwd: Some(project_dir.to_path_buf()),
            skip_user: true,
            skip_global: true,
            ..Default::default()
        })
        .unwrap();

        assert_eq!(
            config.registry_for("@myorg/package").as_str(),
            "https://myorg.registry.com/"
        );
        assert_eq!(
            config.registry_for("@another/pkg").as_str(),
            "https://another.registry.com/"
        );
        assert_eq!(
            config.registry_for("regular-package").as_str(),
            DEFAULT_REGISTRY
        );
    }

    #[test]
    fn test_credentials_token() {
        let temp = setup_test_dir();
        let project_dir = temp.path();

        fs::write(project_dir.join("package.json"), "{}").unwrap();
        fs::write(
            project_dir.join(".npmrc"),
            "//registry.npmjs.org/:_authToken = my-secret-token\n",
        )
        .unwrap();

        let config = NpmrcConfig::load_with_options(LoadOptions {
            cwd: Some(project_dir.to_path_buf()),
            skip_user: true,
            skip_global: true,
            ..Default::default()
        })
        .unwrap();

        let registry = Url::parse("https://registry.npmjs.org/").unwrap();
        let creds = config.credentials_for(&registry).unwrap();

        match creds {
            Credentials::Token { token, cert } => {
                assert_eq!(token, "my-secret-token");
                assert!(cert.is_none());
            }
            _ => panic!("Expected Token credentials"),
        }
    }

    #[test]
    fn test_credentials_basic_auth() {
        let temp = setup_test_dir();
        let project_dir = temp.path();

        fs::write(project_dir.join("package.json"), "{}").unwrap();
        // "password" in base64 is "cGFzc3dvcmQ="
        fs::write(
            project_dir.join(".npmrc"),
            "//registry.example.com/:username = myuser\n\
             //registry.example.com/:_password = cGFzc3dvcmQ=\n",
        )
        .unwrap();

        let config = NpmrcConfig::load_with_options(LoadOptions {
            cwd: Some(project_dir.to_path_buf()),
            skip_user: true,
            skip_global: true,
            ..Default::default()
        })
        .unwrap();

        let registry = Url::parse("https://registry.example.com/").unwrap();
        let creds = config.credentials_for(&registry).unwrap();

        match creds {
            Credentials::BasicAuth {
                username,
                password,
                cert,
            } => {
                assert_eq!(username, "myuser");
                assert_eq!(password, "password");
                assert!(cert.is_none());
            }
            _ => panic!("Expected BasicAuth credentials"),
        }
    }

    #[test]
    fn test_credentials_legacy_auth() {
        let temp = setup_test_dir();
        let project_dir = temp.path();

        fs::write(project_dir.join("package.json"), "{}").unwrap();
        // "user:password" in base64 is "dXNlcjpwYXNzd29yZA=="
        fs::write(
            project_dir.join(".npmrc"),
            "//registry.example.com/:_auth = dXNlcjpwYXNzd29yZA==\n",
        )
        .unwrap();

        let config = NpmrcConfig::load_with_options(LoadOptions {
            cwd: Some(project_dir.to_path_buf()),
            skip_user: true,
            skip_global: true,
            ..Default::default()
        })
        .unwrap();

        let registry = Url::parse("https://registry.example.com/").unwrap();
        let creds = config.credentials_for(&registry).unwrap();

        match creds {
            Credentials::LegacyAuth {
                username, password, ..
            } => {
                assert_eq!(username, "user");
                assert_eq!(password, "password");
            }
            _ => panic!("Expected LegacyAuth credentials"),
        }
    }

    #[test]
    fn test_credentials_with_client_cert() {
        let temp = setup_test_dir();
        let project_dir = temp.path();

        fs::write(project_dir.join("package.json"), "{}").unwrap();
        fs::write(
            project_dir.join(".npmrc"),
            "//registry.example.com/:_authToken = token123\n\
             //registry.example.com/:certfile = /path/to/cert.pem\n\
             //registry.example.com/:keyfile = /path/to/key.pem\n",
        )
        .unwrap();

        let config = NpmrcConfig::load_with_options(LoadOptions {
            cwd: Some(project_dir.to_path_buf()),
            skip_user: true,
            skip_global: true,
            ..Default::default()
        })
        .unwrap();

        let registry = Url::parse("https://registry.example.com/").unwrap();
        let creds = config.credentials_for(&registry).unwrap();

        match creds {
            Credentials::Token { token, cert } => {
                assert_eq!(token, "token123");
                let cert = cert.unwrap();
                assert_eq!(cert.certfile, PathBuf::from("/path/to/cert.pem"));
                assert_eq!(cert.keyfile, PathBuf::from("/path/to/key.pem"));
            }
            _ => panic!("Expected Token credentials with cert"),
        }
    }

    #[test]
    fn test_no_credentials() {
        let temp = setup_test_dir();
        let project_dir = temp.path();

        fs::write(project_dir.join("package.json"), "{}").unwrap();
        fs::write(
            project_dir.join(".npmrc"),
            "registry = https://example.com/\n",
        )
        .unwrap();

        let config = NpmrcConfig::load_with_options(LoadOptions {
            cwd: Some(project_dir.to_path_buf()),
            skip_user: true,
            skip_global: true,
            ..Default::default()
        })
        .unwrap();

        let registry = Url::parse("https://example.com/").unwrap();
        assert!(config.credentials_for(&registry).is_none());
    }

    #[test]
    fn test_scoped_registries_map() {
        let temp = setup_test_dir();
        let project_dir = temp.path();

        fs::write(project_dir.join("package.json"), "{}").unwrap();
        fs::write(
            project_dir.join(".npmrc"),
            "@foo:registry = https://foo.example.com/\n\
             @bar:registry = https://bar.example.com/\n",
        )
        .unwrap();

        let config = NpmrcConfig::load_with_options(LoadOptions {
            cwd: Some(project_dir.to_path_buf()),
            skip_user: true,
            skip_global: true,
            ..Default::default()
        })
        .unwrap();

        let scoped = config.scoped_registries();
        assert_eq!(scoped.len(), 2);
        assert_eq!(
            scoped.get("@foo").map(|u| u.as_str()),
            Some("https://foo.example.com/")
        );
        assert_eq!(
            scoped.get("@bar").map(|u| u.as_str()),
            Some("https://bar.example.com/")
        );
    }

    #[test]
    fn test_load_from_file() {
        let temp = setup_test_dir();
        let npmrc_path = temp.path().join("custom.npmrc");

        fs::write(
            &npmrc_path,
            "registry = https://custom.registry.com/\n\
             @myorg:registry = https://myorg.registry.com/\n",
        )
        .unwrap();

        let config = NpmrcConfig::load_from_file(&npmrc_path).unwrap();

        assert_eq!(config.get("registry"), Some("https://custom.registry.com/"));
        assert_eq!(
            config.get("@myorg:registry"),
            Some("https://myorg.registry.com/")
        );
        assert!(config.has_project_config());
        assert!(!config.has_user_config());
        assert!(!config.has_global_config());
    }

    #[test]
    fn test_load_from_file_not_found() {
        let temp = setup_test_dir();
        let npmrc_path = temp.path().join("nonexistent.npmrc");

        let result = NpmrcConfig::load_from_file(&npmrc_path);

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::FileNotFound(path) => {
                assert_eq!(path, npmrc_path);
            }
            other => panic!("Expected FileNotFound error, got: {:?}", other),
        }
    }

    #[test]
    fn test_load_from_file_with_credentials() {
        let temp = setup_test_dir();
        let npmrc_path = temp.path().join("auth.npmrc");

        fs::write(
            &npmrc_path,
            "//registry.example.com/:_authToken = secret-token\n",
        )
        .unwrap();

        let config = NpmrcConfig::load_from_file(&npmrc_path).unwrap();

        let registry = Url::parse("https://registry.example.com/").unwrap();
        let creds = config.credentials_for(&registry).unwrap();

        match creds {
            Credentials::Token { token, .. } => {
                assert_eq!(token, "secret-token");
            }
            _ => panic!("Expected Token credentials"),
        }
    }
}
