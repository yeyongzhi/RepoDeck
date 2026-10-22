<script setup lang="ts">
import { isTauri } from '@tauri-apps/api/core'
import { invoke } from '@/lib/git-client'
import { open } from '@tauri-apps/plugin-dialog'
import {
  ArrowLeftIcon,
  CalendarDaysIcon,
  ExternalLinkIcon,
  FolderOpenIcon,
  GitBranchIcon,
  GlobeIcon,
  LayoutGridIcon,
  ListIcon,
  CheckIcon,
  CircleAlertIcon,
  CircleCheckIcon,
  FolderGit2Icon,
  GitCommitHorizontalIcon,
  GitPullRequestIcon,
  LoaderCircleIcon,
  RefreshCwIcon,
  PlusIcon,
  SearchIcon,
  TriangleAlertIcon,
} from '@lucide/vue'
import { computed, onMounted, ref, watch } from 'vue'
import { watchDebounced } from '@vueuse/core'
import { onBeforeRouteLeave, onBeforeRouteUpdate, useRoute, useRouter } from 'vue-router'
import { toast } from 'vue-sonner'
import { displayPath } from '@/lib/path'
import { business, visit, togglePin, initializeBusiness } from '@/lib/business'
import { cn } from '@/lib/utils'
import { useSettingsStore } from '@/stores/settings'
import { formatAppDate } from '@/lib/app-options'
import { useWorkspaceStore } from '@/stores/workspace'
import type { BranchCommit, ChangedFile, RepositorySummary, TreeEntry } from '@/types/git'
import { remoteWebUrl, remoteBranchUrl, remoteCommitUrl } from '@/lib/remote-url'
import { Item, ItemGroup, ItemContent, ItemTitle, ItemDescription, ItemActions, ItemMedia } from '@/components/ui/item'
import { ScrollArea } from '@/components/ui/scroll-area'
import AppTooltip from '@/components/AppTooltip.vue'
import DatePicker from '@/components/DatePicker.vue'
import RepoTree from '@/components/RepoTree.vue'
import ChangeStatusBadge from '@/components/ChangeStatusBadge.vue'
import { useGroupsStore } from '@/stores/groups'
import GroupBatchFetch from '@/components/GroupBatchFetch.vue'
import { useCloneStore } from '@/stores/clone'
import AddRepositoryMenu from '@/components/AddRepositoryMenu.vue'
import RepositoryCard from '@/components/RepositoryCard.vue'
import BranchPanel from '@/components/BranchPanel.vue'
import RepositoryTools from '@/components/RepositoryTools.vue'
import RepositoryActions from '@/components/RepositoryActions.vue'
import RepositoryDiff from '@/components/RepositoryDiff.vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import { Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from '@/components/ui/empty'
import { Input } from '@/components/ui/input'
import { InputGroup, InputGroupAddon, InputGroupInput } from '@/components/ui/input-group'
import { Checkbox } from '@/components/ui/checkbox'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Field, FieldGroup, FieldLabel, FieldLegend, FieldSet } from '@/components/ui/field'
import { Select, SelectContent, SelectGroup, SelectItem, SelectLabel, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'

const workspace = useWorkspaceStore()
const clone = useCloneStore()
const settings = useSettingsStore()
const groups = useGroupsStore()
const activeGroupId = computed(() => typeof route.query.group === 'string' ? route.query.group : '')
const activeGroup = computed(() => groups.groups.find(item => item.id === activeGroupId.value))
const detailTab = ref(settings.options.repositoryTab)
const recentRepositories = computed(() => business.visits)
const desktop = isTauri()
const route = useRoute()
const router = useRouter()
const isDetailPage = computed(() => route.name === 'repository')
const layout = ref<'grid' | 'list'>('grid')
layout.value = settings.rememberedLayout
if (settings.options.repositoryLayout !== 'remember') layout.value = settings.options.repositoryLayout as 'grid' | 'list'
watch(() => settings.options.repositoryLayout, value => { if (value !== 'remember') layout.value = value as 'grid' | 'list' })
watch(() => settings.options.repositoryTab, value => { detailTab.value = value })
watch(() => route.params.id, id => {
 if (typeof id !== 'string') return
 detailTab.value = settings.options.repositoryTab
 visit(id)
}, { immediate: true })
function setLayout(value: unknown) {
  if (value !== 'grid' && value !== 'list') return
  layout.value = value
  settings.rememberedLayout = value
}
const search = ref('')
const statusFilters = ref<string[]>([])
const filterOptions = [{id:'dirty',label:'有改动'},{id:'pull',label:'待拉取'},{id:'push',label:'待推送'},{id:'conflict',label:'有冲突'},{id:'pinned',label:'已置顶'}]
function toggleFilter(id: string) { statusFilters.value = statusFilters.value.includes(id) ? statusFilters.value.filter(item => item !== id) : [...statusFilters.value,id] }
const selectedId = ref('')
const checkedRepositoryIds = ref<string[]>([])
const selectedRepository = computed(() => workspace.repositories.find((repo) => repo.id === (isDetailPage.value ? String(route.params.id) : selectedId.value)) ?? null)
const trackingRemote = computed(() => [...(selectedRepository.value?.remotes ?? [])].sort((a,b) => b.name.length - a.name.length).find(remote => selectedRepository.value?.upstream?.startsWith(remote.name + '/')))
const repositoryDates = ref<[number, number] | null>(null)
const repositoryDatesError = ref(false)
function formatRepositoryDate(timestamp: number) { return formatDate(new Date(timestamp * 1000).toISOString()) }
const overviewItems = computed(() => {
 const repo = selectedRepository.value
 if (!repo) return []
 const tracked = trackingRemote.value
 const trackedBranch = repo.upstream && tracked ? repo.upstream.slice(tracked.name.length + 1) : null
 return [
  {label: '本地目录', value: displayPath(repo.path), target: repo.path, local: true, tooltip: '打开本地目录'},
  ...((repo.remotes.length ? repo.remotes : [{name: '', url: ''}]).map(item => ({label: repo.remotes.length > 1 ? `仓库地址 · ${item.name}` : '仓库地址', value: item.url || '未配置远程仓库', target: remoteWebUrl(item.url), local: false, tooltip: '打开远程仓库'}))),
  {branchInfo: {current: repo.currentBranch ?? 'Detached HEAD', upstream: repo.upstream ?? '未设置关联分支', remoteUrl: tracked?.url}, label: '当前分支 / 关联分支', value: `${repo.currentBranch ?? 'Detached HEAD'} / ${repo.upstream ?? '未设置关联分支'}`, target: remoteBranchUrl(tracked?.url ?? '', trackedBranch), local: false, tooltip: '打开关联远程分支'},
 ]
})
async function openOverviewItem(item: {target: string | null; local: boolean}) {
 if (!item.target) return
 try {
  if (desktop) await invoke('open_repository_target', {target: item.target, local: item.local})
  else if (!item.local) window.open(item.target, '_blank', 'noopener,noreferrer')
  else { toast.info('请在桌面应用中打开本地目录'); return }
  toast.success(item.local ? '已请求打开目录' : '已请求打开链接')
 } catch (reason) { toast.error(String(reason)) }
}
const activeRoot = computed(() => String(route.query.root ?? ''))
const collection = computed(() => activeGroupId.value ? workspace.repositories.filter(repo => activeGroup.value?.members.includes(repo.id)) : workspace.repositories.filter(repo => !activeRoot.value || repo.scanRoot === activeRoot.value || displayPath(repo.path).toLowerCase().replaceAll('\\', '/').startsWith(displayPath(activeRoot.value).toLowerCase().replaceAll('\\', '/') + '/')))
const collectionTitle = computed(() => activeGroupId.value ? activeGroup.value?.name ?? '分组不存在' : activeRoot.value ? displayPath(activeRoot.value).split(/[\\/]/).filter(Boolean).at(-1) : '全部仓库')
const filteredRepositories = computed(() => {
  const term = search.value.trim().toLowerCase()
  const sorted = [...collection.value].sort((a, b) => {
    const pinned = Number(business.pins.includes(b.id)) - Number(business.pins.includes(a.id))
    if (pinned) return pinned
    if (business.pins.includes(a.id) && business.pins.includes(b.id)) return business.pins.indexOf(a.id) - business.pins.indexOf(b.id)
    if (settings.options.repositorySort === 'recent') return (recentRepositories.value[b.id] ?? 0) - (recentRepositories.value[a.id] ?? 0) || a.name.localeCompare(b.name)
    if (settings.options.repositorySort === 'status') return Number(b.hasConflicts) - Number(a.hasConflicts) || Number(b.isDirty) - Number(a.isDirty) || (b.ahead + b.behind) - (a.ahead + a.behind) || a.name.localeCompare(b.name)
    return a.name.localeCompare(b.name, 'zh-CN', {numeric: true})
  })
  return sorted.filter(repo => !statusFilters.value.length || statusFilters.value.every(filter => filter === 'dirty' ? repo.isDirty : filter === 'pull' ? repo.behind > 0 : filter === 'push' ? repo.ahead > 0 : filter === 'conflict' ? repo.hasConflicts : business.pins.includes(repo.id))).filter((repo) => `${repo.name} ${repo.path} ${repo.currentBranch ?? ''}`.toLowerCase().includes(term))
})
const selectedCount = computed(() => checkedRepositoryIds.value.length)
const allVisibleSelected = computed(() => !!filteredRepositories.value.length && filteredRepositories.value.every((repo) => checkedRepositoryIds.value.includes(repo.id)))

const busy = ref(false)
watch(busy, (active) => { workspace.operationBusy = active }, { flush: 'sync' })
const busyLabel = ref('')
const branchChoice = ref('')
const branchCommits = ref<BranchCommit[]>([])
const commitSearch = ref('')
const commitDateFrom = ref('')
const commitDateTo = ref('')
const diffOpen = ref(false)
const diffKind = ref('commit')
const diffReference = ref('')
function viewDiff(kind: string, reference: string) { diffKind.value = kind; diffReference.value = reference; diffOpen.value = true }
const commitPage = ref(1)
const commitPageSize = 10
const hasNextCommitPage = ref(false)
const filteredCommits = computed(() => branchCommits.value)
const commitsLoading = ref(false)
const commitsError = ref('')
function commitUrl(hash: string) {
 const repo = selectedRepository.value
 const remote = repo?.remotes.find(item => branchChoice.value.startsWith(item.name + '/')) ?? trackingRemote.value ?? repo?.remotes.find(item => item.name === 'origin') ?? repo?.remotes[0]
 return remoteCommitUrl(remote?.url ?? '', hash)
}
const hasCommitFilters = computed(() => !!(commitSearch.value.trim() || commitDateFrom.value || commitDateTo.value))
const latestCommit = ref<BranchCommit | null>(null)
const latestCommitError = ref('')
const summaryLoading = ref(false)
const worktreeCounts = computed(() => ({
 staged: changes.value.filter(file => file.staged).length,
 unstaged: changes.value.filter(file => file.unstaged).length,
 untracked: changes.value.filter(file => file.untracked).length,
}))
const syncLabel = computed(() => {
 const repo = selectedRepository.value
 if (!repo) return '—'
 if (repo.isDetached) return 'Detached HEAD'
 if (!repo.upstream) return '未设置跟踪分支'
 if (repo.ahead && repo.behind) return '本地与远端已分叉'
 if (repo.ahead) return `${repo.ahead} 个提交待推送`
 if (repo.behind) return `${repo.behind} 个提交待拉取`
 return '与远端引用一致'
})
let detailsRequest = 0
let commitsRequest = 0
const treeEntries = ref<TreeEntry[]>([])
const treeError = ref('')
const changes = ref<ChangedFile[]>([])
const selectedFiles = ref<string[]>([])
const commitMessage = ref('')
let commitRepositoryId = ''

const batchResults = ref<{ id: string; name: string; state: 'success' | 'attention' | 'failed' | 'skipped'; message: string }[]>([])
const dialogOpen = ref(false)
const dialogAction = ref<'checkout' | 'merge' | 'rebase' | null>(null)
const checkoutChoice = ref('')
const newLocalBranch = ref('')
const checkoutRemote = computed(() => checkoutChoice.value.startsWith('remote:') ? checkoutChoice.value.slice(7) : null)
const checkoutLocal = computed(() => checkoutChoice.value.startsWith('local:') ? checkoutChoice.value.slice(6) : '')
const checkoutBlockedReason = computed(() => {
  const repo = selectedRepository.value
  if (!repo) return '请先选择仓库'
  if (repo.hasConflicts) return '存在合并冲突，请先在 IDE 或终端解决冲突，再刷新'
  if (repo.operationInProgress) return 'Git 操作尚未完成，请先在 IDE 或终端完成，再刷新'
  if (repo.isDirty) return '工作区有未提交改动，请先审阅并提交，或在其他 Git 工具中处理后刷新'
  if (!checkoutChoice.value) return '当前没有可切换的分支，请先 Fetch 获取远端分支'
  if (checkoutRemote.value) {
    if (!repo.remoteBranches.includes(checkoutRemote.value)) return '所选远端分支已不存在，请重新选择'
    const name = newLocalBranch.value.trim()
    if (!name) return '请填写新建的本地分支名'
    if (repo.localBranches.includes(name)) return '该本地分支已存在，请在本地分支列表中选择它，或使用其他名称'
  } else {
    if (!repo.localBranches.includes(checkoutLocal.value)) return '所选本地分支已不存在，请重新选择'
    if (checkoutLocal.value === repo.currentBranch) return '已处于该分支，请选择其他分支'
  }
  return ''
})
watch(checkoutRemote, (remoteBranch) => {
  if (!remoteBranch) return
  const remote = [...(selectedRepository.value?.remotes ?? [])]
    .sort((a, b) => b.name.length - a.name.length)
    .find((item) => remoteBranch.startsWith(`${item.name}/`))
  newLocalBranch.value = remoteBranch.slice((remote?.name.length ?? remoteBranch.indexOf('/')) + 1)
})
const trackedBranchChoice = ref('')

const branchNames = computed(() => {
  const repo = selectedRepository.value
  if (!repo) return []
  return [...new Set([...repo.localBranches, ...repo.remoteBranches])]
})
function setNotice(text: string, kind: 'success' | 'error' | 'info' = 'info') {
  toast[kind](text)
}

function getRepo(id: string) {
  return workspace.repositories.find((repo) => repo.id === id)
}

async function loadDetails(repo: RepositorySummary | null) {
  const request = ++detailsRequest
  treeEntries.value = []
  treeError.value = ''
  branchCommits.value = []
  commitPage.value = 1
  hasNextCommitPage.value = false
  latestCommit.value = null
  commitSearch.value = ''
  commitDateFrom.value = ''
  commitDateTo.value = ''
  repositoryDates.value = null
  repositoryDatesError.value = false
  latestCommitError.value = ''
  changes.value = []
  selectedFiles.value = []
  if (!repo || !desktop) return
  summaryLoading.value = true
  const datesRequest = invoke<[number, number] | null>('get_repository_dates', {path: repo.path})
    .then(dates => { if (request === detailsRequest) repositoryDates.value = dates })
    .catch(() => { if (request === detailsRequest) repositoryDatesError.value = true })
  const latestRequest = invoke<BranchCommit[]>('get_branch_commits', {path: repo.path, branch: 'HEAD', limit: 1})
    .then(commits => { if (request === detailsRequest) latestCommit.value = commits[0] ?? null })
    .catch(reason => { if (request === detailsRequest) latestCommitError.value = String(reason) })
  try {
    const [tree, modified] = await Promise.all([
      invoke<TreeEntry[]>('get_repository_tree', { path: repo.path, relativePath: null }),
      invoke<ChangedFile[]>('get_worktree_changes', { path: repo.path }),
    ])
    if (request !== detailsRequest || selectedRepository.value?.id !== repo.id) return
    treeEntries.value = tree
    changes.value = modified
  } catch (reason) {
    if (request !== detailsRequest || selectedRepository.value?.id !== repo.id) return
    treeError.value = String(reason)
  }
  await Promise.all([latestRequest, datesRequest])
  if (request !== detailsRequest || selectedRepository.value?.id !== repo.id) return
  summaryLoading.value = false
  branchChoice.value = repo.currentBranch ?? branchNames.value[0] ?? ''
  trackedBranchChoice.value = repo.remoteBranches[0] ?? ''
  await loadCommits(repo.path, branchChoice.value)
}

async function loadCommits(path = selectedRepository.value?.path, branch = branchChoice.value) {
  const request = ++commitsRequest
  if (!desktop || !path || !branch) {
    branchCommits.value = []
    return
  }
  commitsLoading.value = true
  commitsError.value = ''
  try {
    const commits = await invoke<BranchCommit[]>('get_branch_commits', {
      path,
      branch,
      since: commitDateFrom.value ? new Date(`${commitDateFrom.value}T00:00:00`).toISOString() : null,
      until: commitDateTo.value ? new Date(`${commitDateTo.value}T23:59:59.999`).toISOString() : null,
      search: commitSearch.value.trim() || null,
      limit: commitPageSize + 1,
      skip: (commitPage.value - 1) * commitPageSize,
    })
    if (request === commitsRequest && path === selectedRepository.value?.path) {
      hasNextCommitPage.value = commits.length > commitPageSize
      branchCommits.value = commits.slice(0, commitPageSize)
    }
  } catch {
    if (request === commitsRequest) { branchCommits.value = []; commitsError.value = '读取提交失败，请刷新后重试' }
  } finally {
    if (request === commitsRequest) commitsLoading.value = false
  }
}

watch([activeGroupId, activeRoot], () => { checkedRepositoryIds.value = []; batchResults.value = [] })
watch(() => isDetailPage.value ? selectedRepository.value?.id : undefined, (id) => {
  void loadDetails(getRepo(id ?? '') ?? null)
}, { immediate: true })
watch(() => workspace.repositories.map((repo) => repo.id).join('\u0000'), () => {
  const visible = new Set(workspace.repositories.map((repo) => repo.id))
  checkedRepositoryIds.value = checkedRepositoryIds.value.filter((id) => visible.has(id))
})
watch(branchChoice, () => { commitPage.value = 1; void loadCommits() })
watch(commitPage, () => void loadCommits())
watchDebounced([commitSearch, commitDateFrom, commitDateTo], () => { commitPage.value = 1; void loadCommits() }, { debounce: 300 })
watch(() => selectedRepository.value?.currentBranch, (branch) => {
  if (branch && branch !== branchChoice.value) branchChoice.value = branch
})


onMounted(() => {
  if (!desktop) {
    setNotice('请通过 Tauri 桌面应用使用本机目录与 Git 功能', 'info')
    return
  }
})

function canNavigate() {
  if (!busy.value) return true
  setNotice('当前操作仍在进行，请完成后再切换页面')
  return false
}
onBeforeRouteLeave(canNavigate)
onBeforeRouteUpdate(canNavigate)

async function refreshAll() {
  if (!desktop) return setNotice('请在桌面应用中刷新本机仓库')
  try {
    await workspace.refresh(true)
    if (workspace.error) return setNotice(`刷新失败：${workspace.error}`, 'error')
    await loadDetails(selectedRepository.value)
    if (selectedRepository.value && (treeError.value || latestCommitError.value || repositoryDatesError.value || commitsError.value)) {
      return setNotice('仓库列表已刷新，部分详情读取失败，请重试', 'error')
    }
    if (workspace.issues.length) return setNotice('刷新完成，部分目录无法访问，请查看扫描详情')
    setNotice(`刷新成功，已更新 ${workspace.repositories.length} 个仓库`, 'success')
  } catch (reason) { setNotice(`刷新失败：${String(reason)}`, 'error') }
}

async function relocate(path: string) {
  try {
    const next = await open({directory:true,multiple:false,title:'选择仓库的新位置（仅更新登记，不移动文件）'})
    if (typeof next !== 'string') return
    await invoke('relocate_repository',{oldPath:path,newPath:next})
    await initializeBusiness(); await workspace.refresh(); await groups.initialize(true)
    toast.success('已更新仓库位置，分组和置顶已保留')
  } catch(e) { toast.error(String(e)) }
}
async function addDirectory() {
  if (!desktop) return setNotice('目录选择器只在桌面应用中可用')
  try {
    const path = await open({ directory: true, multiple: false, title: '选择 Git 仓库根目录' })
    if (typeof path !== 'string') return
    const added = await workspace.addRoot(path, 'repository')
    if (workspace.error) throw new Error(workspace.error)
    setNotice(added ? '仓库已添加' : '仓库已登记，状态已更新', added ? 'success' : 'info')
  } catch (reason) {
    setNotice(String(reason), 'error')
  }
}

function toggleRepository(id: string) {
  checkedRepositoryIds.value = checkedRepositoryIds.value.includes(id)
    ? checkedRepositoryIds.value.filter((item) => item !== id)
    : [...checkedRepositoryIds.value, id]
}

function toggleAllRepositories() {
  const visibleIds = filteredRepositories.value.map((repo) => repo.id)
  checkedRepositoryIds.value = allVisibleSelected.value
    ? checkedRepositoryIds.value.filter((id) => !visibleIds.includes(id))
    : [...new Set([...checkedRepositoryIds.value, ...visibleIds])]
}

async function runSingle(label: string, action: () => Promise<unknown>) {
  if (!selectedRepository.value) return
  busy.value = true
  busyLabel.value = label
  try {
    await action()
    await workspace.refresh()
    await refreshChanges().catch(() => undefined)
    await loadCommits(selectedRepository.value?.path, branchChoice.value)
    setNotice(`${selectedRepository.value?.name ?? '仓库'}：${label}完成`, 'success')
  } catch (reason) {
    await workspace.refresh()
    await refreshChanges().catch(() => undefined)
    await loadCommits(selectedRepository.value?.path, branchChoice.value)
    setNotice(String(reason), 'error')
  } finally {
    busy.value = false
    busyLabel.value = ''
  }
}

async function fetchOne() {
  const repo = selectedRepository.value
  if (!repo) return
  await runSingle('获取远端更新', () => invoke('fetch_repository', { path: repo.path }))
}

async function pullOne() {
  const repo = selectedRepository.value
  if (!repo) return
  await runSingle('快进拉取', async () => {
    await invoke('fetch_repository', { path: repo.path })
    await workspace.refresh()
    const latest = getRepo(repo.id)
    if (!latest || latest.isDirty || latest.isDetached || !latest.upstream || latest.ahead !== 0 || latest.behind === 0) {
      throw new Error('Fetch 后状态已变化；没有自动拉取，请按新的分支状态选择操作')
    }
    await invoke('pull_fast_forward', { path: repo.path })
  })
}

async function pushOne() {
  const repo = selectedRepository.value
  if (!repo) return
  await runSingle('推送当前分支', () => invoke('push_current_branch', { path: repo.path }))
}

async function syncSelected() {
  if (!selectedCount.value || busy.value) return
  const targets = checkedRepositoryIds.value.map(getRepo).filter((repo): repo is RepositorySummary => !!repo)
  busy.value = true
  busyLabel.value = '批量检查与同步'
  batchResults.value = []
  for (const repo of targets) {
    try {
      await invoke('fetch_repository', { path: repo.path })
      await workspace.refresh()
      const latest = getRepo(repo.id)
      if (!latest) {
        batchResults.value.push({ id: repo.id, name: repo.name, state: 'failed', message: '扫描后未找到仓库' })
      } else if (latest.hasConflicts) {
        batchResults.value.push({ id: repo.id, name: repo.name, state: 'attention', message: '仓库中存在合并冲突；请在 IDE 或终端解决后再继续' })
      } else if (latest.operationInProgress) {
        batchResults.value.push({ id: repo.id, name: repo.name, state: 'attention', message: 'Git 合并/变基仍在进行；请在 IDE 或终端完成后再同步' })
      } else if (latest.isDirty) {
        batchResults.value.push({ id: repo.id, name: repo.name, state: 'attention', message: '工作区有改动；请审阅、提交或跳过' })
      } else if (latest.isDetached) {
        batchResults.value.push({ id: repo.id, name: repo.name, state: 'attention', message: '当前为 Detached HEAD；需要先选择分支' })
      } else if (!latest.upstream) {
        batchResults.value.push({ id: repo.id, name: repo.name, state: 'attention', message: '没有远端跟踪分支；请先选择 upstream' })
      } else if (latest.ahead === 0 && latest.behind > 0) {
        await invoke('pull_fast_forward', { path: repo.path })
        batchResults.value.push({ id: repo.id, name: repo.name, state: 'success', message: `已快进 ${latest.behind} 个提交` })
      } else if (latest.ahead === 0 && latest.behind === 0) {
        batchResults.value.push({ id: repo.id, name: repo.name, state: 'success', message: '已同步，无需拉取' })
      } else if (latest.ahead > 0 && latest.behind > 0) {
        batchResults.value.push({ id: repo.id, name: repo.name, state: 'attention', message: `已分叉：本地领先 ${latest.ahead}，远端领先 ${latest.behind}。请选合并、变基或跳过` })
      } else {
        batchResults.value.push({ id: repo.id, name: repo.name, state: 'attention', message: `本地领先 ${latest.ahead} 个提交；可推送或跳过` })
      }
    } catch (reason) {
      batchResults.value.push({ id: repo.id, name: repo.name, state: 'failed', message: String(reason) })
    }
  }
  await workspace.refresh()
  await loadCommits(selectedRepository.value?.path, branchChoice.value)
  busy.value = false
  busyLabel.value = ''
  const done = batchResults.value.filter((result) => result.state === 'success').length
  const attention = batchResults.value.filter((result) => result.state === 'attention').length
  const failed = batchResults.value.filter((result) => result.state === 'failed').length
  const skipped = batchResults.value.filter((result) => result.state === 'skipped').length
  setNotice(`批量同步已完成：${done} 项完成，${attention} 项待处理，${skipped} 项跳过，${failed} 项失败`, failed ? 'error' : 'success')
}

function openAction(action: 'checkout' | 'merge' | 'rebase') {
  if (action === 'checkout') {
    const repo = selectedRepository.value
    if (!repo) return
    checkoutChoice.value = repo.currentBranch && repo.localBranches.includes(repo.currentBranch)
      ? `local:${repo.currentBranch}`
      : repo.localBranches[0] ? `local:${repo.localBranches[0]}`
        : repo.remoteBranches[0] ? `remote:${repo.remoteBranches[0]}` : ''
  }
  dialogAction.value = action
  dialogOpen.value = true
}

async function confirmDialogAction() {
  const repo = selectedRepository.value
  const action = dialogAction.value
  if (!repo || !action) return
  if (action === 'checkout' && checkoutBlockedReason.value) return
  const targetBranch = checkoutRemote.value ? newLocalBranch.value.trim() : checkoutLocal.value
  const remoteBranch = checkoutRemote.value
  dialogOpen.value = false
  if (action === 'checkout') {
    await runSingle('切换分支', () => invoke('checkout_branch', { path: repo.path, branch: targetBranch, remoteBranch }))
    return
  }
  await runSingle(action === 'merge' ? '合并远端提交' : '变基到远端分支', () => invoke('integrate_upstream', { path: repo.path, strategy: action }))
}

function changeFileSelection(path: string, checked: boolean | 'indeterminate') {
  selectedFiles.value = checked
    ? [...new Set([...selectedFiles.value, path])]
    : selectedFiles.value.filter((item) => item !== path)
}

async function refreshChanges() {
  const repo = selectedRepository.value
  if (!repo) return
  changes.value = await invoke<ChangedFile[]>('get_worktree_changes', { path: repo.path })
}

async function stageSelected() {
  const repo = selectedRepository.value
  if (!repo || !selectedFiles.value.length) return
  await runSingle('暂存所选文件', async () => {
    await invoke('stage_files', { path: repo.path, files: selectedFiles.value })
    await refreshChanges()
  })
}

async function commitSelected() {
  const repo = selectedRepository.value
  if (!repo || !selectedFiles.value.length || !commitMessage.value.trim()) return
  await runSingle('提交所选文件', async () => {
    await invoke('commit_files', { path: repo.path, files: selectedFiles.value, message: commitMessage.value })
    commitMessage.value = ''
    selectedFiles.value = []
    dialogOpen.value = false
    await refreshChanges()
  })
  const latest = getRepo(repo.id)
  if (latest?.behind) setNotice('提交已创建。远端也有新提交，请重新检查关系并选择合并或变基', 'info')
}

async function openCommitDialog() {
  const repo = selectedRepository.value
  if (!repo || busy.value) return
  if (repo.hasConflicts || repo.operationInProgress) {
    setNotice('请先在 IDE 或终端完成正在进行的 Git 操作，完成后刷新再提交', 'error')
    return
  }
  busy.value = true
  busyLabel.value = '读取工作区改动'
  try {
    await refreshChanges()
    if (commitRepositoryId !== repo.id) {
      selectedFiles.value = []
      commitMessage.value = ''
      commitRepositoryId = repo.id
    }
    selectedFiles.value = selectedFiles.value.filter((path) => changes.value.some((change) => change.path === path))
    dialogAction.value = null
    dialogOpen.value = true
  } catch (reason) { setNotice(String(reason), 'error') }
  finally { busy.value = false; busyLabel.value = '' }
}

function formatDate(value: string) { return formatAppDate(value, settings.options.dateFormat) }

async function copyPath(path: string) {
  try {
    await navigator.clipboard.writeText(displayPath(path))
    setNotice('仓库路径已复制', 'success')
  } catch {
    setNotice(displayPath(path))
  }
}

function viewRepository(id: string) {
  if (busy.value) return
  void router.push({ name: 'repository', params: { id } })
}

function checkoutFromPanel(branch: string, remote: boolean) {
  openAction('checkout')
  checkoutChoice.value = `${remote ? 'remote' : 'local'}:${branch}`
}
function branchActivity(active: boolean) {
  busy.value = active
  busyLabel.value = active ? '分支操作' : ''
}

async function repositoryAction(repo: RepositorySummary, action: 'checkout' | 'fetch' | 'commit' | 'pull' | 'push' | 'copy' | 'groups') {
  if (busy.value || workspace.isLoading) return
  if (action === 'groups') return groups.assign(repo.id)
  if (action === 'copy') return copyPath(repo.path)
  selectedId.value = repo.id
  if (action === 'checkout') return openAction('checkout')
  if (action === 'commit') {
    await openCommitDialog()
    return
  }
  if (action === 'fetch') await fetchOne()
  else if (action === 'pull') await pullOne()
  else await pushOne()
}
</script>

<template>
  <div :class="cn('flex min-w-0 flex-col gap-5', isDetailPage && 'repository-detail h-full min-h-0')">
    <Card :class="cn('shrink-0 gap-0', isDetailPage ? 'p-4' : 'border-0 bg-transparent p-0 shadow-none')"><header class="flex flex-wrap items-center justify-between gap-3">
      <div class="flex min-w-0 items-center gap-3"><Button v-if="isDetailPage || activeGroupId" variant="outline" size="icon-sm" aria-label="返回仓库总览" :disabled="busy" @click="router.push('/')" title="返回仓库总览"><ArrowLeftIcon /></Button><div class="min-w-0"><h1 class="truncate text-xl font-semibold tracking-tight">{{ isDetailPage ? (selectedRepository?.name ?? '仓库详情') : collectionTitle }}</h1><p v-if="!isDetailPage" class="mt-1 text-sm text-muted-foreground">集中管理你的项目，点击仓库进入详情</p></div></div>
      <div class="flex flex-wrap items-center gap-2">
        <RepositoryActions v-if="isDetailPage && selectedRepository" :key="selectedRepository.id" :repo="selectedRepository" :disabled="busy" @updated="loadDetails(selectedRepository)"><AppTooltip text="刷新"><Button variant="outline" size="icon-sm" aria-label="刷新" :disabled="busy || workspace.isLoading" @click="refreshAll" title="刷新"><RefreshCwIcon :class="cn(workspace.isLoading && 'animate-spin')" /></Button></AppTooltip></RepositoryActions>
        <template v-if="!isDetailPage || !selectedRepository"><AppTooltip text="刷新"><Button variant="outline" size="icon-sm" aria-label="刷新" :disabled="busy || workspace.isLoading" @click="refreshAll" title="刷新"><RefreshCwIcon :class="cn(workspace.isLoading && 'animate-spin')" /></Button></AppTooltip></template>
        <AppTooltip text="快速克隆仓库"><Button variant="outline" size="icon-sm" aria-label="快速克隆仓库" @click="clone.openDialog(true)" title="快速克隆仓库"><PlusIcon /></Button></AppTooltip>
        <AddRepositoryMenu v-if="!isDetailPage" :disabled="busy || workspace.isLoading" @local="addDirectory" />
      </div>
    </header></Card>

    <Alert v-if="workspace.error" variant="destructive"><CircleAlertIcon /><AlertTitle>扫描失败</AlertTitle><AlertDescription class="break-words">{{ workspace.error }}</AlertDescription></Alert>
    <Alert v-if="workspace.issues.length" variant="warning"><TriangleAlertIcon /><AlertTitle>部分目录无法扫描</AlertTitle><AlertDescription class="flex flex-col gap-1"><p v-for="issue in workspace.issues.slice(0, 4)" :key="`${issue.path}-${issue.message}`" class="break-all">{{ displayPath(issue.path) }} · {{ issue.message }} <Button size="sm" variant="outline" :disabled="busy || clone.running" @click="relocate(issue.path)">重新定位仓库</Button></p></AlertDescription></Alert>

    <div :class="cn('flex min-w-0 flex-col gap-4', isDetailPage && 'min-h-0 flex-1')">
      <section v-if="!isDetailPage && (workspace.repositories.length || workspace.roots.length || activeGroupId)" class="flex min-w-0 flex-col gap-4">
        <Card class="gap-0 py-4">
          <CardContent class="flex flex-wrap items-center gap-3 px-4">
            <div class="mr-auto"><p class="text-sm font-semibold">{{ collectionTitle }} · {{ collection.length }}</p><p class="mt-1 text-xs text-muted-foreground">点击卡片查看详情，勾选仓库进行批量同步</p></div>
            <div class="flex flex-wrap gap-3"><Field v-for="filter in filterOptions" :key="filter.id" orientation="horizontal" class="w-auto"><Checkbox :id="`repo-filter-${filter.id}`" :model-value="statusFilters.includes(filter.id)" @update:model-value="toggleFilter(filter.id)" /><FieldLabel :for="`repo-filter-${filter.id}`">{{ filter.label }}</FieldLabel></Field></div><InputGroup class="w-full sm:w-64"><InputGroupInput v-model="search" placeholder="搜索仓库、路径或分支" aria-label="搜索仓库" /><InputGroupAddon><SearchIcon /></InputGroupAddon></InputGroup>
            <Tabs :model-value="layout" @update:model-value="setLayout"><TabsList aria-label="仓库布局"><TabsTrigger value="grid" aria-label="网格布局" title="网格布局"><LayoutGridIcon /></TabsTrigger><TabsTrigger value="list" aria-label="列表布局" title="列表布局"><ListIcon /></TabsTrigger></TabsList></Tabs>
            <Button variant="ghost" size="sm" :disabled="!filteredRepositories.length || busy" @click="toggleAllRepositories">{{ allVisibleSelected ? '取消全选' : '全选' }}</Button>
            <Button v-if="!activeGroupId" size="sm" :disabled="busy || workspace.isLoading || workspace.operationBusy || !selectedCount" @click="syncSelected"><LoaderCircleIcon v-if="busy && busyLabel === '批量检查与同步'" data-icon="inline-start" class="animate-spin" /><GitPullRequestIcon v-else data-icon="inline-start" />同步所选 · {{ selectedCount }}</Button>
          <Button size="sm" variant="outline" :disabled="!selectedCount || busy" @click="groups.assignMany(checkedRepositoryIds)">所选加入分组 · {{ selectedCount }}</Button><template v-if="activeGroup"><Button size="sm" variant="outline" :disabled="groups.saving" @click="groups.edit(activeGroup.id)">管理仓库</Button><GroupBatchFetch :key="activeGroup.id" :members="collection" :selected="checkedRepositoryIds" :disabled="busy || workspace.isLoading" /></template></CardContent>
        </Card>
        <div v-if="filteredRepositories.length" :class="cn('grid min-w-0 gap-4', layout === 'grid' && 'repository-gallery')">
          <RepositoryCard v-for="repo in filteredRepositories" :key="repo.id" :repo="repo" :layout="layout" :pinned="business.pins.includes(repo.id)" @pin="togglePin(repo.id); toast.success(business.pins.includes(repo.id) ? '仓库已置顶' : '已取消置顶')" :checked="checkedRepositoryIds.includes(repo.id)" :disabled="busy || workspace.isLoading || workspace.operationBusy" @select="toggleRepository(repo.id)" @action="repositoryAction(repo, $event)" />
        </div>
        <Card v-else class="py-6"><Empty><EmptyHeader><EmptyMedia variant="icon"><FolderGit2Icon /></EmptyMedia><EmptyTitle>{{ search ? '没有匹配的仓库' : activeGroupId ? '分组中暂无可用仓库' : '未发现仓库' }}</EmptyTitle><EmptyDescription>{{ search ? '试试其他名称、路径或分支' : activeGroupId ? '点击管理仓库添加成员；已移除或暂时无法访问的仓库不会显示在列表中' : '所选目录中暂未发现 Git 仓库' }}</EmptyDescription></EmptyHeader></Empty></Card>
        <Card v-if="batchResults.length && !activeGroupId" class="gap-3 py-4"><CardHeader class="px-4"><CardTitle>同步结果</CardTitle><CardDescription>点击结果进入对应仓库，处理需要确认的操作</CardDescription></CardHeader><CardContent class="grid gap-2 px-3 sm:grid-cols-2"><Button v-for="result in batchResults" :key="result.id" variant="ghost" class="h-auto justify-start whitespace-normal py-2 text-left" :disabled="busy" @click="viewRepository(result.id)"><CircleCheckIcon v-if="result.state === 'success'" data-icon="inline-start" /><CircleAlertIcon v-else data-icon="inline-start" /><span class="min-w-0"><span class="block truncate">{{ result.name }}</span><span class="mt-1 block text-xs text-muted-foreground">{{ result.message }}</span></span></Button></CardContent></Card>
      </section>
    <Card v-if="isDetailPage && !selectedRepository" class="min-h-0 flex-1 justify-center py-8"><Empty><EmptyHeader><EmptyMedia variant="icon"><FolderGit2Icon /></EmptyMedia><EmptyTitle>{{ workspace.isLoading ? '正在读取仓库' : '未找到该仓库' }}</EmptyTitle><EmptyDescription>{{ workspace.isLoading ? '扫描完成后会显示仓库详情' : '仓库可能已移除或路径已变化，请返回总览重新选择' }}</EmptyDescription></EmptyHeader><EmptyContent><Button variant="outline" @click="router.push('/')">返回仓库总览</Button></EmptyContent></Empty></Card>
      <section v-if="isDetailPage && selectedRepository" class="mx-auto flex w-full min-h-0 min-w-0 flex-1 flex-col gap-4">
<Tabs :key="selectedRepository.id" v-model="detailTab" class="repository-detail-tabs flex min-h-0 flex-1 flex-col gap-4 rounded-xl border bg-card p-4 shadow-sm"><div class="flex shrink-0 flex-wrap items-center gap-2"><TabsList class="h-auto max-w-full shrink-0 flex-wrap justify-start p-1"><TabsTrigger value="overview" class="px-3">总览</TabsTrigger><TabsTrigger value="branches" class="px-3">分支管理</TabsTrigger><TabsTrigger value="commits" class="px-3">提交历史</TabsTrigger><TabsTrigger value="worktree" class="px-3">工作区</TabsTrigger></TabsList><Select :model-value="['compare','stash','tags','activity'].includes(detailTab) ? detailTab : ''" @update:model-value="value => { detailTab = String(value) }"><SelectTrigger class="w-32" aria-label="更多仓库工具"><SelectValue placeholder="更多工具" /></SelectTrigger><SelectContent><SelectGroup><SelectItem value="compare">分支对比</SelectItem><SelectItem value="stash">暂存区</SelectItem><SelectItem value="tags">标签管理</SelectItem><SelectItem value="activity">操作记录</SelectItem></SelectGroup></SelectContent></Select></div><ScrollArea class="min-h-0 flex-1"><TabsContent value="overview"><div class="mb-3 flex flex-wrap items-center gap-x-6 gap-y-2 px-4 text-xs text-muted-foreground" title="基于本地所有分支和标签的提交时间；浅克隆仅包含已获取的历史"><span class="inline-flex items-center gap-2"><CalendarDaysIcon class="size-4" />创建日期：{{ repositoryDates ? formatRepositoryDate(repositoryDates[0]) : summaryLoading ? '加载中…' : repositoryDatesError ? '读取失败' : '暂无提交' }}</span><span>最新更新日期：{{ repositoryDates ? formatRepositoryDate(repositoryDates[1]) : summaryLoading ? '加载中…' : repositoryDatesError ? '读取失败' : '暂无提交' }}</span></div><ItemGroup class="overflow-hidden rounded-xl border bg-card"><template v-for="item in overviewItems" :key="item.label"><Item size="sm" class="flex-nowrap gap-3 px-4 py-2.5 transition-colors hover:bg-muted/40"><ItemMedia class="size-8 [&_svg]:size-4"><FolderOpenIcon v-if="item.local" /><GitBranchIcon v-else-if="item.label.includes('分支')" /><GlobeIcon v-else /></ItemMedia><ItemContent class="min-w-0 gap-1"><ItemTitle>{{ item.label }}</ItemTitle><ItemDescription class="break-all leading-relaxed"><template v-if="'branchInfo' in item">{{ item.branchInfo.current }} / <AppTooltip v-if="item.branchInfo.remoteUrl" :text="item.branchInfo.remoteUrl"><span tabindex="0">{{ item.branchInfo.upstream }}</span></AppTooltip><span v-else>{{ item.branchInfo.upstream }}</span></template><template v-else>{{ item.value }}</template></ItemDescription></ItemContent><ItemActions><AppTooltip :text="item.target ? item.tooltip : '未配置对应地址或暂不支持此远程平台的分支链接'"><span tabindex="0"><Button variant="outline" size="icon-sm" :aria-label="item.tooltip" :disabled="!item.target" @click="openOverviewItem(item)" :title="item.tooltip"><FolderOpenIcon v-if="item.local" /><ExternalLinkIcon v-else /></Button></span></AppTooltip></ItemActions></Item></template></ItemGroup>
<div class="mt-4 grid min-w-0 grid-cols-3 gap-3">
 <Card class="min-w-0 gap-3 py-4">
  <CardHeader class="flex flex-row items-center justify-between gap-2 px-4"><CardTitle class="flex shrink-0 items-center gap-2 text-sm"><FolderOpenIcon class="size-4" />工作区状态</CardTitle><Badge :variant="selectedRepository.hasConflicts ? 'destructive' : selectedRepository.operationInProgress || selectedRepository.isDirty ? 'warning' : 'success'" class="min-w-0 max-w-[55%]" :title="selectedRepository.hasConflicts ? '存在冲突' : selectedRepository.operationInProgress ? 'Git 操作待完成' : selectedRepository.isDirty ? '有未提交改动' : '工作区干净'"><span class="truncate">{{ selectedRepository.hasConflicts ? '存在冲突' : selectedRepository.operationInProgress ? '操作待完成' : selectedRepository.isDirty ? '有改动' : '工作区干净' }}</span></Badge></CardHeader>
  <CardContent class="min-h-10 flex-1 px-4"><p class="line-clamp-2 text-sm leading-5 text-muted-foreground">{{ summaryLoading ? '正在读取变更…' : treeError ? '变更明细读取失败，请刷新重试' : `${changes.length} 个变更文件` }}</p></CardContent>
  <CardFooter class="min-w-0 px-4"><p class="truncate text-xs text-muted-foreground">{{ summaryLoading ? '正在统计文件状态' : treeError ? '请刷新仓库重试' : `已暂存 ${worktreeCounts.staged} · 未暂存 ${worktreeCounts.unstaged} · 未跟踪 ${worktreeCounts.untracked}` }}</p></CardFooter>
 </Card>
 <Card class="min-w-0 gap-3 py-4">
  <CardHeader class="flex flex-row items-center justify-between gap-2 px-4"><CardTitle class="flex shrink-0 items-center gap-2 text-sm"><RefreshCwIcon class="size-4" />同步状态</CardTitle><Badge :variant="selectedRepository.ahead && selectedRepository.behind ? 'destructive' : selectedRepository.isDetached || !selectedRepository.upstream ? 'warning' : selectedRepository.ahead || selectedRepository.behind ? 'info' : 'success'" class="min-w-0 max-w-[55%]" :title="syncLabel"><span class="truncate">{{ syncLabel }}</span></Badge></CardHeader>
  <CardContent class="min-h-10 flex-1 px-4"><p class="line-clamp-2 text-sm leading-5 text-muted-foreground">↑ {{ selectedRepository.ahead }} 待推送 · ↓ {{ selectedRepository.behind }} 待拉取</p></CardContent>
  <CardFooter class="min-w-0 px-4"><p class="truncate text-xs text-muted-foreground" :title="selectedRepository.upstream ? '基于最近一次 Fetch 的远端引用' : '配置跟踪分支后可比较同步差异'">{{ selectedRepository.upstream ? '基于最近一次 Fetch 的远端引用' : '配置跟踪分支后可比较同步差异' }}</p></CardFooter>
 </Card>
 <Card class="min-w-0 gap-3 py-4">
  <CardHeader class="px-4"><CardTitle class="flex items-center gap-2 text-sm"><GitCommitHorizontalIcon class="size-4" />最近一次提交</CardTitle></CardHeader>
  <CardContent class="min-h-10 flex-1 px-4"><p class="line-clamp-2 break-words text-sm leading-5 text-muted-foreground" :title="latestCommit?.subject">{{ summaryLoading ? '正在读取…' : latestCommitError ? '读取失败' : latestCommit?.subject || '暂无提交' }}</p></CardContent>
  <CardFooter class="min-w-0 px-4"><div v-if="latestCommit" class="flex min-w-0 flex-1 items-center justify-between gap-2 text-xs text-muted-foreground"><span class="min-w-0 truncate" :title="`${latestCommit.shortHash} · ${latestCommit.author}`">{{ latestCommit.shortHash }} · {{ latestCommit.author }}</span><span class="shrink-0 whitespace-nowrap">{{ formatDate(latestCommit.committedAt) }}</span></div><p v-else class="truncate text-xs text-muted-foreground">{{ latestCommitError ? '请刷新仓库后重试' : '当前 HEAD 尚无提交记录' }}</p></CardFooter>
 </Card>
</div></TabsContent><TabsContent value="branches">        <BranchPanel :key="selectedRepository.id" :repo="selectedRepository" :disabled="busy || workspace.isLoading || workspace.operationBusy" @checkout="checkoutFromPanel" @activity="branchActivity" @updated="loadDetails(selectedRepository)" /><RepositoryTools :repo="selectedRepository" mode="branches" :disabled="busy" @updated="loadDetails(selectedRepository)" />

</TabsContent>              <TabsContent value="commits" class="commit-history-panel h-full min-h-0 min-w-0 flex-col overflow-hidden data-[state=active]:flex">
                <FieldGroup class="mb-3 grid shrink-0 grid-cols-1 items-end gap-3 pt-1 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.3fr)_minmax(0,2fr)]"><Field class="min-w-0 gap-2"><Select v-model="branchChoice" :disabled="!branchNames.length"><SelectTrigger id="history-branch" class="w-full min-w-0 max-w-full" aria-label="选择分支查看提交"><SelectValue placeholder="选择分支" /></SelectTrigger><SelectContent><SelectGroup><SelectItem v-for="branch in branchNames" :key="branch" :value="branch">{{ branch }} · {{ selectedRepository.localBranches.includes(branch) ? '本地' : '远端' }}</SelectItem></SelectGroup></SelectContent></Select></Field>
                  <Field class="min-w-0 gap-2"><InputGroup><InputGroupInput id="history-search" aria-label="搜索提交名称" v-model="commitSearch" placeholder="搜索提交名称" /><InputGroupAddon><SearchIcon /></InputGroupAddon></InputGroup></Field>
                  <Field class="min-w-0 gap-2"><div class="flex min-w-0 items-center gap-2"><DatePicker v-model="commitDateFrom" :max="commitDateTo" placeholder="开始日期" class="min-w-0 flex-1" /><span class="text-xs text-muted-foreground">至</span><DatePicker v-model="commitDateTo" :min="commitDateFrom" placeholder="结束日期" class="min-w-0 flex-1" /><Button v-if="hasCommitFilters" variant="ghost" size="sm" @click="commitSearch = ''; commitDateFrom = ''; commitDateTo = ''">重置</Button></div></Field>
                </FieldGroup>
                <ScrollArea :key="`${selectedRepository.id}-${commitPage}-${branchChoice}`" class="min-h-0 flex-1"><div class="pr-2 pb-1"><ItemGroup class="gap-2"><Item v-for="commit in filteredCommits" :key="commit.hash" role="listitem" variant="outline" size="sm" class="flex-nowrap"><ItemMedia variant="icon"><GitCommitHorizontalIcon /></ItemMedia><ItemContent class="min-w-0"><ItemTitle class="max-w-full"><button class="truncate text-left hover:underline" :title="commit.subject" @click="viewDiff('commit', commit.hash)">{{ commit.subject || '(无提交说明)' }}</button></ItemTitle><ItemDescription>{{ commit.shortHash }} · {{ commit.author }} · {{ formatDate(commit.committedAt) }}</ItemDescription></ItemContent><ItemActions><AppTooltip :text="commitUrl(commit.hash) ? '打开远程提交页面' : '未配置远程仓库或暂不支持此平台'"><span tabindex="0"><Button variant="outline" size="icon-sm" aria-label="打开远程提交页面" :disabled="!commitUrl(commit.hash)" @click="openOverviewItem({target: commitUrl(commit.hash), local: false})" title="打开远程提交页面"><ExternalLinkIcon /></Button></span></AppTooltip></ItemActions></Item></ItemGroup><Empty v-if="!filteredCommits.length && !commitsLoading" class="p-5"><EmptyHeader><EmptyTitle>{{ commitsError || (hasCommitFilters ? '没有匹配的提交' : '暂无提交记录') }}</EmptyTitle><EmptyDescription>{{ hasCommitFilters ? '调整搜索名称或日期范围' : '请选择有提交的分支' }}</EmptyDescription></EmptyHeader></Empty>
                </div></ScrollArea><div class="flex shrink-0 flex-wrap items-center justify-between gap-3 border-t pt-3"><span class="text-xs text-muted-foreground" role="status">{{ commitsLoading ? '查询中…' : `第 ${commitPage} 页 · 每页 ${commitPageSize} 条` }}</span><div class="flex items-center gap-2"><Button variant="outline" size="xs" :disabled="commitPage === 1 || commitsLoading" @click="commitPage = 1">首页</Button><Button variant="outline" size="xs" :disabled="commitPage === 1 || commitsLoading" @click="commitPage--">上一页</Button><Button variant="outline" size="xs" :disabled="!hasNextCommitPage || commitsLoading" @click="commitPage++">下一页</Button></div></div>
              </TabsContent><TabsContent value="worktree"><Tabs default-value="changes"><TabsList><TabsTrigger value="changes">未提交改动</TabsTrigger><TabsTrigger value="files">项目文件</TabsTrigger></TabsList>              <TabsContent value="changes" class="min-w-0"><div class="mb-3 flex flex-wrap items-center justify-between gap-2"><p class="text-xs text-muted-foreground">{{ changes.length }} 个文件发生改动</p><Button v-if="selectedRepository.isDirty && !selectedRepository.hasConflicts && !selectedRepository.operationInProgress" size="sm" variant="outline" :disabled="busy" @click="openCommitDialog">选择文件并提交</Button></div><Alert v-if="selectedRepository.hasConflicts || selectedRepository.operationInProgress" variant="warning" class="mb-3"><TriangleAlertIcon /><AlertTitle>请先完成 Git 操作</AlertTitle><AlertDescription>在 IDE 或终端解决冲突或完成当前操作后，再刷新工作区</AlertDescription></Alert><div class="detail-content flex flex-col gap-1 "><div v-for="change in changes" :key="change.path" class="flex min-w-0 items-center gap-2 rounded-md bg-muted/50 p-2"><ChangeStatusBadge :change="change" /><button class="min-w-0 flex-1 truncate text-left text-xs hover:underline" :title="change.path" @click="viewDiff('worktree', change.path)">{{ change.path }}</button></div><Empty v-if="!changes.length" class="p-5"><EmptyHeader><EmptyMedia variant="icon"><CircleCheckIcon /></EmptyMedia><EmptyTitle>工作区干净</EmptyTitle><EmptyDescription>没有需要提交的文件</EmptyDescription></EmptyHeader></Empty></div></TabsContent>              <TabsContent value="files" class="min-w-0"><p class="mb-3 text-xs text-muted-foreground">按需展开文件夹，隐藏依赖与构建目录</p><Alert v-if="treeError" variant="destructive"><CircleAlertIcon /><AlertTitle>无法读取目录</AlertTitle><AlertDescription class="break-all">{{ treeError }}</AlertDescription></Alert><ScrollArea v-else class="detail-scroll "><RepoTree v-for="entry in treeEntries" :key="entry.path" :repo-path="selectedRepository.path" :entry="entry" /><Empty v-if="!treeEntries.length" class="p-5"><EmptyHeader><EmptyTitle>没有可显示的文件</EmptyTitle><EmptyDescription>仓库为空，或暂时无法读取</EmptyDescription></EmptyHeader></Empty></ScrollArea></TabsContent></Tabs></TabsContent><TabsContent v-for="mode in (['compare', 'stash', 'tags', 'activity'] as const)" :key="mode" :value="mode" class="repository-fill-panel h-full min-h-0 flex-col overflow-hidden data-[state=active]:flex"><RepositoryTools :repo="selectedRepository" :mode="mode" :disabled="busy" @updated="loadDetails(selectedRepository)" /></TabsContent></ScrollArea></Tabs><RepositoryDiff v-model:open="diffOpen" :path="selectedRepository.path" :kind="diffKind" :first="diffReference" :title="diffKind === 'commit' ? '提交详情' : '文件差异'" /></section>
    </div>


    <Card v-if="!isDetailPage && !activeGroupId && !workspace.repositories.length && !workspace.roots.length" class="py-8"><Empty><EmptyHeader><EmptyMedia variant="icon"><FolderGit2Icon /></EmptyMedia><EmptyTitle>把你的项目放在同一个工作台</EmptyTitle><EmptyDescription>添加单个 Git 仓库；也可在设置中配置固定扫描目录，自动发现仓库</EmptyDescription></EmptyHeader><EmptyContent><AddRepositoryMenu :disabled="workspace.isLoading" label="添加第一个仓库" @local="addDirectory" /><p class="text-xs text-muted-foreground">扫描设置与偏好保存在本机</p></EmptyContent></Empty></Card>

    <Dialog v-model:open="dialogOpen">
      <DialogContent class="max-h-[85dvh] flex flex-col">
        <DialogHeader><DialogTitle>{{ dialogAction === 'checkout' ? '切换分支' : dialogAction === 'merge' ? '合并远端提交' : dialogAction === 'rebase' ? '变基到远端' : '审阅并提交改动' }}</DialogTitle><DialogDescription v-if="dialogAction === 'checkout'">{{ selectedRepository?.name }} · 当前分支 {{ selectedRepository?.currentBranch ?? 'Detached HEAD' }}。选择目标分支后确认切换</DialogDescription><DialogDescription v-else-if="dialogAction">操作前已获取最新远端引用。{{ dialogAction === 'merge' ? 'Git 会创建合并提交（若可快进则快进）' : '本地提交会重新应用到远端分支之后' }}若出现冲突，请在 IDE 或终端中解决</DialogDescription><DialogDescription v-else>勾选本次提交包含的文件，并填写提交说明</DialogDescription></DialogHeader><ScrollArea class="min-h-0 flex-1 max-h-[60dvh]">
        <FieldGroup v-if="dialogAction === 'checkout'">
          <Field>
            <FieldLabel for="checkout-branch">目标分支</FieldLabel>
            <Select v-model="checkoutChoice" :disabled="busy || !branchNames.length">
              <SelectTrigger id="checkout-branch" class="w-full min-w-0" aria-label="选择要切换的分支"><SelectValue placeholder="请先 Fetch 获取分支" /></SelectTrigger>
              <SelectContent>
                <SelectGroup v-if="selectedRepository?.localBranches.length"><SelectLabel>本地分支</SelectLabel><SelectItem v-for="branch in selectedRepository.localBranches" :key="`local:${branch}`" :value="`local:${branch}`">{{ branch }}{{ branch === selectedRepository.currentBranch ? ' · 当前' : '' }}</SelectItem></SelectGroup>
                <SelectGroup v-if="selectedRepository?.remoteBranches.length"><SelectLabel>远端分支 · 创建本地跟踪分支</SelectLabel><SelectItem v-for="branch in selectedRepository.remoteBranches" :key="`remote:${branch}`" :value="`remote:${branch}`">{{ branch }}</SelectItem></SelectGroup>
              </SelectContent>
            </Select>
          </Field>
          <Field v-if="checkoutRemote"><FieldLabel for="new-local-branch">新建本地分支名</FieldLabel><Input id="new-local-branch" v-model="newLocalBranch" placeholder="例如 feature/design" :disabled="busy" /><p class="break-all text-xs leading-5 text-muted-foreground">确认后将从 {{ checkoutRemote }} 创建 {{ newLocalBranch.trim() || '指定名称的本地分支' }}，设置远端跟踪关系并切换。远端列表来自最近一次 Fetch</p></Field>
          <Alert v-if="checkoutBlockedReason" variant="warning"><TriangleAlertIcon /><AlertTitle>暂时无法切换</AlertTitle><AlertDescription><p>{{ checkoutBlockedReason }}</p><Button v-if="selectedRepository?.isDirty && !selectedRepository.hasConflicts && !selectedRepository.operationInProgress" size="sm" variant="outline" class="mt-3" @click="openCommitDialog"><GitCommitHorizontalIcon data-icon="inline-start" />审阅并提交改动</Button></AlertDescription></Alert>
          <p v-else class="break-all text-sm">将切换到 <span class="font-medium">{{ checkoutRemote ? newLocalBranch.trim() : checkoutLocal }}</span></p>
        </FieldGroup>
        <FieldGroup v-if="!dialogAction">
          <FieldSet><FieldLegend variant="label">选择变更文件</FieldLegend><ScrollArea class="h-56 rounded-lg border"><FieldGroup class="gap-1 p-2"><Field v-for="(change, index) in changes" :key="change.path" orientation="horizontal" class="gap-2 rounded-md p-2 hover:bg-muted"><Checkbox :id="`commit-file-${index}`" :model-value="selectedFiles.includes(change.path)" @update:model-value="changeFileSelection(change.path, $event)" /><ChangeStatusBadge :change="change" /><FieldLabel :for="`commit-file-${index}`" class="min-w-0 flex-1"><span class="truncate" :title="change.path">{{ change.path }}</span></FieldLabel></Field><p v-if="!changes.length" class="p-3 text-xs text-muted-foreground">当前没有可提交的变更</p></FieldGroup></ScrollArea></FieldSet>
          <Button variant="outline" size="sm" :disabled="!selectedFiles.length || busy" @click="stageSelected"><CheckIcon data-icon="inline-start" />暂存所选文件</Button>
          <Field><FieldLabel for="commit-message">提交说明</FieldLabel><Input id="commit-message" v-model="commitMessage" placeholder="描述本次提交的改动" maxlength="200" /></Field>
          <p class="text-xs leading-5 text-muted-foreground">「未跟踪」表示新文件尚未纳入 Git，选中提交后会纳入版本管理。提交只包含选中的文件，完成后会重新检查远端分支关系</p>
        </FieldGroup>
        </ScrollArea><DialogFooter><DialogClose as-child><Button variant="outline">取消</Button></DialogClose><Button v-if="dialogAction" :disabled="busy || (dialogAction === 'checkout' && !!checkoutBlockedReason)" @click="confirmDialogAction">{{ dialogAction === 'checkout' ? (checkoutRemote ? '创建并切换' : '确认切换') : dialogAction === 'merge' ? '确认合并' : '确认变基' }}</Button><Button v-else :disabled="busy || !selectedFiles.length || !commitMessage.trim()" @click="commitSelected"><GitCommitHorizontalIcon data-icon="inline-start" />提交 {{ selectedFiles.length }} 个文件</Button></DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>
