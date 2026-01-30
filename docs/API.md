# API Reference

Complete API documentation for the `npmrc-config-rs` crate.

## Table of Contents

- [Structs](#structs)
  - [NpmrcConfig](#npmrcconfig)
  - [LoadOptions](#loadoptions)
  - [ConfigData](#configdata)
  - [ClientCert](#clientcert)
- [Enums](#enums)
  - [Credentials](#credentials)
  - [Error](#error)
- [Functions](#functions)
  - [nerf_dart](#nerf_dart)
  - [expand_env_vars](#expand_env_vars)
  - [expand_tilde](#expand_tilde)
  - [parse_bool](#parse_bool)
  - [find_global_prefix](#find_global_prefix)
  - [find_local_prefix](#find_local_prefix)
  - [user_config_path](#user_config_path)
  - [global_config_path](#global_config_path)
  - [project_config_path](#project_config_path)
- [Module: registry](#module-registry)
- [Type Aliases](#type-aliases)

---

## Structs

### NpmrcConfig

The main configuration struct that loads and queries `.npmrc` files.

```rust
pub struct NpmrcConfig {
    pub global_prefix: Option<PathBuf>,
    pub local_prefix: PathBuf,
    pub home: Option<PathBuf>,
    // ... private fields
}
```

#### Fields

| Field | Type | Description |
|-------|------|-------------|
| `global_prefix` | `Option<PathBuf>` | Global prefix path (e.g., `/usr/local`) |
| `local_prefix` | `PathBuf` | Local/project prefix path |
| `home` | `Option<PathBuf>` | User's home directory |

#### Methods

##### `load`

```rust
pub fn load() -> Result<Self>
```

Load configuration from standard locations with auto-detected paths.

##### `load_with_options`

```rust
pub fn load_with_options(opts: LoadOptions) -> Result<Self>
```

Load configuration with custom options for path overrides and skipping config levels.

##### `get`

```rust
pub fn get(&self, key: &str) -> Option<&str>
```

Get a raw config value by key. Searches all config layers by priority (project > user > global).

##### `default_registry`

```rust
pub fn default_registry(&self) -> Url
```

Get the default registry URL. Returns `https://registry.npmjs.org/` if not configured.

##### `registry_for`

```rust
pub fn registry_for(&self, package: &str) -> Url
```

Get the registry URL for a specific package. For scoped packages (e.g., `@myorg/package`), looks up the scoped registry configuration. Falls back to the default registry.

##### `scoped_registries`

```rust
pub fn scoped_registries(&self) -> HashMap<String, Url>
```

Get all configured scoped registries. Returns a map from scope (e.g., `@myorg`) to registry URL.

##### `credentials_for`

```rust
pub fn credentials_for(&self, registry: &Url) -> Option<Credentials>
```

Get credentials for a registry URL using nerf-darting to scope credentials.

##### `has_project_config`

```rust
pub fn has_project_config(&self) -> bool
```

Check if project-level config was loaded.

##### `has_user_config`

```rust
pub fn has_user_config(&self) -> bool
```

Check if user-level config was loaded.

##### `has_global_config`

```rust
pub fn has_global_config(&self) -> bool
```

Check if global config was loaded.

##### `project_config_path`

```rust
pub fn project_config_path(&self) -> Option<&Path>
```

Get the path to the project config if loaded.

##### `user_config_path`

```rust
pub fn user_config_path(&self) -> Option<&Path>
```

Get the path to the user config if loaded.

##### `global_config_path`

```rust
pub fn global_config_path(&self) -> Option<&Path>
```

Get the path to the global config if loaded.

---

### LoadOptions

Options for customizing configuration loading.

```rust
#[derive(Debug, Clone, Default)]
pub struct LoadOptions {
    pub cwd: Option<PathBuf>,
    pub global_prefix: Option<PathBuf>,
    pub user_config: Option<PathBuf>,
    pub skip_project: bool,
    pub skip_user: bool,
    pub skip_global: bool,
}
```

#### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cwd` | `Option<PathBuf>` | `None` | Override current working directory for project config discovery |
| `global_prefix` | `Option<PathBuf>` | `None` | Override global prefix path |
| `user_config` | `Option<PathBuf>` | `None` | Override user config path (default: `~/.npmrc`) |
| `skip_project` | `bool` | `false` | Skip loading project-level `.npmrc` |
| `skip_user` | `bool` | `false` | Skip loading user-level `~/.npmrc` |
| `skip_global` | `bool` | `false` | Skip loading global config |

---

### ConfigData

Parsed configuration data from a single `.npmrc` file.

```rust
#[derive(Debug, Clone, Default)]
pub struct ConfigData {
    pub source: PathBuf,
    pub data: HashMap<String, String>,
}
```

#### Fields

| Field | Type | Description |
|-------|------|-------------|
| `source` | `PathBuf` | Path to the source file |
| `data` | `HashMap<String, String>` | Raw key-value pairs from the INI file |

#### Methods

##### `load`

```rust
pub fn load(path: &Path) -> Result<Option<Self>>
```

Load configuration from a file path. Returns `Ok(None)` if the file doesn't exist.

##### `get`

```rust
pub fn get(&self, key: &str) -> Option<&str>
```

Get a value from this config layer.

---

### ClientCert

Client certificate for mTLS authentication.

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ClientCert {
    pub certfile: PathBuf,
    pub keyfile: PathBuf,
}
```

#### Fields

| Field | Type | Description |
|-------|------|-------------|
| `certfile` | `PathBuf` | Path to the certificate file |
| `keyfile` | `PathBuf` | Path to the private key file |

---

## Enums

### Credentials

Credentials for authenticating with an npm registry.

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Credentials {
    Token {
        token: String,
        cert: Option<ClientCert>,
    },
    BasicAuth {
        username: String,
        password: String,
        cert: Option<ClientCert>,
    },
    LegacyAuth {
        auth: String,
        username: String,
        password: String,
        cert: Option<ClientCert>,
    },
    ClientCertOnly(ClientCert),
}
```

#### Variants

| Variant | Description |
|---------|-------------|
| `Token` | Bearer token authentication (`_authToken`). Recommended method. |
| `BasicAuth` | Username and password authentication. Password decoded from base64 `_password` field. |
| `LegacyAuth` | Legacy `_auth` field containing base64-encoded `username:password`. |
| `ClientCertOnly` | Client certificate only (mTLS without token/password auth). |

#### Methods

##### `client_cert`

```rust
pub fn client_cert(&self) -> Option<&ClientCert>
```

Get the client certificate if present.

##### `token`

```rust
pub fn token(&self) -> Option<&str>
```

Get the token if this is token-based auth.

##### `username_password`

```rust
pub fn username_password(&self) -> Option<(&str, &str)>
```

Get username and password if available (for `BasicAuth` or `LegacyAuth`).

##### `basic_auth_header`

```rust
pub fn basic_auth_header(&self) -> Option<String>
```

Get the base64-encoded auth string for HTTP Basic auth header.

---

### Error

Errors that can occur when working with npmrc configuration.

```rust
#[derive(Error, Debug)]
pub enum Error {
    ReadFile { path: PathBuf, source: std::io::Error },
    ParseIni { path: PathBuf, message: String },
    InvalidUrl { url: String, message: String },
    InvalidBase64(base64::DecodeError),
    InvalidUtf8(std::string::FromUtf8Error),
}
```

#### Variants

| Variant | Description |
|---------|-------------|
| `ReadFile` | Failed to read a config file |
| `ParseIni` | Failed to parse INI content |
| `InvalidUrl` | Invalid URL in configuration |
| `InvalidBase64` | Invalid base64 encoding in password field |
| `InvalidUtf8` | UTF-8 decoding error |

---

## Functions

### nerf_dart

```rust
pub fn nerf_dart(url: &Url) -> String
```

Convert a registry URL to nerf-dart format for credential lookup.

**Example:**
```rust
use url::Url;
use npmrc_config_rs::nerf_dart;

let url = Url::parse("https://registry.npmjs.org/").unwrap();
assert_eq!(nerf_dart(&url), "//registry.npmjs.org/");
```

---

### expand_env_vars

```rust
pub fn expand_env_vars(value: &str) -> String
```

Expand `${VAR}` environment variable references in a value.

- `${VAR}` - Expands to the value of VAR, or keeps `${VAR}` literal if undefined
- `${VAR?}` - Expands to the value of VAR, or empty string if undefined
- `\\${VAR}` - Escaped, keeps the literal (with one less backslash)

---

### expand_tilde

```rust
pub fn expand_tilde(path: &str) -> PathBuf
```

Expand `~` at the start of a path to the user's home directory.

**Example:**
```rust
use npmrc_config_rs::expand_tilde;

let path = expand_tilde("~/.npmrc");
// Returns: /home/user/.npmrc (on Unix)
```

---

### parse_bool

```rust
pub fn parse_bool(value: &str) -> Option<bool>
```

Parse a boolean value from a string.

Returns `Some(true)` for "true", `Some(false)` for "false", and `None` for other values. Case-insensitive.

---

### find_global_prefix

```rust
pub fn find_global_prefix() -> Option<PathBuf>
```

Find the global prefix by locating the node executable and deriving the prefix from its location.

- **Unix**: Parent of parent of node executable (e.g., `/usr/local/bin/node` -> `/usr/local`)
- **Windows**: Parent of node executable (e.g., `c:\node\node.exe` -> `c:\node`)

Returns `None` if node cannot be found.

---

### find_local_prefix

```rust
pub fn find_local_prefix(cwd: &Path) -> PathBuf
```

Walk up from the given directory looking for the first directory containing either a `package.json` file or a `node_modules` directory.

Falls back to the starting directory if nothing is found.

---

### user_config_path

```rust
pub fn user_config_path() -> Option<PathBuf>
```

Get the path to the user's `.npmrc` file (`~/.npmrc`).

Returns `None` if the home directory cannot be determined.

---

### global_config_path

```rust
pub fn global_config_path(prefix: &Path) -> PathBuf
```

Get the path to the global `.npmrc` file (`{globalPrefix}/etc/npmrc`).

---

### project_config_path

```rust
pub fn project_config_path(prefix: &Path) -> PathBuf
```

Get the path to the project `.npmrc` file (`{localPrefix}/.npmrc`).

---

## Module: registry

Public module for registry-related utilities.

### Constants

#### `DEFAULT_REGISTRY`

```rust
pub const DEFAULT_REGISTRY: &str = "https://registry.npmjs.org/";
```

The default npm registry URL.

### Functions

#### `extract_scope`

```rust
pub fn extract_scope(package: &str) -> Option<&str>
```

Extract the scope from a package name if present.

**Example:**
```rust
use npmrc_config_rs::registry::extract_scope;

assert_eq!(extract_scope("@myorg/package"), Some("@myorg"));
assert_eq!(extract_scope("lodash"), None);
```

#### `scope_registry_key`

```rust
pub fn scope_registry_key(scope: &str) -> String
```

Build the config key for a scoped registry.

**Example:**
```rust
use npmrc_config_rs::registry::scope_registry_key;

assert_eq!(scope_registry_key("@myorg"), "@myorg:registry");
```

#### `parse_registry_url`

```rust
pub fn parse_registry_url(url: &str) -> Result<Url, url::ParseError>
```

Parse a registry URL, ensuring it has a trailing slash.

---

## Type Aliases

### Result

```rust
pub type Result<T> = std::result::Result<T, Error>;
```

Result type alias for npmrc-config-rs operations.
