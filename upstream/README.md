# Upstream parity harness

Keeps this Rust port in sync with [@npmcli/config](https://github.com/npm/cli/tree/main/workspaces/config).
The upstream JavaScript tests are the source of truth: Rust tests match them,
then Rust code conforms to the Rust tests.

The split that matters: **detection is deterministic, interpretation is not.**
Nothing here decides whether a change is correct — it only makes drift
impossible to miss.

## Files

| File | Role |
|---|---|
| `UPSTREAM_VERSION` | The pinned upstream tag. One line. |
| `parity.json` | A verdict for every upstream test, plus per-file content hashes. |
| `lib.mjs` | Fetching, extraction, and Rust rendering. No side effects. |
| `sync.mjs` | Writes the generated tables and refreshes `parity.json`'s derived fields. |
| `check.mjs` | Reports drift. Exits non-zero so CI can gate on it. |
| `.cache/` | Fetched upstream sources, per tag. Gitignored, re-downloaded on demand. |

## Commands

```sh
just upstream-check          # full check, including "is there a newer tag"
just upstream-check-pinned   # skip the tag probe (what CI runs on PRs)
just upstream-sync           # regenerate tables at the pinned version
just upstream-sync-bump      # repin to the newest config-v* tag, then regenerate
```

`/upstream-sync` is the Claude Code command that reconciles whatever
`upstream-check` reports.

## What gets checked

1. **Is the pin stale?** Compares `UPSTREAM_VERSION` against the newest
   `config-v*` tag on npm/cli.
2. **Did the pinned files change?** A tag should be immutable; this catches a
   re-tag, a poisoned cache, or a hand-edited manifest. This is the real
   tripwire — it fires on any upstream content change even if test-name
   extraction misses something.
3. **Are the generated tables stale?** Re-runs generation and compares byte for
   byte, so `tests/generated/` can never drift from upstream or be hand-edited.
4. **Does every upstream test have a verdict?** Each of upstream's tests must
   match a rule in `parity.json` carrying either `rust` (the Rust test that
   ports it) or `unported` (why not). Rules that match nothing are also
   reported, since that means upstream renamed or dropped the test.

## How extraction works

`lib.mjs` **executes** the upstream test files with `tap` and the upstream `lib/`
modules stubbed, rather than scraping them with regexes. The stubbed
implementation records its arguments and returns a sentinel, so an assertion like

```js
t.equal(nerfDart(url), dart, url)
```

yields an exact `(input, expected)` pair. Upstream can reformat, rename locals,
or change how an expected value is computed (`Buffer.from(...).toString('base64')`)
and extraction still holds.

Two deliberate compromises:

- **Test names use a union of execution and a static scan.** Execution catches
  names built at runtime (upstream names one subtest per credentials fixture);
  the static scan catches literals in files whose top level dies before
  registering anything. A missed name is a missed drift signal, so both run.
- **`mustCollect` is read from source with a regex.** It is a local `const`
  inside a subtest and not observable through the stubs. Extraction fails loudly
  if the literal disappears.

Adding a new extractor means teaching `lib.mjs` to render it and adding the
generated file to `generate()`. Prefer that over hand-authoring cases whenever
upstream expresses assertions as data.
