// Shared core for the upstream parity harness.
//
// Fetches @npmcli/config sources at the pinned tag, extracts the data-driven
// test tables by *executing* the upstream test files against stubbed `tap` and
// `lib/*` modules, and renders them as Rust case tables.
//
// Executing beats scraping: upstream can reformat, rename locals, or change how
// an expected value is computed (`Buffer.from(...).toString('base64')`) and the
// extraction still yields exact values.

import { createHash } from 'node:crypto'
import { createRequire } from 'node:module'
import Module from 'node:module'
import { mkdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const HERE = dirname(fileURLToPath(import.meta.url))
export const REPO = resolve(HERE, '..')
export const CACHE = join(HERE, '.cache')

const RAW = 'https://raw.githubusercontent.com/npm/cli'
const UPSTREAM_PATH = 'workspaces/config'

/** Files the harness reads. Anything here is cached per tag. */
export const TRACKED = [
  'test/index.js',
  'test/nerf-dart.js',
  'test/env-replace.js',
  'test/parse-field.js',
  'test/set-envs.js',
  'test/type-defs.js',
  'test/type-description.js',
  'test/extension-file.js',
  'test/definitions/definition.js',
  'test/definitions/definitions.js',
  'test/definitions/index.js',
  'tap-snapshots/test/index.js.test.cjs',
]

export function pinnedVersion () {
  return readFileSync(join(HERE, 'UPSTREAM_VERSION'), 'utf8').trim()
}

// ---------------------------------------------------------------------------
// Fetching
// ---------------------------------------------------------------------------

/** Fetch one upstream file at `tag`, caching under upstream/.cache/<tag>/. */
export async function fetchFile (tag, path) {
  const cached = join(CACHE, tag, path)
  if (existsSync(cached)) {
    return readFileSync(cached, 'utf8')
  }
  const url = `${RAW}/${tag}/${UPSTREAM_PATH}/${path}`
  const res = await fetch(url)
  if (!res.ok) {
    throw new Error(`fetch ${url} failed: ${res.status} ${res.statusText}`)
  }
  const body = await res.text()
  mkdirSync(dirname(cached), { recursive: true })
  writeFileSync(cached, body)
  return body
}

/** Fetch every tracked file at `tag` and return a path -> source map. */
export async function fetchTracked (tag) {
  const out = {}
  await Promise.all(TRACKED.map(async p => {
    out[p] = await fetchFile(tag, p)
  }))
  return out
}

/** Newest `config-v*` tag on npm/cli, via the matching-refs API. */
export async function latestVersion () {
  const url = 'https://api.github.com/repos/npm/cli/git/matching-refs/tags/config-v'
  const res = await fetch(url, { headers: { accept: 'application/vnd.github+json' } })
  if (!res.ok) {
    throw new Error(`fetch ${url} failed: ${res.status} ${res.statusText}`)
  }
  const refs = await res.json()
  const tags = refs.map(r => r.ref.replace('refs/tags/', ''))
  return tags.sort(compareTags).at(-1)
}

/** Sort `config-vX.Y.Z` ascending. Prerelease suffixes sort before releases. */
export function compareTags (a, b) {
  const parse = t => {
    const m = /^config-v(\d+)\.(\d+)\.(\d+)(?:-(.+))?$/.exec(t)
    return m ? [+m[1], +m[2], +m[3], m[4] ?? ''] : [0, 0, 0, '']
  }
  const [aMaj, aMin, aPat, aPre] = parse(a)
  const [bMaj, bMin, bPat, bPre] = parse(b)
  if (aMaj !== bMaj) {
    return aMaj - bMaj
  }
  if (aMin !== bMin) {
    return aMin - bMin
  }
  if (aPat !== bPat) {
    return aPat - bPat
  }
  // '' (release) sorts after any prerelease
  if (aPre === bPre) {
    return 0
  }
  if (aPre === '') {
    return 1
  }
  if (bPre === '') {
    return -1
  }
  return aPre < bPre ? -1 : 1
}

// ---------------------------------------------------------------------------
// Running upstream test files against stubs
// ---------------------------------------------------------------------------

/**
 * Execute an upstream CJS test file with `require` intercepted.
 *
 * `stubs` maps a matcher to a module value. A string key matches when the
 * request ends with it, so '../lib/nerf-dart.js' and 'lib/nerf-dart.js' both
 * hit the same stub regardless of how upstream writes the specifier.
 */
function runWithStubs (source, filename, stubs) {
  const dir = join(CACHE, '.run')
  mkdirSync(dir, { recursive: true })
  const scratch = join(dir, filename)
  writeFileSync(scratch, source)

  const origLoad = Module._load
  activeStubs = stubs
  Module._load = function (request, ...rest) {
    const stub = matchStub(request, stubs)
    if (stub !== undefined) {
      return stub
    }
    // Every relative require from an upstream test file points back into
    // upstream's own tree, which we deliberately do not vendor. Anything not
    // explicitly stubbed becomes permissive rather than a load failure.
    if (request.startsWith('.')) {
      return permissiveStub()
    }
    return origLoad.call(this, request, ...rest)
  }
  try {
    const require = createRequire(scratch)
    delete require.cache[scratch]
    return require(scratch)
  } finally {
    Module._load = origLoad
    activeStubs = null
  }
}

/** Stubs in effect for the file currently executing, for `t.mock` to reuse. */
let activeStubs = null

function matchStub (request, stubs) {
  for (const [suffix, value] of Object.entries(stubs)) {
    if (request === suffix || request.endsWith(suffix)) {
      return value
    }
  }
  return undefined
}

/** Sentinel returned by stubbed implementations so calls pair with assertions. */
const CALLED = Symbol('called-implementation')

/** Thrown to abort an upstream subtest once we have what we need. */
const ABORT = Symbol('abort-subtest')

/**
 * A module stub that tolerates any use: callable, constructible, and every
 * property access returns itself. Upstream test files build real `Definition`
 * instances and poke at `typeDefs` at module scope; this lets that top-level
 * code run without vendoring the whole `lib/` tree.
 */
function permissiveStub () {
  const proxy = new Proxy(function () {}, {
    get: (_t, prop) => {
      // Must not look thenable, or `await` on it hangs.
      if (prop === 'then' || prop === Symbol.toStringTag) {
        return undefined
      }
      if (prop === Symbol.toPrimitive) {
        return () => ''
      }
      return proxy
    },
    apply: () => proxy,
    construct: () => proxy,
  })
  return proxy
}

/** Stubs for the upstream `lib/` modules the test files pull in. */
function libStubs () {
  return {
    '../': permissiveStub(),
    'lib/index.js': permissiveStub(),
    'lib/type-defs.js': permissiveStub(),
    'lib/definitions/definition.js': permissiveStub(),
    'lib/definitions/definitions.js': permissiveStub(),
    'lib/definitions/index.js': {
      definitions: {},
      shorthands: {},
      nerfDarts: [],
      defaults: {},
      flatten: () => ({}),
    },
  }
}

/**
 * Build a `tap` stub. Records assertions in order and runs `t.test` callbacks
 * synchronously so nested cases are visited.
 */
function tapStub (recorded, extra = {}) {
  let proxy
  let depth = 0
  const t = {
    test (name, ...rest) {
      const cb = rest.at(-1)
      recorded.push({ kind: 'test', name, depth })
      if (typeof cb !== 'function') {
        return
      }
      depth++
      try {
        // `async t => {}` subtests reject rather than throw; anything we needed
        // was captured synchronously before the first await, so swallow it.
        Promise.resolve(cb(proxy)).catch(() => {})
      } catch (err) {
        if (err !== ABORT) {
          throw err
        }
      } finally {
        depth--
      }
    },
    equal (found, wanted, msg) {
      recorded.push({ kind: 'equal', found, wanted, msg })
    },
    same (found, wanted, msg) {
      recorded.push({ kind: 'equal', found, wanted, msg })
    },
    strictSame (found, wanted, msg) {
      recorded.push({ kind: 'equal', found, wanted, msg })
    },
    throws (fn, ...rest) {
      recorded.push({ kind: 'throws', args: captureArgs(fn) })
    },
    plan () {},
    end () {},
    matchSnapshot () {},
    ok () {},
    match () {},
    fail () {},
    rejects () {},
    testdir () {
      return '/tmp/testdir'
    },
    /**
     * Upstream loads `lib/` through `t.mock(path, overrides)`. Resolve it out of
     * the same stub table `require` uses so both paths agree.
     */
    mock (request) {
      return matchStub(request, activeStubs ?? {}) ?? permissiveStub()
    },
    ...extra,
  }
  // tap has a wide surface (teardown, beforeEach, comment, ...) and upstream
  // reaches for more of it over time. Anything not modelled becomes a no-op so
  // a new tap call in an upstream test never breaks extraction.
  proxy = new Proxy(t, {
    get (target, prop) {
      if (prop in target) {
        return target[prop]
      }
      if (typeof prop === 'symbol') {
        return undefined
      }
      return () => undefined
    },
  })
  return proxy
}

/** Run a zero-arg thunk and report the args its stubbed callee received. */
function captureArgs (fn) {
  try {
    fn()
  } catch {
    // upstream's t.throws thunks may throw for unrelated reasons; the stub
    // records whatever args it saw before that happened
  }
  return lastCall
}

let lastCall = null

/** A stubbed implementation: records its args, returns the pairing sentinel. */
function recordingStub () {
  return (...args) => {
    lastCall = args
    return CALLED
  }
}

// ---------------------------------------------------------------------------
// Extractors
// ---------------------------------------------------------------------------

/**
 * `test/nerf-dart.js`: `t.equal(nerfDart(url), dart, url)` inside
 * `t.test(dart, ...)`. Yields { url, dart } pairs plus the invalid-URL case.
 */
export function extractNerfDart (source) {
  const recorded = []
  const nerfDart = recordingStub()
  runWithStubs(source, 'nerf-dart.js', {
    tap: tapStub(recorded),
    'lib/nerf-dart.js': nerfDart,
  })

  const cases = []
  let throwsUrl = null
  for (const entry of recorded) {
    if (entry.kind === 'equal' && entry.found === CALLED) {
      cases.push({ url: entry.msg, dart: entry.wanted })
    } else if (entry.kind === 'throws' && entry.args) {
      throwsUrl = entry.args[0]
    }
  }
  if (!cases.length) {
    throw new Error('extractNerfDart: no cases recorded — upstream shape changed')
  }
  return { cases, throwsUrl }
}

/**
 * `test/env-replace.js`: flat `t.equal(envReplace(input, env), expected, msg)`.
 *
 * Upstream passes an env object; the Rust port reads the process environment,
 * so variable names are rewritten to a collision-proof prefix here. Doing the
 * rewrite in the generator keeps the Rust side a dumb loop.
 */
export function extractEnvReplace (source, prefix = 'NPMRC_UPSTREAM_ENV_') {
  const recorded = []
  const envReplace = recordingStub()
  const calls = []
  const wrapped = (...args) => {
    calls.push(args)
    return envReplace(...args)
  }
  runWithStubs(source, 'env-replace.js', {
    tap: tapStub(recorded),
    'lib/env-replace.js': wrapped,
  })

  const asserts = recorded.filter(e => e.kind === 'equal' && e.found === CALLED)
  if (asserts.length !== calls.length || !asserts.length) {
    throw new Error('extractEnvReplace: call/assert mismatch — upstream shape changed')
  }

  // The env object is the same across upstream's assertions; take it from the
  // first call and rename every key.
  const env = calls[0][1] ?? {}
  const names = Object.keys(env)
  // Names referenced but undefined still need renaming so the "unreplaced"
  // expectations line up. Collect them from the raw inputs.
  for (const [input] of calls) {
    for (const m of String(input).matchAll(/\$\{([^${}?]+)\??\}/g)) {
      if (!names.includes(m[1])) {
        names.push(m[1])
      }
    }
  }
  // Longest first so `foo` never clobbers a `foobar`.
  const ordered = [...names].sort((a, b) => b.length - a.length)
  const rename = s => ordered.reduce(
    (acc, n) => acc.split(`\${${n}}`).join(`\${${prefix}${n}}`)
      .split(`\${${n}?}`).join(`\${${prefix}${n}?}`),
    String(s)
  )

  const vars = Object.entries(env).map(([k, v]) => [prefix + k, v])
  const cases = asserts.map((a, i) => ({
    input: rename(calls[i][0]),
    expected: rename(a.wanted),
    note: a.msg ?? '',
  }))
  return { vars, cases, unsetVars: names.filter(n => !(n in env)).map(n => prefix + n) }
}

/**
 * `test/index.js` -> `t.test('credentials management')`: the `fixtures` object
 * of .npmrc bodies, captured off the `t.testdir(fixtures)` call, plus the
 * `mustCollect` set of fixtures upstream treats as unknown-config cases.
 */
export function extractCredentialFixtures (source) {
  let fixtures = null
  const recorded = []
  const t = tapStub(recorded, {
    testdir (dirs) {
      if (dirs && !fixtures && dirs.nerfed_authToken) {
        fixtures = dirs
        // Everything after this needs a real Config; stop here.
        throw ABORT
      }
      return '/tmp/testdir'
    },
  })
  let ran = null
  try {
    runWithStubs(source, 'index.js', { tap: t, ...libStubs() })
  } catch (err) {
    // Once the fixtures are captured we abort the subtest, and the rest of the
    // file may fail without a real Config. Whatever we captured still stands.
    ran = err
  }

  if (!fixtures) {
    throw new Error('extractCredentialFixtures: fixtures not captured — upstream shape '
      + `changed. Upstream file threw: ${ran ? ran.message : '(nothing)'}`)
  }

  // `mustCollect` lives in a const inside the same subtest and is not
  // observable through the stub, so read it from source. It is a plain literal.
  const m = /const mustCollect = new Set\(\[([\s\S]*?)\]\)/.exec(source)
  if (!m) {
    throw new Error('extractCredentialFixtures: mustCollect not found — upstream shape changed')
  }
  const mustCollect = [...m[1].matchAll(/'([^']+)'/g)].map(x => x[1])

  return {
    fixtures: Object.fromEntries(
      Object.entries(fixtures).map(([name, files]) => [name, files['.npmrc'] ?? null])
    ),
    mustCollect,
  }
}

/**
 * Parse tap's snapshot format into per-fixture expectation objects.
 * Keys look like: `test/index.js TAP credentials management <fixture> > default registry 1`
 */
export function extractSnapshots (source) {
  const mod = { exports: {} }
  // eslint-disable-next-line no-new-func
  new Function('exports', 'module', source)(mod.exports, mod)

  const out = {}
  for (const [key, value] of Object.entries(mod.exports)) {
    const m = /^test\/index\.js TAP credentials management (\S+) > (.+) \d+$/.exec(key)
    if (!m) {
      continue
    }
    const [, fixture, which] = m
    out[fixture] ??= {}
    out[fixture][which] = parseTapObject(value)
  }
  return out
}

/** `Object {\n  "k": "v",\n}` -> { k: 'v' }. Returns null if not parseable. */
function parseTapObject (text) {
  const body = text.trim()
  if (body === 'Object {}') {
    return {}
  }
  const m = /^Object \{([\s\S]*)\}$/.exec(body)
  if (!m) {
    return null
  }
  const obj = {}
  for (const line of m[1].split('\n')) {
    const kv = /^\s*"([^"]+)":\s*(.*),\s*$/.exec(line)
    if (!kv) {
      continue
    }
    try {
      obj[kv[1]] = JSON.parse(kv[2])
    } catch {
      // Values tap escaped as raw bytes (def_authEnv) are not round-trippable.
      return null
    }
  }
  return obj
}

/** Every `t.test(name)` in an upstream test file, in order, with nesting depth. */
export function extractTestNames (source, filename) {
  const recorded = []
  // Don't execute `t.throws` thunks here: we only want names, and running
  // upstream assertions against stubs is pointless work that can throw.
  const t = tapStub(recorded, { throws: () => {} })
  try {
    runWithStubs(source, filename, { tap: t, ...libStubs() })
  } catch {
    // Files whose top level needs a real implementation still yield the names
    // recorded before the throw.
  }
  const executed = recorded
    .filter(e => e.kind === 'test')
    .map(({ name, depth }) => ({ name, depth }))

  // Union with a static scan. Execution catches names built at runtime (the
  // credentials fixture loop names every fixture); the scan catches literals in
  // files whose top level dies before registering anything. Neither alone is
  // complete, and a missed name is a missed drift signal.
  const seen = new Set(executed.map(e => e.name))
  for (const m of source.matchAll(/\.(?:test|only|skip)\(\s*(['"`])((?:\\.|(?!\1).)*)\1/g)) {
    const name = m[2]
    if (!seen.has(name)) {
      seen.add(name)
      executed.push({ name, depth: 0, staticOnly: true })
    }
  }
  return executed
}

/** SHA-256 of every tracked upstream file at `tag`. The real drift tripwire. */
export async function fileHashes (tag) {
  const src = await fetchTracked(tag)
  const out = {}
  for (const path of TRACKED) {
    out[path] = createHash('sha256').update(src[path]).digest('hex').slice(0, 16)
  }
  return out
}

// ---------------------------------------------------------------------------
// Rust rendering
// ---------------------------------------------------------------------------

/** Rust string literal. Uses a raw-ish escape so backslashes survive. */
export function rustStr (s) {
  return JSON.stringify(String(s))
}

function header (tag, source) {
  return [
    `// @generated by upstream/sync.mjs from @npmcli/config ${tag} (${source}).`,
    '// Do not edit by hand; run `just upstream-sync` to regenerate.',
    '',
  ].join('\n')
}

export function renderNerfDart (tag, { cases, throwsUrl }) {
  const rows = cases.map(c => `    (${rustStr(c.url)}, ${rustStr(c.dart)}),`).join('\n')
  return `${header(tag, 'test/nerf-dart.js')}/// `.concat(
    `(url, expected nerf dart) pairs, in upstream order.\n`,
    `pub const NERF_DART_CASES: &[(&str, &str)] = &[\n${rows}\n];\n`,
    '\n/// Upstream asserts this input is rejected outright.\n',
    `pub const NERF_DART_INVALID: &str = ${rustStr(throwsUrl ?? 'not a valid url')};\n`
  )
}

export function renderEnvReplace (tag, { vars, cases, unsetVars }) {
  const set = vars.map(([k, v]) => `    (${rustStr(k)}, ${rustStr(v)}),`).join('\n')
  const unset = unsetVars.map(v => `    ${rustStr(v)},`).join('\n')
  const rows = cases.map(c =>
    `    (${rustStr(c.input)}, ${rustStr(c.expected)}, ${rustStr(c.note)}),`).join('\n')
  return `${header(tag, 'test/env-replace.js')}`.concat(
    '/// Variables the cases expect to be set. Names carry a prefix because the\n',
    '/// Rust port reads the process environment instead of an injected map.\n',
    `pub const ENV_REPLACE_VARS: &[(&str, &str)] = &[\n${set}\n];\n`,
    '\n/// Variables the cases expect to be absent.\n',
    `pub const ENV_REPLACE_UNSET: &[&str] = &[\n${unset}\n];\n`,
    '\n/// (input, expected, upstream message) triples, in upstream order.\n',
    `pub const ENV_REPLACE_CASES: &[(&str, &str, &str)] = &[\n${rows}\n];\n`
  )
}

export function renderCredentials (tag, { fixtures, mustCollect }, snapshots) {
  const rows = []
  for (const [name, npmrc] of Object.entries(fixtures)) {
    const snap = snapshots[name]?.['default registry']
    const other = snapshots[name]?.['other registry']
    // A fixture whose .npmrc carries no nerf-darted key can only produce
    // credentials after upstream's repair() migrates the top-level key into
    // nerfdart form. repair() is unported, so those snapshots are unreachable.
    const hasNerfDartedKey = /^\s*\/\//m.test(npmrc ?? '')
    const snapHasCreds = snap && Object.keys(snap).length > 0

    let expect
    if (mustCollect.includes(name)) {
      // Upstream asserts getUnknownConfigs() is non-empty; this crate has no
      // unknown-config tracking, so the port asserts no credentials instead.
      expect = 'Expect::UnknownUpstream'
    } else if (snap === null || (snapHasCreds && !hasNerfDartedKey)) {
      expect = 'Expect::SkipNeedsRepair'
    } else if (!snapHasCreds) {
      expect = 'Expect::NoCredentials'
    } else {
      const f = k => (snap[k] === undefined ? 'None' : `Some(${rustStr(snap[k])})`)
      expect = `Expect::Credentials { token: ${f('token')}, username: ${f('username')}, `
        + `password: ${f('password')}, auth: ${f('auth')}, certfile: ${f('certfile')}, `
        + `keyfile: ${f('keyfile')}, email: ${f('email')} }`
    }
    const otherEmpty = !other || !Object.keys(other).length
    rows.push(
      '    Case {\n'
      + `        name: ${rustStr(name)},\n`
      + `        npmrc: ${npmrc === null ? 'None' : `Some(${rustStr(npmrc)})`},\n`
      + `        expect: ${expect},\n`
      + `        other_registry_empty: ${otherEmpty},\n`
      + '    },'
    )
  }
  return `${header(tag, 'test/index.js + tap-snapshots')}`.concat(
    '/// What upstream\'s snapshot says `getCredentialsByURI` returns.\n',
    '#[derive(Debug)]\n',
    'pub enum Expect {\n',
    '    /// Snapshot is `Object {}`.\n',
    '    NoCredentials,\n',
    '    /// Upstream asserts unknown-config collection, which this crate lacks;\n',
    '    /// the port asserts no credentials instead.\n',
    '    UnknownUpstream,\n',
    '    /// Snapshot is only reachable through `repair()`, which is unported.\n',
    '    SkipNeedsRepair,\n',
    '    /// Snapshot fields, verbatim.\n',
    '    Credentials {\n',
    '        token: Option<&\'static str>,\n',
    '        username: Option<&\'static str>,\n',
    '        password: Option<&\'static str>,\n',
    '        auth: Option<&\'static str>,\n',
    '        certfile: Option<&\'static str>,\n',
    '        keyfile: Option<&\'static str>,\n',
    '        email: Option<&\'static str>,\n',
    '    },\n',
    '}\n\n',
    '#[derive(Debug)]\n',
    'pub struct Case {\n',
    '    pub name: &\'static str,\n',
    '    /// `None` means the fixture has no .npmrc at all.\n',
    '    pub npmrc: Option<&\'static str>,\n',
    '    pub expect: Expect,\n',
    '    pub other_registry_empty: bool,\n',
    '}\n\n',
    '/// Upstream `credentials management` fixtures, in upstream order.\n',
    `pub const CREDENTIAL_CASES: &[Case] = &[\n${rows.join('\n')}\n];\n`
  )
}

/** Build every generated artifact for `tag`. Returns relPath -> contents. */
export async function generate (tag) {
  const src = await fetchTracked(tag)
  const nerf = extractNerfDart(src['test/nerf-dart.js'])
  const env = extractEnvReplace(src['test/env-replace.js'])
  const creds = extractCredentialFixtures(src['test/index.js'])
  const snaps = extractSnapshots(src['tap-snapshots/test/index.js.test.cjs'])
  return {
    'tests/generated/nerf_dart_cases.rs': renderNerfDart(tag, nerf),
    'tests/generated/env_replace_cases.rs': renderEnvReplace(tag, env),
    'tests/generated/credentials_cases.rs': renderCredentials(tag, creds, snaps),
  }
}

/** Every upstream test name, keyed by file. Used for manifest coverage. */
export async function upstreamTestNames (tag) {
  const src = await fetchTracked(tag)
  const out = {}
  for (const path of TRACKED) {
    if (!path.startsWith('test/')) {
      continue
    }
    out[path] = extractTestNames(src[path], path.replace(/\//g, '_'))
  }
  return out
}

export function readManifest () {
  return JSON.parse(readFileSync(join(HERE, 'parity.json'), 'utf8'))
}
