#!/usr/bin/env node
import { basename } from 'node:path'
import { proc, toARSM, getSubscriptions } from '../lib/index.js'
import { readFileSync, writeFileSync } from 'node:fs'
import { execa } from 'execa'

const [major, minor, patch] = process.versions.node.split('.')
if (major < 19) {
  console.error('detected node.js', process.version)
  console.error('this tool requires at least node.js version 19')
  process.exit(1)
}

const args = process.argv
const [node, cli, dst] = args.splice(0, 3)
if (dst === undefined || args.length > 0) {
  console.error(`usage: ${basename(node)} ${basename(cli)} <output>`)
  process.exit(1)
}

const HEADER = '// auto-generated by machine-check, do not edit'

try {
  const c = readFileSync(dst)
  if (!c.toString('utf-8', 0, 1000).startsWith(HEADER)) {
    console.error(`cowardly refusing to overwrite non-generated file '${dst}'`)
    process.exit(1)
  }
} catch (e) {
  if (e.code !== 'ENOENT') {
    console.error(e.toString())
    process.exit(1)
  } else {
    writeFileSync(
      dst,
      `${HEADER}
export default function setup() { return 'done' }`,
    )
  }
}

const extracted = proc()

writeFileSync(
  dst,
  `${HEADER}
import { init } from 'machine-runner'
init(${JSON.stringify(extracted)})
export default function setup() { return 'done' }`,
)
