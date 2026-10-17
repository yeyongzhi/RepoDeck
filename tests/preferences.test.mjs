import { test } from 'node:test'
import assert from 'node:assert/strict'
import { displayPath } from '../src/lib/path.ts'
import { readPreferences } from '../src/lib/preferences.ts'

test('Windows drive and network labels hide the extended path prefix', () => {
  assert.equal(displayPath(String.raw`\\?\E:\Code\Aurora-Blog`), String.raw`E:\Code\Aurora-Blog`)
  assert.equal(displayPath(String.raw`\\?\UNC\server\share\项目`), String.raw`\\server\share\项目`)
  assert.equal(displayPath(String.raw`\\?\unc\server\share`), String.raw`\\server\share`)
  assert.equal(displayPath('\\\\?\\E:\\'), 'E:\\')
})

test('ordinary and other device paths keep their original meaning', () => {
  for (const path of ['/Users/aurora/Code/项目', String.raw`E:\Code\项目`, String.raw`\\server\share`, '\\\\?\\Volume{example}\\']) {
    assert.equal(displayPath(path), path)
  }
})

test('corrupt or unknown saved settings cannot force dark mode or break startup', () => {
  for (const raw of ['{broken', 'null', '[]', '{"theme":"invalid","accent":"invalid"}']) {
    assert.deepEqual(readPreferences(raw), { theme: 'system', accent: 'blue', commitLimit: 10 })
  }
})

test('saved appearance is restored and legacy commit limits migrate within bounds', () => {
  assert.deepEqual(readPreferences('{"theme":"dark","accent":"violet","commitLimit":200}'), { theme: 'dark', accent: 'violet', commitLimit: 100 })
  assert.equal(readPreferences(null, '25').commitLimit, 25)
  assert.equal(readPreferences(null, 'oops').commitLimit, 10)
  assert.equal(readPreferences('{"commitLimit":-1}').commitLimit, 1)
})
