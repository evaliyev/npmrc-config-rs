---
description: Reconcile this port with the upstream @npmcli/config test suite
allowed-tools: Bash, Read, Edit, Write, Grep, Glob
---

Reconcile this Rust port with the upstream @npmcli/config JavaScript test suite.
Upstream tests are the source of truth: Rust tests match them, then Rust code
conforms to the Rust tests.

## 1. Get the facts before changing anything

```
just upstream-check
```

Every problem it prints names a file and a fix. If it reports no drift, stop and
say so — do not invent work.

Read `upstream/parity.json` for the current verdict on every upstream test, and
`docs/COMPATIBILITY.md` for the declared scope. The `upstream/.cache/<tag>/`
directory holds the pinned upstream sources; read them there rather than
refetching.

## 2. Handle each kind of drift

**A newer `config-v*` tag exists.** Run `just upstream-sync-bump`, then diff the
old and new upstream sources to see what actually changed:

```
diff -ru upstream/.cache/<old-tag>/test upstream/.cache/<new-tag>/test
```

**A pinned file's hash changed.** Same diff, then `just upstream-sync`.

**A generated table is stale.** Run `just upstream-sync`. Never hand-edit
anything under `tests/generated/` — the header says so and the check enforces it.

**An upstream test has no verdict.** For each one, decide and record it in
`upstream/parity.json`:
- In scope for a read-only, file-based config reader → write the Rust test, then
  add `"rust": "<test_fn>"`.
- Out of scope (CLI parsing, `npm_config_*` env, builtin level, writes,
  validate/repair, workspaces, the definitions/type system) → add
  `"unported": "<specific reason>"`. Reasons must say what is missing, not
  just "not supported".

**A rule matches nothing.** Upstream renamed or dropped that test. Decide whether
the Rust test it named still covers real behaviour; if not, delete it.

## 3. Port tests before touching src/

When an upstream test is in scope:

1. Write or update the Rust test first, mirroring upstream's assertions and
   values exactly. Keep upstream fixture names verbatim (`nerfed_authToken`).
2. Run it and let it fail.
3. Only then change `src/` to make it pass.

If a value in the upstream suite comes from a tap snapshot, take it from
`upstream/.cache/<tag>/tap-snapshots/`, not from inference. If the upstream
assertion is data-driven and the extractor in `upstream/lib.mjs` could pull it
out mechanically, prefer extending the generator over hand-authoring cases.

## 4. Report divergence honestly

If upstream's behaviour and this crate's behaviour genuinely differ and you are
not changing the code, say so in three places: a `note` on the parity.json rule,
`docs/COMPATIBILITY.md`, and your summary to the user. Do not quietly relax a
Rust assertion to make a check pass.

## 5. Verify

```
just check
```

That runs lint, tests, docs, MSRV, and `upstream-check`. Report what changed,
what you deliberately left unported and why, and anything that still diverges.
