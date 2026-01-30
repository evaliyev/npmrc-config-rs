//! Rust port of npm .npmrc configuration logic.
//!
//! This crate provides functionality to load and query npm configuration
//! from `.npmrc` files, including support for:
//!
//! - Multiple config levels (global, user, project)
//! - Scoped registries
//! - Authentication (bearer tokens, basic auth, mTLS)
//! - Environment variable expansion
//!
//! # Quick Start
//!
//! ```no_run
//! use npmrc_config_rs::NpmrcConfig;
//!
//! // Load config from standard locations
//! let config = NpmrcConfig::load().unwrap();
//!
//! // Get registry for a package
//! let registry = config.registry_for("@myorg/package");
//!
//! // Get credentials for authentication
//! if let Some(creds) = config.credentials_for(&registry) {
//!     match creds {
//!         npmrc_config_rs::Credentials::Token { token, .. } => {
//!             println!("Using token: {}", token);
//!         }
//!         npmrc_config_rs::Credentials::BasicAuth { username, password, .. } => {
//!             println!("Using basic auth: {}:***", username);
//!         }
//!         _ => {}
//!     }
//! }
//! ```
//!
//! # Configuration Priority
//!
//! Configuration is loaded from multiple levels with the following priority
//! (highest to lowest):
//!
//! 1. **Project** - `{localPrefix}/.npmrc` (found by walking up from cwd)
//! 2. **User** - `~/.npmrc`
//! 3. **Global** - `{globalPrefix}/etc/npmrc`
//!
//! Values from higher-priority sources override lower-priority ones.
//!
//! # Authentication (Nerf-Darting)
//!
//! npm uses "nerf-darting" to scope credentials to specific registries,
//! preventing credential leakage. Registry URLs are converted to a
//! canonical form:
//!
//! ```text
//! https://registry.npmjs.org/ → //registry.npmjs.org/
//! ```
//!
//! Auth configuration uses this format:
//!
//! ```ini
//! //registry.npmjs.org/:_authToken = your-token
//! //private.registry.com/:username = user
//! //private.registry.com/:_password = base64-encoded-password
//! ```

mod auth;
mod config;
mod error;
mod parser;
mod paths;
pub mod registry;

// Re-export main types
pub use auth::{nerf_dart, ClientCert, Credentials};
pub use config::{ConfigData, LoadOptions, NpmrcConfig};
pub use error::{Error, Result};
pub use parser::{expand_env_vars, parse_bool};
pub use paths::{
    expand_tilde, find_global_prefix, find_local_prefix, global_config_path, project_config_path,
    user_config_path,
};
