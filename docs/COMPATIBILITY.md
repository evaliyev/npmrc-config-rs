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

The upstream JavaScript suite is the source of truth, and parity is enforced
mechanically rather than by periodic manual review. See
[`upstream/README.md`](../upstream/README.md) for the harness.

`upstream/parity.json` records a verdict for every one of upstream's 207 tests:
either the Rust test that ports it or an explicit reason it is unported. At
`config-v11.0.1` that is 30 mapped and 177 unported. `just upstream-check` fails
if an upstream test has no verdict, if a mapping names a Rust test that no longer
exists, if a rule matches nothing upstream, or if any pinned upstream file's
content hash changes.

Assertions that upstream expresses as data are generated, not transcribed:

| Upstream source | Generated table | Consumed by |
|---|---|---|
| `test/nerf-dart.js` | `tests/generated/nerf_dart_cases.rs` (24 URL pairs) | `tests/upstream_parity_tests.rs` |
| `test/env-replace.js` | `tests/generated/env_replace_cases.rs` (15 cases) | `tests/upstream_parity_tests.rs` |
| `test/index.js` + tap snapshots | `tests/generated/credentials_cases.rs` (15 fixtures) | `tests/upstream_parity_tests.rs` |

The hand-written test files remain as readable named cases and cover behaviour
with no upstream counterpart:

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

## Known Behavioural Divergences

Places where this crate deliberately differs from @npmcli/config v11.0.1, beyond
simply omitting a feature:

1. **Unreadable config files are fatal.** Upstream's `verbose log if config file
   read is weird error` asserts that a non-`ENOENT` read failure is logged and
   `load()` still resolves. This crate returns `Error::ReadFile` instead.
2. **No top-level `email` fallback.** Upstream's `getCredentialsByURI` reads
   `this.get('${nerfed}:email') || this.get('email')`. This crate only honours
   the nerf-darted key, matching npm 12's direction (where top-level `email` is
   a hard error) rather than v11.0.1's literal behaviour.
3. **Same-file project/user config is loaded as both levels.** Upstream dedups,
   marking the project source `(same as "user" config, ignored)`. The merged
   values agree; the source bookkeeping does not.
4. **Fixtures upstream routes through `repair()`** (`def_authEnv`) are skipped
   rather than asserted, since `repair()` is unported. The generated table marks
   them `Expect::SkipNeedsRepair` so the gap stays visible.

## Use Cases

The Rust port is designed for **read-only file-based config access**, which covers common use cases:

- Reading registry URLs for package resolution
- Retrieving authentication credentials for private registries
- Resolving scoped package registries

If you need full npm config compatibility (CLI args, env vars, writing configs), consider using the original Node.js package or contributing these features to this crate.
