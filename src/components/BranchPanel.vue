<script setup lang="ts">
import { ScrollArea } from '@/components/ui/scroll-area'
import { computed, ref } from 'vue'
import { invoke } from '@/lib/git-client'
import { GitBranchIcon, GitMergeIcon, PlusIcon, SearchIcon, Trash2Icon, TriangleAlertIcon } from '@lucide/vue'
import { toast } from 'vue-sonner'
import type { RepositorySummary } from '@/types/git'
import { useWorkspaceStore } from '@/stores/workspace'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Item, ItemGroup, ItemMedia, ItemContent, ItemTitle, ItemActions } from '@/components/ui/item'
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '@/components/ui/card'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs'
import { InputGroup, InputGroupInput, InputGroupAddon } from '@/components/ui/input-group'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '@/components/ui/dialog'
import { FieldGroup, Field, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Checkbox } from '@/components/ui/checkbox'
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Alert, AlertTitle, AlertDescription } from '@/components/ui/alert'
import { Empty, EmptyHeader, EmptyTitle, EmptyDescription } from '@/components/ui/empty'

const props = defineProps<{ repo: RepositorySummary; disabled: boolean }>()
const emit = defineEmits<{ checkout: [branch: string, remote: boolean]; activity: [busy: boolean]; updated: [] }>()
const workspace = useWorkspaceStore()
const search = ref('')
const working = ref(false)
const error = ref('')
const dialogOpen = ref(false)
const action = ref<'create' | 'merge' | 'delete'>('create')
const newName = ref('')
const sourceRef = ref('HEAD')
const deleteName = ref('')
const switchAfter = ref(true)
const localBranches = computed(() => props.repo.localBranches.filter(matches))
const remoteBranches = computed(() => props.repo.remoteBranches.filter(matches))
function matches(branch: string) { return branch.toLowerCase().includes(search.value.trim().toLowerCase()) }
const sources = computed(() => [
  ...props.repo.localBranches.map((name) => ({ value: `refs/heads/${name}`, label: `${name} · 本地` })),
  ...props.repo.remoteBranches.map((name) => ({ value: `refs/remotes/${name}`, label: `${name} · 远端` })),
])
const availableSources = computed(() => action.value === 'merge'
  ? sources.value.filter((item) => item.value !== `refs/heads/${props.repo.currentBranch}`)
  : [{ value: 'HEAD', label: 'HEAD · 当前提交' }, ...sources.value])
const sourceLabel = computed(() => availableSources.value.find((item) => item.value === sourceRef.value)?.label ?? sourceRef.value)
const blocked = computed(() => {
  const repo = props.repo
  if (repo.hasConflicts || repo.operationInProgress) return '请先在 IDE 或终端解决冲突并完成当前 Git 操作，再刷新'
  if (action.value === 'create') {
    if (!newName.value.trim()) return '请填写新分支名称'
    if (repo.localBranches.includes(newName.value.trim())) return '该本地分支已存在，请使用其他名称'
    if (switchAfter.value && repo.isDirty) return '工作区有未提交改动，请先处理，或取消“创建后切换”'
  }
  if (action.value === 'merge') {
    if (!repo.currentBranch || repo.isDetached) return '请先切换到一个本地分支，再合并'
    if (repo.isDirty) return '工作区有未提交改动，请先审阅并提交，或在其他 Git 工具中处理后刷新'
  }
  if (action.value !== 'delete' && !availableSources.value.some((item) => item.value === sourceRef.value)) return '请选择有效的起点或来源分支'
  if (action.value === 'delete' && deleteName.value === repo.currentBranch) return '不能删除当前分支，请先切换到其他分支'
  return ''
})

function openCreate() {
  error.value = ''
  action.value = 'create'
  newName.value = ''
  sourceRef.value = props.repo.currentBranch && props.repo.localBranches.includes(props.repo.currentBranch) ? `refs/heads/${props.repo.currentBranch}` : 'HEAD'
  switchAfter.value = !props.repo.isDirty
  dialogOpen.value = true
}
function openMerge(branch: string, remote: boolean) {
  error.value = ''
  action.value = 'merge'
  sourceRef.value = `refs/${remote ? 'remotes' : 'heads'}/${branch}`
  dialogOpen.value = true
}
function openDelete(branch: string) {
  error.value = ''
  action.value = 'delete'
  deleteName.value = branch
  dialogOpen.value = true
}
async function confirm() {
  if (working.value || props.disabled || blocked.value) return
  const repo = props.repo
  const operation = action.value
  const label = operation === 'create' ? '新建分支' : operation === 'merge' ? '合并分支' : '删除本地分支'
  working.value = true
  emit('activity', true)
  error.value = ''
  try {
    if (operation === 'create') await invoke('create_branch', { path: repo.path, branch: newName.value.trim(), startRef: sourceRef.value, switchAfter: switchAfter.value })
    else if (operation === 'merge') await invoke('merge_branch', { path: repo.path, sourceRef: sourceRef.value, targetBranch: repo.currentBranch })
    else await invoke('delete_local_branch', { path: repo.path, branch: deleteName.value })
    dialogOpen.value = false
    toast.success(`${label}完成`)
  } catch (reason) {
    error.value = String(reason)
    toast.error(error.value)
  } finally {
    await workspace.refresh()
    emit('updated')
    working.value = false
    emit('activity', false)
  }
}
</script>

<template>
  <Card class="gap-4 py-4">
    <CardHeader class="gap-3 px-4">
      <div class="flex flex-wrap items-center justify-between gap-3"><div><CardTitle>分支</CardTitle><CardDescription class="mt-1">管理本地与远端分支，合并到当前分支</CardDescription></div><Button size="sm" :disabled="disabled || working" @click="openCreate"><PlusIcon data-icon="inline-start" />新建分支</Button></div>
      <InputGroup><InputGroupInput v-model="search" aria-label="搜索分支" placeholder="搜索分支名称" /><InputGroupAddon><SearchIcon /></InputGroupAddon></InputGroup>
    </CardHeader>
    <CardContent class="min-w-0 px-4">
      <Alert v-if="error" variant="destructive" class="mb-4"><TriangleAlertIcon /><AlertTitle>操作未完成</AlertTitle><AlertDescription class="break-all">{{ error }}</AlertDescription></Alert>
      <Tabs default-value="local" class="gap-3">
        <TabsList><TabsTrigger value="local">本地分支 · {{ repo.localBranches.length }}</TabsTrigger><TabsTrigger value="remote">远端分支 · {{ repo.remoteBranches.length }}</TabsTrigger></TabsList>
        <TabsContent value="local" class="min-w-0">
          <ScrollArea class="max-h-80">
            <ItemGroup class="gap-2">
            <Item v-for="branch in localBranches" :key="branch" role="listitem" variant="outline" size="sm" class="min-w-0">
              <ItemMedia><GitBranchIcon class="size-4 text-muted-foreground" /></ItemMedia><ItemContent class="min-w-0"><ItemTitle class="max-w-full"><span class="truncate" :title="branch">{{ branch }}</span><Badge v-if="branch === repo.currentBranch" variant="secondary">当前</Badge></ItemTitle></ItemContent>
              <ItemActions class="gap-1"><Button variant="outline" size="xs" :disabled="disabled || working || branch === repo.currentBranch" @click="emit('checkout', branch, false)">切换</Button><Button variant="ghost" size="xs" :disabled="disabled || working || branch === repo.currentBranch" @click="openMerge(branch, false)"><GitMergeIcon data-icon="inline-start" />合并</Button><Button variant="destructive-ghost" size="icon-xs" :aria-label="`删除本地分支 ${branch}`" :title="branch === repo.currentBranch ? '不能删除当前分支' : '删除本地分支'" :disabled="disabled || working || branch === repo.currentBranch" @click="openDelete(branch)"><Trash2Icon /></Button></ItemActions>
            </Item>
            </ItemGroup>
            <Empty v-if="!localBranches.length" class="p-4"><EmptyHeader><EmptyTitle>{{ search ? '没有匹配分支' : '暂无本地分支' }}</EmptyTitle><EmptyDescription>可新建分支，或从远端创建本地跟踪分支</EmptyDescription></EmptyHeader></Empty>
          </ScrollArea>
        </TabsContent>
        <TabsContent value="remote" class="min-w-0">
          <p class="mb-3 text-xs text-muted-foreground">来自最近一次 Fetch。选择远端分支可创建本地跟踪分支，或合并到当前本地分支</p>
          <ScrollArea class="max-h-80">
            <ItemGroup class="gap-2">
            <Item v-for="branch in remoteBranches" :key="branch" role="listitem" variant="outline" size="sm" class="min-w-0"><ItemMedia><GitBranchIcon class="size-4 text-muted-foreground" /></ItemMedia><ItemContent class="min-w-0"><ItemTitle class="max-w-full"><span class="truncate" :title="branch">{{ branch }}</span><Badge v-if="branch === repo.upstream" variant="secondary">跟踪</Badge></ItemTitle></ItemContent><ItemActions class="gap-1"><Button variant="outline" size="xs" :disabled="disabled || working" @click="emit('checkout', branch, true)">创建并切换</Button><Button variant="destructive-ghost" size="xs" :disabled="disabled || working" @click="openMerge(branch, true)"><GitMergeIcon data-icon="inline-start" />合并</Button></ItemActions></Item>
            </ItemGroup>
            <Empty v-if="!remoteBranches.length" class="p-4"><EmptyHeader><EmptyTitle>{{ search ? '没有匹配分支' : '暂无远端分支' }}</EmptyTitle><EmptyDescription>检查远端配置，并通过 Fetch 获取分支</EmptyDescription></EmptyHeader></Empty>
          </ScrollArea>
        </TabsContent>
      </Tabs>
    </CardContent>
  </Card>

  <Dialog v-model:open="dialogOpen">
    <DialogContent class="max-h-[85dvh] flex flex-col">
      <DialogHeader><DialogTitle>{{ action === 'create' ? '新建本地分支' : action === 'merge' ? '合并分支' : '删除本地分支' }}</DialogTitle><DialogDescription>{{ repo.name }} · 当前分支 {{ repo.currentBranch ?? 'Detached HEAD' }}</DialogDescription></DialogHeader><ScrollArea class="min-h-0 flex-1 max-h-[60dvh]">
      <FieldGroup>
        <Field v-if="action === 'create'"><FieldLabel for="branch-new-name">新分支名称</FieldLabel><Input id="branch-new-name" v-model="newName" placeholder="例如 feature/login" :disabled="working" /></Field>
        <Field v-if="action !== 'delete'"><FieldLabel for="branch-source">{{ action === 'create' ? '创建起点' : '来源分支' }}</FieldLabel><Select v-model="sourceRef" :disabled="working"><SelectTrigger id="branch-source" class="w-full min-w-0"><SelectValue placeholder="选择分支" /></SelectTrigger><SelectContent><SelectGroup><SelectItem v-for="source in availableSources" :key="source.value" :value="source.value">{{ source.label }}</SelectItem></SelectGroup></SelectContent></Select></Field>
        <Field v-if="action === 'create'" orientation="horizontal"><Checkbox id="branch-switch-after" v-model="switchAfter" :disabled="working" /><FieldLabel for="branch-switch-after">创建后切换到新分支</FieldLabel></Field>
        <p v-if="action === 'create'" class="text-xs leading-5 text-muted-foreground">从远端分支创建时会设置跟踪关系；从本地分支或 HEAD 创建时不会自动设置 upstream</p>
        <Alert v-if="action === 'merge'" variant="info"><GitMergeIcon /><AlertTitle>合并方向</AlertTitle><AlertDescription class="break-all">{{ sourceLabel }} → {{ repo.currentBranch }}。可以快进时直接快进，否则创建合并提交；若出现冲突，请在 IDE 或终端解决。远端来源使用最近 Fetch 的引用</AlertDescription></Alert>
        <Alert v-if="action === 'delete'" variant="warning"><Trash2Icon /><AlertTitle class="break-all">确认删除 {{ deleteName }}？</AlertTitle><AlertDescription>仅删除这个本地分支。系统会检查它的提交是否已合并到当前 HEAD，未合并时拒绝删除；其他工作区占用的分支也无法删除</AlertDescription></Alert>
        <Alert v-if="blocked" variant="warning"><TriangleAlertIcon /><AlertTitle>请先处理</AlertTitle><AlertDescription>{{ blocked }}</AlertDescription></Alert>
        <Alert v-if="error" variant="destructive"><TriangleAlertIcon /><AlertTitle>操作未完成</AlertTitle><AlertDescription class="break-all">{{ error }}</AlertDescription></Alert>
      </FieldGroup>
      </ScrollArea><DialogFooter><Button variant="outline" :disabled="working" @click="dialogOpen = false">取消</Button><Button :variant="action === 'delete' ? 'destructive' : 'default'" :disabled="working || disabled || !!blocked" @click="confirm">{{ working ? '执行中…' : action === 'create' ? (switchAfter ? '创建并切换' : '创建分支') : action === 'merge' ? '确认合并' : '确认删除' }}</Button></DialogFooter>
    </DialogContent>
  </Dialog>
</template>
