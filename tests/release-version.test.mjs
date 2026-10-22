import { test } from 'node:test'
import assert from 'node:assert/strict'
import { compareVersions, newestRelease } from '../src/lib/release-version.ts'

test('compares stable and preview versions, including numeric beta suffixes', () => {
 assert.equal(compareVersions('v0.0.1-beta1', '0.0.1-beta1'), 0)
 for (const [a, b] of [['0.0.1', '0.0.1-beta10'], ['0.0.1-beta10', '0.0.1-beta2'], ['0.0.2-beta1', '0.0.1'], ['1.0.0-rc.2', '1.0.0-rc.1']]) {
  assert.ok(compareVersions(a, b) > 0)
  assert.ok(compareVersions(b, a) < 0)
 }
})

test('selects newest public version from release JSON, including previews', () => {
 assert.equal(newestRelease([{tag_name:'v0.0.1'}, {tag_name:'v0.0.2-beta1', prerelease:true}, {tag_name:'v9.0.0', draft:true}, {tag_name:'other'}]), 'v0.0.2-beta1')
 assert.equal(newestRelease([{tag_name:'v0.0.1-beta1', prerelease:true}]), 'v0.0.1-beta1')
 assert.equal(newestRelease([]), undefined)
 assert.throws(() => newestRelease({}))
})
