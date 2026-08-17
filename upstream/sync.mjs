#!/usr/bin/env node
// Regenerate the upstream-derived Rust case tables and the parity manifest's
// bookkeeping fields.
//
//   node upstream/sync.mjs              regenerate at the pinned version
//   node upstream/sync.mjs --bump       repin to the newest config-v* tag first
//
// Writing files is all this does. Deciding whether a change is *correct* is the
// job of the reviewer (or `/upstream-sync`), not of this script.

import { writeFileSync, mkdirSync, readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import {
  REPO, generate, pinnedVersion, latestVersion, upstreamTestNames, fileHashes, readManifest,
} from './lib.mjs'

const bump = process.argv.includes('--bump')

let tag = pinnedVersion()
if (bump) {
  const latest = await latestVersion()
  if (latest !== tag) {
    console.log(`repinning ${tag} -> ${latest}`)
    writeFileSync(join(REPO, 'upstream', 'UPSTREAM_VERSION'), `${latest}\n`)
    tag = latest
  } else {
    console.log(`already at the newest tag (${tag})`)
  }
}

const generated = await generate(tag)
for (const [rel, body] of Object.entries(generated)) {
  const target = join(REPO, rel)
  mkdirSync(dirname(target), { recursive: true })
  writeFileSync(target, body)
  console.log(`wrote ${rel}`)
}

// Refresh the manifest's derived fields. Rules are hand-written and never
// rewritten here: deciding what a new upstream test means is a review decision,
// so unmatched tests get reported rather than auto-absorbed.
const manifest = readManifest()
manifest.upstream_version = tag
manifest.upstream_file_hashes = await fileHashes(tag)
writeFileSync(join(REPO, 'upstream', 'parity.json'), `${JSON.stringify(manifest, null, 2)}\n`)
console.log('wrote upstream/parity.json (version + hashes)')

const names = await upstreamTestNames(tag)
const unmatched = []
for (const [file, entries] of Object.entries(names)) {
  for (const { name } of entries) {
    const covered = (manifest.rules ?? []).some(r => r.file === file && (
      r.match === '*'
      || (r.match.startsWith('prefix:') && name.startsWith(r.match.slice('prefix:'.length)))
      || r.match === name
    ))
    if (!covered) {
      unmatched.push(`${file} > "${name}"`)
    }
  }
}
if (unmatched.length) {
  console.log(`\n${unmatched.length} upstream test(s) have no rule in parity.json:`)
  for (const u of unmatched) {
    console.log(`  ${u}`)
  }
  console.log('\nAdd a rule with either "rust": "<test_fn>" or "unported": "<reason>".')
}

// Keep the crate's recorded upstream version honest.
const readme = join(REPO, 'README.md')
const before = readFileSync(readme, 'utf8')
const after = before.replace(/@npmcli\/config v\d+\.\d+\.\d+/g, `@npmcli/config v${tag.replace('config-v', '')}`)
  .replace(/tree\/config-v\d+\.\d+\.\d+/g, `tree/${tag}`)
if (after !== before) {
  writeFileSync(readme, after)
  console.log('wrote README.md (upstream version reference)')
}
