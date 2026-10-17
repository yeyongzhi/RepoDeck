import { test } from 'node:test'
import assert from 'node:assert/strict'
import { remoteWebUrl, remoteBranchUrl } from '../src/lib/remote-url.ts'
test('Git remotes normalize to credential-free web URLs', () => {
 assert.equal(remoteWebUrl('git@github.com:owner/repo.git'), 'https://github.com/owner/repo')
 assert.equal(remoteWebUrl('ssh://git@gitlab.com/group/repo.git'), 'https://gitlab.com/group/repo')
 assert.equal(remoteWebUrl('https://user:secret@github.com/owner/repo.git'), 'https://github.com/owner/repo')
 assert.equal(remoteWebUrl('file:///tmp/project'), null)
})
test('branch links encode branch names and use provider routes', () => {
 assert.equal(remoteBranchUrl('git@github.com:owner/repo.git', 'feature/ui'), 'https://github.com/owner/repo/tree/feature%2Fui')
 assert.equal(remoteBranchUrl('https://gitlab.com/group/repo.git', 'dev'), 'https://gitlab.com/group/repo/-/tree/dev')
 assert.equal(remoteBranchUrl('https://example.com/repo.git', 'main'), null)
 assert.equal(remoteBranchUrl('https://github.com/owner/repo', null), null)
})
