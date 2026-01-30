# Compatibility with @npmcli/config

This document compares the Rust port (`npmrc-config-rs`) with the original [@npmcli/config](https://github.com/npm/cli/tree/latest/workspaces/config) package.

## Summary

**Partially compatible** - The Rust port implements a subset focused on file-based config reading.

## Feature Comparison

| Feature | @npmcli/config | npmrc-config-rs (Rust) |
|---------|---------------|---------------------|
| **Config Levels** | 7 levels | 3 levels |
| CLI switches | Yes | No |
| Environment variables (`npm_config_*`) | Yes | No |
| Project `.npmrc` | Yes | Yes |
| User `.npmrc` | Yes | Yes |
| Global `.npmrc` | Yes | Yes |
| Builtin config | Yes | No |
| Default values | Yes | Yes (registry only) |
| **Reading** | | |
| `load()` | Yes | Yes |
| `get(key)` | Yes | Yes |
| `find(key)` | Yes | No |
| `isDefault(key)` | Yes | No |
| **Writing** | | |
| `set()` | Yes | No |
| `delete()` | Yes | No |
| `save()` | Yes | No |
| **Validation** | | |
| `validate()` | Yes | No |
| `repair()` | Yes | No |
| **Registry/Auth** | | |
| Scoped registries | Yes | Yes |
| Credentials lookup | Yes | Yes |
| Nerf-darting | Yes | Yes |

## What's Included

- Loading `.npmrc` files from project, user, and global locations
- Configuration priority (project > user > global)
- Scoped registry resolution (`@scope:registry`)
- Full authentication support (tokens, basic auth, legacy auth, mTLS)
- Environment variable expansion in values (`${VAR}`)
- Path expansion (`~`)

## What's Not Included

1. **CLI parsing** - No `nopt` integration for command-line arguments
2. **Environment variables** - No `npm_config_*` prefix support
3. **Builtin config level** - Not implemented
4. **Write operations** - This is a read-only library
5. **Validation/repair** - No schema validation or config repair

## Use Cases

The Rust port is designed for **read-only file-based config access**, which covers common use cases:

- Reading registry URLs for package resolution
- Retrieving authentication credentials for private registries
- Resolving scoped package registries

If you need full npm config compatibility (CLI args, env vars, writing configs), consider using the original Node.js package or contributing these features to this crate.
