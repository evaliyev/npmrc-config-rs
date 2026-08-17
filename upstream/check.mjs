#!/usr/bin/env node
// Deterministic upstream drift check. No AI, no network writes, no guessing.
//
//   node upstream/check.mjs                  full check
//   node upstream/check.mjs --no-tag-probe   skip the "is there a newer tag" lookup
//
// Upstream sources are fetched once per tag and cached under upstream/.cache/,
// so repeat runs need no network beyond the tag probe.
//
// Exits non-zero when anything drifted, so CI can gate on it. Every failure
// names the file and the fix, because the output is the prompt for the sync step.

import { readFileSync, existsSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'
import {
  REPO, TRACKED, generate, pinnedVersion, latestVersion, upstreamTestNames,
  fileHashes, readManifest, compareTags,
} from './lib.mjs'

const skipTagProbe = process.argv.includes('--no-tag-probe')
const problems = []
const notes = []

const tag = pinnedVersion()
const manifest = readManifest()

// -- 1. Is the pin stale? ----------------------------------------------------
if (!skipTagProbe) {
  const latest = await latestVersion()
  if (latest && latest !== tag && compareTags(latest, tag) > 0) {
    problems.push(
      `upstream released ${latest}; this port is pinned to ${tag}.\n`
      + '    fix: `just upstream-sync-bump`, then reconcile the diff.'
    )
  }
}

// -- 2. Did the pinned files change under us? --------------------------------
// A tag should be immutable, so this catches a re-tag, a bad cache, or a
// hand-edited manifest.
const hashes = await fileHashes(tag)
if (manifest.upstream_version !== tag) {
  problems.push(
    `parity.json records upstream_version ${manifest.upstream_version}, `
    + `but UPSTREAM_VERSION says ${tag}.\n    fix: \`just upstream-sync\`.`
  )
} else {
  for (const path of TRACKED) {
    const recorded = manifest.upstream_file_hashes?.[path]
    if (!recorded) {
      problems.push(`no recorded hash for ${path}.\n    fix: \`just upstream-sync\`.`)
    } else if (recorded !== hashes[path]) {
      problems.push(
        `${path} changed at ${tag} (recorded ${recorded}, now ${hashes[path]}).\n`
        + '    fix: review the diff, then `just upstream-sync`.'
      )
    }
  }
}

// -- 3. Are the generated Rust tables stale? ---------------------------------
const generated = await generate(tag)
for (const [rel, body] of Object.entries(generated)) {
  const target = join(REPO, rel)
  if (!existsSync(target)) {
    problems.push(`${rel} is missing.\n    fix: \`just upstream-sync\`.`)
  } else if (readFileSync(target, 'utf8') !== body) {
    problems.push(
      `${rel} does not match what upstream ${tag} generates.\n`
      + '    fix: `just upstream-sync` (do not hand-edit generated files).'
    )
  }
}

// -- 4. Does every upstream test have a verdict? -----------------------------
const names = await upstreamTestNames(tag)
const rustTests = collectRustTestNames()
const rules = manifest.rules ?? []
const usedRules = new Set()
let mapped = 0
let unported = 0

for (const [file, entries] of Object.entries(names)) {
  for (const { name } of entries) {
    const idx = rules.findIndex(r => r.file === file && ruleMatches(r.match, name))
    if (idx === -1) {
      problems.push(
        `upstream test has no verdict: ${file} > "${name}".\n`
        + '    fix: add a rule to upstream/parity.json with either "rust" or "unported".'
      )
      continue
    }
    usedRules.add(idx)
    const rule = rules[idx]
    if (rule.rust) {
      mapped++
      for (const fn of [].concat(rule.rust)) {
        if (!rustTests.has(fn)) {
          problems.push(
            `parity.json maps ${file} > "${name}" to Rust test \`${fn}\`, `
            + 'which does not exist.\n    fix: rename the mapping or restore the test.'
          )
        }
      }
    } else if (rule.unported) {
      unported++
    } else {
      problems.push(
        `parity.json rule ${file} > "${rule.match}" has neither "rust" nor "unported".`
      )
    }
  }
}

// A rule that matches nothing means upstream dropped or renamed the test it was
// written for, so the Rust side may now be testing a behaviour nobody has.
rules.forEach((rule, idx) => {
  if (usedRules.has(idx) || rule.match === '*') {
    return
  }
  problems.push(
    `parity.json rule ${rule.file} > "${rule.match}" matches nothing at ${tag}.\n`
    + '    fix: drop it, and decide whether the Rust test it names still earns its keep.'
  )
})

notes.push(`${mapped} upstream tests mapped to Rust tests, ${unported} explicitly unported`)

/** Exact name, `prefix:...`, or `*`. */
function ruleMatches (match, name) {
  if (match === '*') {
    return true
  }
  if (match.startsWith('prefix:')) {
    return name.startsWith(match.slice('prefix:'.length))
  }
  return match === name
}

// -- report ------------------------------------------------------------------
console.log(`upstream parity check — pinned at ${tag}${skipTagProbe ? ' (tag probe skipped)' : ''}`)
for (const note of notes) {
  console.log(`  · ${note}`)
}
if (!problems.length) {
  console.log('\nno drift.')
  process.exit(0)
}
console.log(`\n${problems.length} problem${problems.length === 1 ? '' : 's'}:\n`)
for (const p of problems) {
  console.log(`  ✗ ${p}\n`)
}
process.exit(1)

/** Every `fn name` under a #[test] in tests/*.rs and src/*.rs. */
function collectRustTestNames () {
  const found = new Set()
  for (const dir of ['tests', 'src']) {
    const base = join(REPO, dir)
    if (!existsSync(base)) {
      continue
    }
    for (const file of walk(base)) {
      const source = readFileSync(file, 'utf8')
      for (const m of source.matchAll(/#\[test\][\s\S]{0,200}?\bfn\s+([a-z0-9_]+)/g)) {
        found.add(m[1])
      }
    }
  }
  return found
}

function * walk (dir) {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name)
    if (statSync(full).isDirectory()) {
      yield * walk(full)
    } else if (full.endsWith('.rs')) {
      yield full
    }
  }
}
