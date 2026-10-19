import { test } from 'node:test'
import assert from 'node:assert/strict'
import { cloneProgress } from '../src/lib/clone-progress.ts'
test('overall progress spans Git stages and never reaches 100 before completion', () => {
  assert.equal(cloneProgress('remote: Compressing objects: 50%'), 5)
  assert.equal(cloneProgress('Receiving objects: 50%'), 45)
  assert.equal(cloneProgress('Resolving deltas: 100%'), 95)
  assert.equal(cloneProgress('Updating files: 100%'), 99)
  assert.equal(cloneProgress('remote: Counting objects: 10%', 80), 80)
  assert.equal(cloneProgress('connecting…'), 0)
})
