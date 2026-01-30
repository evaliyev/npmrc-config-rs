# npmrc-config-rs

A Rust library for reading and parsing npm `.npmrc` configuration files.

This crate provides functionality to load npm configuration from `.npmrc` files at various levels (global, user, project), resolve registries for scoped packages, and retrieve authentication credentials for private registries.

> **Note:** This is a Rust port of [@npmcli/config v10.5.0](https://github.com/npm/cli/tree/latest/workspaces/config). See [COMPATIBILITY.md](docs/COMPATIBILITY.md) for details on what's supported.

## Features

- **Multi-level configuration** - Load config from global, user, and project `.npmrc` files with proper priority handling
- **Scoped registries** - Resolve registry URLs for scoped packages (`@myorg/package`)
- **Full authentication support** - Bearer tokens, basic auth, legacy auth, and mTLS client certificates
- **Environment variable expansion** - Support for `${VAR}` and `${VAR?}` syntax in config values
- **Path expansion** - Automatic `~` expansion to home directory

## Documentation

- [Configuration Format](docs/CONFIGURATION.md) - `.npmrc` file format and authentication types
- [API Reference](docs/API.md) - Complete API documentation
- [Compatibility](docs/COMPATIBILITY.md) - Comparison with @npmcli/config

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
npmrc-config-rs = "0.1"
```

## Quick Start

```rust
use npmrc_config::{NpmrcConfig, Credentials};

fn main() -> npmrc_config::Result<()> {
    // Load config from standard locations
    let config = NpmrcConfig::load()?;

    // Get registry URL for a package
    let registry = config.registry_for("@myorg/my-package");
    println!("Registry: {}", registry);

    // Get credentials for authentication
    if let Some(creds) = config.credentials_for(&registry) {
        match creds {
            Credentials::Token { token, .. } => {
                println!("Using bearer token");
            }
            Credentials::BasicAuth { username, .. } => {
                println!("Using basic auth as {}", username);
            }
            _ => {}
        }
    }

    Ok(())
}
```

## Usage Examples

### Loading Configuration

```rust
use npmrc_config::{NpmrcConfig, LoadOptions};

// Load from standard locations
let config = NpmrcConfig::load()?;

// Load with custom options
let config = NpmrcConfig::load_with_options(LoadOptions {
    cwd: Some("/path/to/project".into()),
    global_prefix: Some("/usr/local".into()),
    user_config: Some("/custom/path/.npmrc".into()),
    skip_project: false,
    skip_user: false,
    skip_global: false,
})?;
```

### Querying Configuration

```rust
// Get raw config value
if let Some(value) = config.get("strict-ssl") {
    println!("strict-ssl = {}", value);
}

// Get default registry
let registry = config.default_registry();

// Get registry for a specific package (handles scoped packages)
let registry = config.registry_for("@myorg/package");

// Get all scoped registry mappings
let scoped = config.scoped_registries();
for (scope, url) in scoped {
    println!("{} -> {}", scope, url);
}
```

### Working with Credentials

```rust
use npmrc_config::Credentials;

let registry = config.registry_for("@myorg/package");

if let Some(creds) = config.credentials_for(&registry) {
    match creds {
        Credentials::Token { token, cert } => {
            // Use bearer token
            // cert is Some if mTLS is also configured
        }
        Credentials::BasicAuth { username, password, cert } => {
            // Use basic authentication
        }
        Credentials::LegacyAuth { auth, username, password, cert } => {
            // Use legacy _auth field
        }
        Credentials::ClientCertOnly(cert) => {
            // mTLS only, no token/password auth
        }
    }

    // Helper methods
    if let Some(token) = creds.token() {
        println!("Token: {}", token);
    }
    if let Some((user, pass)) = creds.username_password() {
        println!("User: {}", user);
    }
    if let Some(header) = creds.basic_auth_header() {
        // Ready to use in Authorization header
    }
}
```

## Error Handling

```rust
use npmrc_config::{NpmrcConfig, Error};

match NpmrcConfig::load() {
    Ok(config) => {
        // Use config
    }
    Err(Error::ReadFile { path, source }) => {
        eprintln!("Failed to read {}: {}", path.display(), source);
    }
    Err(Error::ParseIni { path, message }) => {
        eprintln!("Failed to parse {}: {}", path.display(), message);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
