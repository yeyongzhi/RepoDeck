import { invoke as nativeInvoke } from '@tauri-apps/api/core'
import { ref } from 'vue'
import { toast } from 'vue-sonner'
import { businessValues, saveBusiness } from '@/lib/business'
import { explainGitError } from '@/lib/git-errors'

export interface GitActivity { id: string; path: string; label: string; at: string; status: 'running' | 'success' | 'error'; detail: string }
export const gitActivities = ref<GitActivity[]>([])
export async function initializeActivities() {
  gitActivities.value = ((businessValues.activities as GitActivity[]) ?? []).map(item => item.status === 'running' ? {...item,status:'error',detail:'上次退出时任务未完成，结果未知，请刷新仓库确认'} : item)
  await saveBusiness('activities',gitActivities.value)
}
let activityLimit = 300
export function setActivityLimit(limit: number) { activityLimit = limit; gitActivities.value = gitActivities.value.filter(item => item.status === 'running').concat(gitActivities.value.filter(item => item.status !== 'running').slice(0, limit)); void persist().catch(() => {}) }
export function clearGitActivities() { gitActivities.value = gitActivities.value.filter(item => item.status === 'running'); void persist().catch(() => {}) }
const labels: Record<string, string> = { fetch_repository: 'Fetch 获取远端', pull_fast_forward: 'Pull 快进拉取', push_current_branch: 'Push 推送分支', integrate_upstream: '整合远端提交', checkout_branch: '切换分支', create_branch: '创建分支', merge_branch: '合并分支', delete_local_branch: '删除分支', set_upstream: '设置关联分支', stage_files: '暂存文件', commit_files: '提交改动', 'rename': '重命名分支', upstream: '设置关联分支', 'unset-upstream': '取消关联分支', publish: '发布本地分支', 'tag-branch': '从标签创建分支', 'stash-save': '保存 Stash', 'stash-apply': '恢复 Stash', 'stash-drop': '删除 Stash' }
function persist() { return saveBusiness('activities',gitActivities.value) }
export async function invoke<T = void>(command: string, args?: Record<string, unknown>): Promise<T> {
  const label = labels[command === 'repository_action' ? String(args?.kind) : command]
  if (!label || typeof args?.path !== 'string') return nativeInvoke<T>(command, args)
  const entry: GitActivity = { id: crypto.randomUUID(), path: args.path, label, at: new Date().toISOString(), status: 'running', detail: '' }
  gitActivities.value = [entry, ...gitActivities.value].slice(0, activityLimit)
  try { await persist() } catch (e) { Object.assign(gitActivities.value.find(item => item.id === entry.id) ?? entry, {status:'error',detail:'任务记录保存失败，未执行 Git 操作'}); throw e }
  try {
    const result = await nativeInvoke<T>(command, args)
    Object.assign(gitActivities.value.find(item => item.id === entry.id) ?? entry, { status: 'success', detail: typeof result === 'string' && result ? result : '操作完成' })
    await persist().catch(e => toast.error(`操作已完成，但任务记录保存失败：${e}`))
    return result
  } catch (error) {
    Object.assign(gitActivities.value.find(item => item.id === entry.id) ?? entry, { status: 'error', detail: String(error) })
    await persist().catch(e => toast.error(`任务记录保存失败：${e}`))
    throw new Error(explainGitError(String(error)).title + '：' + explainGitError(String(error)).action + '\n\n详细日志：\n' + String(error))
  }
}
