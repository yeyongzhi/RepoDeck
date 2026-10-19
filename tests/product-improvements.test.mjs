import { test } from 'node:test'
import assert from 'node:assert/strict'
import { explainGitError } from '../src/lib/git-errors.ts'
import { actionReason } from '../src/lib/repository-rules.ts'
const clean = {hasConflicts:false,operationInProgress:false,isDirty:false,isDetached:false,upstream:'origin/main',ahead:0,behind:1}
test('operation explanations match actionable blocked states', () => {
 assert.equal(actionReason(clean,'pull'),'')
 assert.match(actionReason({...clean,isDirty:true},'pull'),/Stash/)
 assert.match(actionReason({...clean,upstream:null},'push'),/关联/)
 assert.match(actionReason({...clean,behind:0},'push'),/没有待推送/)
 assert.match(actionReason({...clean,hasConflicts:true},'checkout'),/冲突/)
})
test('Git failures receive distinct causes and next steps', () => {
 assert.equal(explainGitError('fatal: Authentication failed').title,'远程认证失败')
 assert.equal(explainGitError('Could not resolve host').title,'无法连接远程仓库')
 assert.match(explainGitError('destination path already exists').action,/更换/)
 assert.match(explainGitError('Host key verification failed').action,/信任/)
})
