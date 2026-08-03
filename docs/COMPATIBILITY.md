# Compatibility with @npmcli/config

This document compares the Rust port (`npmrc-config-rs`) with [@npmcli/config v11.0.1](https://github.com/npm/cli/tree/config-v11.0.1/workspaces/config).

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
| **Parsing** | | |
| Basic quoted scalar values | Yes | Yes |
| Inline comments | Yes | Yes |
| Arrays and sections | Yes | No |
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
| Registry email | Yes (in credentials) | Yes (`email_for()`) |
| **Prefixes** | | |
| Global prefix from `PREFIX` | Yes | Yes |
| Global prefix from node executable | Yes | Yes |
| `DESTDIR` (Unix only) | Yes | Yes |
| Local prefix discovery | Yes | Yes |

## What's Included

- Loading `.npmrc` files from project, user, and global locations
- Configuration priority (project > user > global)
- Scoped registry resolution (`@scope:registry`)
- Full authentication support (tokens, basic auth, legacy auth, mTLS)
- Environment variable expansion in values (`${VAR}`)
- Quoted scalar values and inline comments
- Path expansion (`~`)

## What's Not Included

1. **CLI parsing** - No `nopt` integration for command-line arguments
2. **Environment variables** - No `npm_config_*` prefix support
3. **Builtin config level** - Not implemented
4. **Write operations** - This is a read-only library
5. **Validation/repair** - No schema validation or config repair
6. **Arrays and sections** - The public API exposes string values only

Upstream warns about unknown `.npmrc` keys by default and can reject them with
`strict-npmrc`. This crate accepts unknown keys because validation is outside
its read-only scope.

## Test Parity

Test files are ported 1:1 from the upstream suite, and their module doc comments
name the upstream file they mirror:

| Upstream test | Rust test |
|---|---|
| `test/nerf-dart.js` | `tests/nerf_dart_tests.rs` |
| `test/env-replace.js` | `tests/env_replace_tests.rs` |
| `test/index.js` - `credentials management` | `tests/auth_tests.rs` (upstream fixture names kept) |
| `test/index.js` - config loading and levels | `tests/config_tests.rs` |
| `test/index.js` - global/local prefix | `tests/paths_tests.rs` |

Upstream tests for unimplemented features have no counterpart here:
`test/parse-field.js` (typed fields), `test/set-envs.js`, `test/type-defs.js`,
`test/type-description.js`, `test/definitions/*`, `test/extension-file.js`, and
the `test/index.js` subtests for CLI parsing, `npm_config_*` env vars,
workspaces, `cafile`, `umask`, and `validate()`/`repair()`.

## Use Cases

The Rust port is designed for **read-only file-based config access**, which covers common use cases:

- Reading registry URLs for package resolution
- Retrieving authentication credentials for private registries
- Resolving scoped package registries

If you need full npm config compatibility (CLI args, env vars, writing configs), consider using the original Node.js package or contributing these features to this crate.
