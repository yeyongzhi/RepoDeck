<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { RefreshCwIcon, ExternalLinkIcon, GitBranchPlusIcon } from '@lucide/vue'
import AppTooltip from '@/components/AppTooltip.vue'
import { toast } from 'vue-sonner'
import { invoke, gitActivities } from '@/lib/git-client'
import { remoteWebUrl } from '@/lib/remote-url'
import type { RepositorySummary } from '@/types/git'
import { useSettingsStore } from '@/stores/settings'
import { formatAppDate } from '@/lib/app-options'
import { useWorkspaceStore } from '@/stores/workspace'
import RepositoryDiff from '@/components/RepositoryDiff.vue'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Input } from '@/components/ui/input'
import { Item, ItemGroup, ItemContent, ItemTitle, ItemDescription, ItemActions } from '@/components/ui/item'
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '@/components/ui/card'
import { Select, SelectTrigger, SelectValue, SelectContent, SelectGroup, SelectItem } from '@/components/ui/select'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '@/components/ui/dialog'
import { Field, FieldGroup, FieldLabel } from '@/components/ui/field'
const props = defineProps<{ repo: RepositorySummary; mode: 'branches' | 'compare' | 'stash' | 'tags' | 'activity'; disabled: boolean }>()
const emit = defineEmits<{ updated: [] }>()
const workspace = useWorkspaceStore()
const settings = useSettingsStore()
const tags = ref<string[]>([])
const stashes = ref<{hash: string; reference: string; subject: string}[]>([])
const error = ref('')
const loading = ref(false)
const working = ref(false)
const action = ref('')
const first = ref('')
const second = ref('')
const message = ref('')
const selectedBranch = ref(props.repo.currentBranch ?? props.repo.localBranches[0] ?? '')
const remoteBranch = ref(props.repo.remoteBranches[0] ?? '')
const remote = ref(props.repo.remotes[0]?.name ?? '')
const compareLeft = ref(props.repo.currentBranch ? `refs/heads/${props.repo.currentBranch}` : '')
const compareRight = ref(props.repo.upstream ? `refs/remotes/${props.repo.upstream}` : '')
const diffOpen = ref(false)
const diffKind = ref('compare')
const diffFirst = ref('')
const diffSecond = ref('')
const dialogOpen = ref(false)
const labels: Record<string, string> = { rename: '重命名分支', upstream: '设置关联分支', 'unset-upstream': '取消关联分支', publish: '发布本地分支', 'tag-branch': '从标签创建分支', 'stash-save': '保存暂存改动', 'stash-apply': '恢复暂存改动', 'stash-drop': '删除暂存改动' }
const refs = computed(() => [...props.repo.localBranches.map(name => ({value: `refs/heads/${name}`, label: `${name} · 本地`})), ...props.repo.remoteBranches.map(name => ({value: `refs/remotes/${name}`, label: `${name} · 远端`}))])
const history = computed(() => gitActivities.value.filter(item => item.path === props.repo.path))
const disabled = computed(() => props.disabled || working.value || workspace.operationBusy || workspace.isLoading)
const invalid = computed(() => ['rename', 'tag-branch', 'upstream', 'publish'].includes(action.value) && !second.value.trim())
let request = 0
async function load() {
 const id = ++request; error.value = ''; loading.value = true
 try {
  if (props.mode === 'tags') { const result = await invoke<string[]>('repository_read', {path: props.repo.path, kind: 'tags'}); if (id === request) tags.value = result }
  if (props.mode === 'stash') { const result = await invoke<typeof stashes.value>('repository_read', {path: props.repo.path, kind: 'stashes'}); if (id === request) stashes.value = result }
 } catch (reason) { if (id === request) error.value = String(reason) }
 finally { if (id === request) loading.value = false }
}
watch(() => [props.repo.path, props.mode], () => { tags.value = []; stashes.value = []; void load() }, { immediate: true })
function prepare(kind: string, value: string, other = '') { action.value = kind; first.value = value; second.value = other; error.value = ''; dialogOpen.value = true }
async function execute() {
 if (disabled.value || invalid.value) return
 working.value = true; workspace.operationBusy = true; error.value = ''
 try {
  await invoke('repository_action', {path: props.repo.path, kind: action.value, first: first.value, second: second.value.trim()})
  dialogOpen.value = false; toast.success(`${labels[action.value]}完成`)
  await workspace.refresh(); await load(); emit('updated')
 } catch (reason) { const failure = String(reason); toast.error(failure); await workspace.refresh(); await load().catch(() => undefined); error.value = failure }
 finally { working.value = false; workspace.operationBusy = false }
}
function showDiff(kind: string, value: string, other = '') { diffKind.value = kind; diffFirst.value = value; diffSecond.value = other; diffOpen.value = true }
async function openTag(tag: string) {
 const remote = props.repo.remotes.find(item => item.name === 'origin') ?? props.repo.remotes[0]
 const base = remoteWebUrl(remote?.url ?? '')
 if (!base) return toast.error('未配置可打开的远程仓库')
 const host = new URL(base).hostname
 const segment = host === 'gitlab.com' ? '/-/tags/' : host === 'bitbucket.org' ? '/src/' : host === 'github.com' || host === 'gitee.com' ? '/tree/' : null
 if (!segment) return toast.info('暂不支持此平台的标签链接')
 try { await invoke('open_repository_target', {target: base + segment + encodeURIComponent(tag), local: false}) } catch (reason) { toast.error(String(reason)) }
}
async function copy(text: string) { try { await navigator.clipboard.writeText(text); toast.success('已复制') } catch { toast.error('复制失败') } }
</script>
<template>
 <Card class="gap-4 py-4" :class="mode !== 'branches' ? 'flex min-h-0 flex-1 flex-col overflow-hidden' : 'mt-3'"><CardHeader class="flex shrink-0 flex-row items-start justify-between gap-3 px-4"><div class="flex min-w-0 flex-1 flex-col gap-2"><CardTitle>{{ {branches: '分支设置', compare: '分支对比', stash: '暂存改动 Stash', tags: '标签管理', activity: '操作记录'}[mode] }}</CardTitle><CardDescription>{{ {branches: '重命名、管理关联关系，或发布分支到远程', compare: '对比两侧独有提交和最终文件差异', stash: '保存已跟踪和未跟踪文件；恢复后保留暂存记录', tags: '查看版本标签，或从标签创建本地分支', activity: `本机最近 ${settings.options.activityLimit} 条 Git 操作，按当前仓库展示`}[mode] }}</CardDescription></div><AppTooltip v-if="mode === 'tags' || mode === 'stash'" :text="mode === 'tags' ? '刷新标签' : '刷新暂存记录'"><Button variant="outline" size="icon-sm" :aria-label="mode === 'tags' ? '刷新标签' : '刷新暂存记录'" :disabled="loading || working" @click="load"><RefreshCwIcon :class="{'animate-spin': loading}" /></Button></AppTooltip></CardHeader><CardContent class="flex min-w-0 flex-col gap-3 px-4" :class="{'min-h-0 flex-1 overflow-y-auto': mode !== 'branches'}">
  <p v-if="error" class="whitespace-pre-wrap break-all text-sm text-destructive">{{ error }}</p>
  <template v-if="mode === 'branches'"><FieldGroup><Field><FieldLabel>本地分支</FieldLabel><Select v-model="selectedBranch"><SelectTrigger class="w-full"><SelectValue placeholder="选择本地分支" /></SelectTrigger><SelectContent><SelectGroup><SelectItem v-for="name in repo.localBranches" :key="name" :value="name">{{ name }}</SelectItem></SelectGroup></SelectContent></Select></Field><Field><FieldLabel>关联远程分支</FieldLabel><Select v-model="remoteBranch"><SelectTrigger class="w-full"><SelectValue placeholder="选择关联分支" /></SelectTrigger><SelectContent><SelectGroup><SelectItem v-for="name in repo.remoteBranches" :key="name" :value="name">{{ name }}</SelectItem></SelectGroup></SelectContent></Select></Field><Field><FieldLabel>发布目标仓库</FieldLabel><Select v-model="remote"><SelectTrigger class="w-full"><SelectValue placeholder="选择远程仓库" /></SelectTrigger><SelectContent><SelectGroup><SelectItem v-for="item in repo.remotes" :key="item.name" :value="item.name">{{ item.name }}</SelectItem></SelectGroup></SelectContent></Select></Field></FieldGroup><div class="flex flex-wrap gap-2"><Button size="sm" variant="outline" :disabled="disabled || !selectedBranch" @click="prepare('rename', selectedBranch)">重命名</Button><Button size="sm" variant="outline" :disabled="disabled || !selectedBranch || !remoteBranch" @click="prepare('upstream', selectedBranch, remoteBranch)">设置关联</Button><Button size="sm" variant="outline" :disabled="disabled || !selectedBranch" @click="prepare('unset-upstream', selectedBranch)">取消关联</Button><Button size="sm" :disabled="disabled || !selectedBranch || !remote" @click="prepare('publish', selectedBranch, remote)">发布分支</Button></div></template>
  <template v-else-if="mode === 'compare'"><FieldGroup class="grid gap-3 sm:grid-cols-2"><Field><FieldLabel>左侧分支</FieldLabel><Select v-model="compareLeft"><SelectTrigger class="w-full"><SelectValue placeholder="选择左侧分支" /></SelectTrigger><SelectContent><SelectGroup><SelectItem v-for="item in refs" :key="item.value" :value="item.value">{{ item.label }}</SelectItem></SelectGroup></SelectContent></Select></Field><Field><FieldLabel>右侧分支</FieldLabel><Select v-model="compareRight"><SelectTrigger class="w-full"><SelectValue placeholder="选择右侧分支" /></SelectTrigger><SelectContent><SelectGroup><SelectItem v-for="item in refs" :key="item.value" :value="item.value">{{ item.label }}</SelectItem></SelectGroup></SelectContent></Select></Field></FieldGroup><Button class="self-start" size="sm" :disabled="disabled || !compareLeft || !compareRight" @click="showDiff('compare', compareLeft, compareRight)">查看对比</Button></template>
  <template v-else-if="mode === 'stash'"><FieldGroup><Field orientation="horizontal" class="[&>[data-slot=field-label]]:flex-none"><FieldLabel for="stash-message" class="shrink-0">暂存说明</FieldLabel><Input id="stash-message" class="min-w-0 flex-1" v-model="message" placeholder="例如：切换分支前保存登录页面改动" /><Button size="sm" class="shrink-0" :disabled="disabled || !repo.isDirty" @click="prepare('stash-save', message)">保存当前改动</Button></Field></FieldGroup><ItemGroup class="gap-2"><Item v-for="item in stashes" :key="item.hash" role="listitem" variant="outline" size="sm"><ItemContent class="min-w-0"><ItemTitle class="break-all">{{ item.reference }}</ItemTitle><ItemDescription class="break-all">{{ item.subject }}</ItemDescription></ItemContent><ItemActions class="flex-wrap"><Button size="xs" variant="outline" @click="showDiff('stash', item.hash)">查看</Button><Button size="xs" variant="outline" :disabled="disabled" @click="prepare('stash-apply', item.hash)">恢复</Button><Button size="xs" variant="destructive-ghost" :disabled="disabled" @click="prepare('stash-drop', item.hash)">删除</Button></ItemActions></Item></ItemGroup><p v-if="!stashes.length && !loading" class="text-sm text-muted-foreground">暂无暂存记录</p></template>
  <template v-else-if="mode === 'tags'"><ScrollArea class="min-h-0 flex-1"><div class="pr-2 pb-1"><ItemGroup class="gap-2"><Item v-for="tag in tags" :key="tag" role="listitem" variant="outline" size="sm"><ItemContent class="min-w-0"><ItemTitle class="break-all">{{ tag }}</ItemTitle></ItemContent><ItemActions class="flex-wrap"><AppTooltip text="打开远程标签"><Button size="icon-sm" variant="outline" aria-label="打开远程标签" @click="openTag(tag)"><ExternalLinkIcon /></Button></AppTooltip><AppTooltip text="从标签创建分支"><Button size="icon-sm" variant="outline" aria-label="从标签创建分支" :disabled="disabled" @click="prepare('tag-branch', tag)"><GitBranchPlusIcon /></Button></AppTooltip></ItemActions></Item></ItemGroup><p v-if="!tags.length && !loading" class="text-sm text-muted-foreground">暂无版本标签</p></div></ScrollArea></template>
  <template v-else><ItemGroup class="gap-2"><Item v-for="item in history" :key="item.id" role="listitem" variant="outline" size="sm"><ItemContent class="min-w-0"><ItemTitle>{{ item.label }} · {{ {running: '执行中', success: '成功', error: '失败'}[item.status] }}</ItemTitle><ItemDescription>{{ formatAppDate(item.at, settings.options.dateFormat) }}</ItemDescription><pre v-if="item.detail" class="max-h-36 overflow-auto whitespace-pre-wrap break-all text-xs" :class="item.status === 'error' ? 'text-destructive' : 'text-muted-foreground'">{{ item.detail }}</pre></ItemContent><ItemActions><Button size="xs" variant="outline" @click="copy(`${item.label}\n${item.at}\n${item.detail}`)">复制{{ item.status === 'error' ? '错误' : '结果' }}</Button></ItemActions></Item></ItemGroup><p v-if="!history.length" class="text-sm text-muted-foreground">暂无操作记录</p></template>
 </CardContent></Card>
 <Dialog v-model:open="dialogOpen"><DialogContent><DialogHeader><DialogTitle>{{ labels[action] }}</DialogTitle><DialogDescription class="break-all">{{ action === 'stash-drop' ? '删除后无法通过本应用恢复这条暂存记录，请确认内容已不再需要。' : action === 'publish' ? `将本地 ${first} 推送到 ${second} 的同名分支并设置关联。` : action === 'stash-apply' ? '恢复到工作区并尝试恢复暂存区；发生冲突时请在编辑器解决。原记录保留。' : first }}</DialogDescription></DialogHeader><FieldGroup><Field v-if="action === 'rename' || action === 'tag-branch'"><FieldLabel for="tool-new-branch">新分支名称</FieldLabel><Input id="tool-new-branch" v-model="second" placeholder="例如 feature/login" /></Field><p v-if="error" class="whitespace-pre-wrap text-sm text-destructive">{{ error }}</p></FieldGroup><DialogFooter><Button variant="outline" :disabled="working" @click="dialogOpen = false">取消</Button><Button :variant="action === 'stash-drop' ? 'destructive' : 'default'" :disabled="disabled || invalid" @click="execute">{{ working ? '执行中…' : '确认' }}</Button></DialogFooter></DialogContent></Dialog>
 <RepositoryDiff v-model:open="diffOpen" :path="repo.path" :kind="diffKind" :first="diffFirst" :second="diffSecond" :title="diffKind === 'compare' ? '分支对比' : '暂存改动详情'" />
</template>
