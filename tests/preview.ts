// Development-only fixture entry. It is not an input to the production bundle.
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks'
import type { Channel } from '@tauri-apps/api/core'
import type { RepositorySummary } from '../src/types/git'

Object.assign(globalThis, { isTauri: true })
mockWindows('main')
const previewData: Record<string,unknown> = {pins:[],visits:{},last:'',activities:[],clones:[]}
const clipboardProbe = document.createElement('output')
clipboardProbe.className = 'sr-only'
clipboardProbe.setAttribute('aria-label', '复制路径检查')
document.body.append(clipboardProbe)
Object.defineProperty(navigator, 'clipboard', { value: {
  async writeText(text: string) { clipboardProbe.value = text },
} })
const base: RepositorySummary = {
  id: 'blog', name: 'Aurora-Blog', path: String.raw`\\?\E:\Code\Aurora-Blog`, scanRoot: String.raw`\\?\E:\Code`,
  currentBranch: 'main', isDetached: false, upstream: 'origin/main',
  remotes: [{ name: 'origin', url: 'https://example.com/aurora/blog.git' }],
  localBranches: ['main', 'feature/design'], remoteBranches: ['origin/main', 'origin/feature/design', 'origin/dev'],
  ahead: 0, behind: 1, isDirty: true, hasConflicts: false, operationInProgress: false,
}
const repositories: RepositorySummary[] = [
  base,
  { ...base, id: 'scope', name: 'RepoDeck', path: String.raw`\\?\E:\Code\RepoDeck`, isDirty: false, behind: 3 },
  { ...base, id: 'skills', name: 'Aurora-Skills', path: String.raw`\\?\E:\Code\Aurora-Skills`, isDirty: false, behind: 0 },
]
// Clone preview simulates a stalled transfer until stopped; no disk or Git writes.
let pendingClone: { id: string; path: string; resolve: (value: unknown) => void } | null = null
mockIPC((command, payload) => {
  if (command === 'migrate_browser_data') return
  if (command === 'read_business_data') return previewData[String(payload?.key)] ?? null
  if (command === 'save_business_data') { previewData[String(payload?.key)] = payload?.value; return }
  if (command === 'read_clone_directories') return [String.raw`E:\Code`, String.raw`D:\Projects`]
  if (command === 'remember_clone_directory') return [String(payload?.path), String.raw`E:\Code`].filter((path, index, all) => all.indexOf(path) === index)
  if (command === 'clone_repository') {
    const progress = payload?.progress as Channel<string>
    progress.onmessage('remote: Compressing objects: 100% (115/115)\rReceiving objects: 50% (200/400)\r')
    return new Promise(resolve => { pendingClone = { id: String(payload?.id), path: `${payload?.parent}\\${payload?.name}`, resolve } })
  }
  if (command === 'stop_clone') {
    if (!pendingClone || pendingClone.id !== payload?.id) throw new Error('任务不存在')
    pendingClone.resolve({ path: pendingClone.path, stopped: true, deleted: payload?.deleteFiles })
    pendingClone = null
    return
  }
  if (command === 'read_repository_groups') return previewData.groups ?? []
  if (command === 'save_repository_groups') {previewData.groups=payload?.groups;return}
  if (command === 'get_data_location') return 'C:\\RepoDeck-dev'
  if (command === 'read_preferences') return null
  if (command === 'save_preferences') return
  if (command === 'scan_repositories') return { roots: [{ path: base.scanRoot }], repositories, issues: [] }
  if (command === 'get_repository_tree') return payload?.relativePath ? [{ name: 'App.vue', path: 'src/App.vue', isDirectory: false }] : [
    { name: 'src', path: 'src', isDirectory: true }, { name: 'package.json', path: 'package.json', isDirectory: false }, { name: 'README.md', path: 'README.md', isDirectory: false },
  ]
  if (command === 'get_worktree_changes') return payload?.path === base.path ? [
    { path: 'frontend-toolchain-report-20260916-215339.md', status: '??', staged: false, unstaged: false, untracked: true, conflicted: false },
    { path: 'src/views/HomeView.vue', status: ' M', staged: false, unstaged: true, untracked: false, conflicted: false },
    { path: 'README.md', status: ' M', staged: false, unstaged: true, untracked: false, conflicted: false },
  ] : []
  if (command === 'get_branch_commits') return Array.from({ length: Math.min(Number(payload?.limit ?? 10), 8) }, (_, index) => ({
    hash: `abc123${index}`, shortHash: `abc123${index}`, author: 'Aurora', committedAt: '2026-10-01T08:30:00+08:00',
    subject: ['优化项目导航与文章排版', '补充亮色与暗色主题配置', '调整移动端响应式布局', '修正代码块样式'][index % 4],
  }))
  if (command === 'fetch_repository') return
  throw new Error(`UI 预览不执行 Git 写入或目录变更：${command}`)
}, {shouldMockEvents:true})
await import('../src/main')
