# Configuration Format

The `.npmrc` file uses a simple INI-like format.

## Basic Format

```ini
# Default registry
registry = https://registry.npmjs.org/

# Scoped registries
@myorg:registry = https://npm.myorg.com/
@another:registry = https://another.example.com/

# Authentication (using nerf-darted URLs)
//npm.myorg.com/:_authToken = your-token-here
//another.example.com/:username = myuser
//another.example.com/:_password = base64-encoded-password

# Client certificates for mTLS
//secure.example.com/:certfile = /path/to/client.crt
//secure.example.com/:keyfile = /path/to/client.key

# Environment variable expansion
//registry.example.com/:_authToken = ${NPM_TOKEN}
```

## Authentication Types

### Bearer Token (Recommended)

```ini
//registry.example.com/:_authToken = npm_xxxxxxxxxxxx
```

### Username and Password

The password must be base64-encoded:

```ini
//registry.example.com/:username = myuser
//registry.example.com/:_password = cGFzc3dvcmQ=
```

### Legacy Auth

Base64-encoded `username:password`:

```ini
//registry.example.com/:_auth = dXNlcjpwYXNzd29yZA==
```

### Client Certificates (mTLS)

Can be used alone or combined with other auth methods:

```ini
//registry.example.com/:certfile = /path/to/cert.pem
//registry.example.com/:keyfile = /path/to/key.pem
```

## Environment Variable Expansion

Values in `.npmrc` can reference environment variables:

```ini
# Standard expansion - keeps literal if undefined
//registry.example.com/:_authToken = ${NPM_TOKEN}

# Optional expansion - empty string if undefined
//registry.example.com/:_authToken = ${NPM_TOKEN?}
```

## Nerf-Darting

"Nerf-darting" is npm's mechanism for scoping credentials to specific registries. Registry URLs are converted to a canonical format:

```
https://registry.npmjs.org/  →  //registry.npmjs.org/
https://npm.example.com/path/  →  //npm.example.com/path/
```

This prevents credentials from accidentally being sent to the wrong registry.

## Configuration Priority

Configuration is loaded from multiple levels with the following priority (highest to lowest):

1. **Project** - `{project}/.npmrc` (found by walking up from cwd looking for `package.json` or `node_modules`)
2. **User** - `~/.npmrc`
3. **Global** - `{prefix}/etc/npmrc` (prefix derived from node executable location)

Values from higher-priority sources override lower-priority ones.
