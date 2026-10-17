<script setup lang="ts">
import { isTauri } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { ChevronDownIcon, ChevronRightIcon, FolderGit2Icon, FolderPlusIcon, GitBranchIcon, SearchIcon, XIcon, SettingsIcon, MonitorIcon, CheckIcon, RefreshCwIcon, Trash2Icon, PanelLeftCloseIcon, PanelLeftOpenIcon, MoonIcon, SunIcon } from '@lucide/vue'
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { toast } from 'vue-sonner'
import { useWorkspaceStore } from '@/stores/workspace'
import { useSettingsStore } from '@/stores/settings'
import { displayPath } from '@/lib/path'
import AppTooltip from '@/components/AppTooltip.vue'
import { Tooltip, TooltipTrigger, TooltipContent } from '@/components/ui/tooltip'
import { Popover, PopoverContent, PopoverAnchor } from '@/components/ui/popover'
import { InputGroup, InputGroupAddon, InputGroupInput, InputGroupButton } from '@/components/ui/input-group'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '@/components/ui/dialog'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Separator } from '@/components/ui/separator'
import { useDebounceFn } from '@vueuse/core'
import { Card } from '@/components/ui/card'
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@/components/ui/resizable'
import { ScrollArea } from '@/components/ui/scroll-area'
const workspace = useWorkspaceStore()
const settings = useSettingsStore()
const route = useRoute()
const router = useRouter()
const collapsed = ref(false)
const sidebarPanel = ref<InstanceType<typeof ResizablePanel>>()
const saveWidth = useDebounceFn(() => {
 if (collapsed.value) return
 const width = sidebarPanel.value?.getSize()
 if (width && Math.abs(width - settings.sidebarWidth) >= 1) settings.sidebarWidth = Math.round(Math.min(480, Math.max(200, width)))
}, 200)
watch([collapsed, () => settings.sidebarWidth], async () => {
 await nextTick()
 sidebarPanel.value?.resize(collapsed.value ? 64 : settings.sidebarWidth)
})
const directorySearch = ref('')
const themeOpen = ref(false)
const themeOptions = [ {value: 'light', label: '亮色', icon: SunIcon}, {value: 'dark', label: '暗色', icon: MoonIcon}, {value: 'system', label: '跟随系统', icon: MonitorIcon} ] as const
const matches = (path: string) => displayPath(path).toLowerCase().includes(directorySearch.value.trim().toLowerCase())
const folded = ref<string[]>([])
const name = (path: string) => displayPath(path).split(/[\\/]/).filter(Boolean).at(-1)
const equal = (a: string, b: string) => displayPath(a).replaceAll('\\', '/').toLowerCase() === displayPath(b).replaceAll('\\', '/').toLowerCase()
const isRepository = (root: {path: string; kind?: string}) => root.kind ? root.kind === 'repository' : workspace.repositories.some(repo => equal(repo.path, root.path))
const workspaces = computed(() => workspace.roots.filter(root => !isRepository(root) && (matches(root.path) || children(root.path).length > 0)))
const singleRepo = (path: string) => workspace.repositories.find(repo => equal(repo.path, path))
const singles = computed(() => workspace.roots.filter(root => isRepository(root) && matches(root.path)))
const children = (path: string) => workspace.repositories.filter(repo => (matches(path) || matches(repo.path)) && (equal(repo.scanRoot, path) || displayPath(repo.path).replaceAll('\\', '/').toLowerCase().startsWith(displayPath(path).replaceAll('\\', '/').toLowerCase() + '/')))
function toggle(path: string) { folded.value = folded.value.includes(path) ? folded.value.filter(item => item !== path) : [...folded.value, path] }
function selectRoot(path: string) { void router.push({path: '/', query: {root: path}}) }
async function add(kind: 'workspace' | 'repository') {
 if (!isTauri()) return toast.info('请在桌面应用中添加本机目录')
 try {
  const path = await open({directory: true, multiple: false, title: kind === 'workspace' ? '选择工作区目录' : '选择 Git 仓库根目录'})
  if (typeof path !== 'string') return
  await workspace.addRoot(path, kind)
  if (workspace.error) return toast.error(workspace.error)
  if (kind === 'workspace') selectRoot(path)
  else { const repo = workspace.repositories.find(repo => equal(repo.path, path)); if (repo) void router.push({name: 'repository', params: {id: repo.id}}) }
 } catch (reason) { toast.error(String(reason)) }
}
const pendingRemoval = ref<string | null>(null)
const removing = ref(false)
function remove(path: string) { pendingRemoval.value = path }
async function confirmRemoval() {
 const path = pendingRemoval.value
 if (!path || removing.value) return
 removing.value = true
 try {
  const selected = workspace.repositories.find(repo => repo.id === route.params.id)
  const belongs = selected && (equal(selected.scanRoot, path) || equal(selected.path, path))
  await workspace.removeRoot(path)
  if (route.query.root === path || belongs) await router.push('/')
  pendingRemoval.value = null
  toast.success('已从列表移除，磁盘文件保留')
 } catch (reason) { toast.error(String(reason)) }
 finally { removing.value = false }
}
onMounted(() => { if (isTauri() && !workspace.lastUpdated) void workspace.refresh() })
</script>
<template>
 <div class="h-dvh min-w-0 overflow-hidden bg-muted/50 p-3">
 <ResizablePanelGroup direction="horizontal" class="min-h-0">
 <ResizablePanel ref="sidebarPanel" size-unit="px" :default-size="settings.sidebarWidth" :min-size="collapsed ? 64 : 200" :max-size="collapsed ? 64 : 480" @resize="saveWidth">
  <Card class="flex h-full min-h-0 flex-col gap-0 overflow-hidden bg-sidebar py-0 shadow-sm">
   <Button as-child variant="ghost" class="h-16 shrink-0 justify-start rounded-none px-4" aria-label="RepoDeck 首页"><RouterLink to="/"><span class="flex size-8 shrink-0 items-center justify-center rounded-lg bg-primary text-primary-foreground"><GitBranchIcon /></span><span v-if="!collapsed" class="text-base font-semibold">RepoDeck</span></RouterLink></Button>
   <div v-if="!collapsed" class="flex shrink-0 items-center gap-1 p-2">
    <InputGroup class="min-w-0 flex-1"><InputGroupAddon><SearchIcon /></InputGroupAddon><InputGroupInput v-model="directorySearch" placeholder="搜索仓库" aria-label="搜索侧栏目录" /><InputGroupAddon v-if="directorySearch" align="inline-end"><InputGroupButton size="icon-sm" aria-label="清空搜索" @click="directorySearch = ''"><XIcon /></InputGroupButton></InputGroupAddon></InputGroup>
    <AppTooltip text="刷新全部"><Button variant="ghost" size="icon" aria-label="刷新全部" :disabled="workspace.isLoading || workspace.operationBusy" @click="workspace.refresh()"><RefreshCwIcon :class="{'animate-spin': workspace.isLoading}" /></Button></AppTooltip>
   </div>
   <Separator />
   <ScrollArea class="min-h-0 flex-1">
    <nav class="flex flex-col gap-4 p-2" aria-label="数据目录">
     <section class="flex flex-col gap-1">
      <div class="flex items-center justify-between px-2 py-2"><span v-if="!collapsed" class="flex items-center gap-2 text-sm font-medium text-muted-foreground">工作区 <Badge variant="secondary">{{ workspaces.length }}</Badge></span><AppTooltip text="添加工作区"><Button size="icon-sm" variant="ghost" aria-label="添加工作区" :disabled="workspace.isLoading || workspace.operationBusy" @click="add('workspace')"><FolderPlusIcon /></Button></AppTooltip></div>
      <p v-if="!collapsed && !workspaces.length" class="px-2 pb-3 text-sm text-muted-foreground">{{ directorySearch ? '无匹配结果' : '暂无数据' }}</p>
      <div v-for="root in workspaces" :key="root.path" class="min-w-0">
       <div class="flex items-center gap-1">
        <AppTooltip :text="folded.includes(root.path) ? '展开工作区' : '收起工作区'" v-if="!collapsed"><Button size="icon-sm" variant="ghost" :aria-label="folded.includes(root.path) ? '展开工作区' : '收起工作区'" :aria-expanded="!folded.includes(root.path)" @click="toggle(root.path)"><ChevronRightIcon v-if="folded.includes(root.path)" /><ChevronDownIcon v-else /></Button></AppTooltip>
        <AppTooltip :text="displayPath(root.path)"><Button class="min-w-0 flex-1 justify-start" :variant="route.query.root === root.path ? 'secondary' : 'ghost'" @click="selectRoot(root.path)"><FolderGit2Icon data-icon="inline-start" /><span v-if="!collapsed" class="truncate">{{ name(root.path) }}</span></Button></AppTooltip>
        <AppTooltip text="移除工作区" v-if="!collapsed"><Button variant="destructive-ghost" size="icon-sm" aria-label="移除工作区" :disabled="workspace.operationBusy || workspace.isLoading" @click="remove(root.path)"><Trash2Icon /></Button></AppTooltip>
       </div>
       <div v-if="!collapsed && !folded.includes(root.path)" class="ml-5 flex flex-col gap-1 border-l pl-2">
        <AppTooltip :text="displayPath(repo.path)" v-for="repo in children(root.path)" :key="repo.id"><Button as-child :variant="route.params.id === repo.id ? 'secondary' : 'ghost'" class="min-w-0 justify-start"><RouterLink :to="{name: 'repository', params: {id: repo.id}}"><GitBranchIcon data-icon="inline-start" /><span class="truncate">{{ repo.name }}</span><span v-if="repo.isDirty" class="ml-auto text-warning" aria-label="有未提交改动">●</span></RouterLink></Button></AppTooltip>
        <p v-if="!children(root.path).length" class="p-2 text-xs text-muted-foreground">暂无 Git 仓库</p>
       </div>
      </div>
     </section>
     <Separator />
     <section class="flex flex-col gap-1">
      <div class="flex items-center justify-between px-2 py-2"><span v-if="!collapsed" class="flex items-center gap-2 text-sm font-medium text-muted-foreground">仓库 <Badge variant="secondary">{{ singles.length }}</Badge></span><AppTooltip text="添加单个仓库"><Button size="icon-sm" variant="ghost" aria-label="添加单个仓库" :disabled="workspace.isLoading || workspace.operationBusy" @click="add('repository')"><FolderPlusIcon /></Button></AppTooltip></div>
      <p v-if="!collapsed && !singles.length" class="px-2 pb-3 text-sm text-muted-foreground">{{ directorySearch ? '无匹配结果' : '暂无数据' }}</p>
      <div v-for="root in singles" :key="root.path" class="flex min-w-0 items-center gap-1">
       <AppTooltip :text="displayPath(root.path)"><Button :variant="singleRepo(root.path)?.id === route.params.id ? 'secondary' : 'ghost'" class="min-w-0 flex-1 justify-start" @click="singleRepo(root.path) ? router.push({name: 'repository', params: {id: singleRepo(root.path)!.id}}) : selectRoot(root.path)"><GitBranchIcon data-icon="inline-start" /><span v-if="!collapsed" class="truncate">{{ name(root.path) }}</span></Button></AppTooltip>
       <AppTooltip text="移除仓库" v-if="!collapsed"><Button variant="destructive-ghost" size="icon-sm" aria-label="移除仓库" :disabled="workspace.operationBusy || workspace.isLoading" @click="remove(root.path)"><Trash2Icon /></Button></AppTooltip>
      </div>
     </section>
    </nav>
   </ScrollArea>
   <Separator />
   <div :class="['flex shrink-0 items-center gap-1 p-2', collapsed ? 'justify-center' : 'justify-start']">
    <template v-if="!collapsed">
     <AppTooltip text="设置"><Button as-child variant="ghost" size="icon" aria-label="设置"><RouterLink to="/settings"><SettingsIcon /></RouterLink></Button></AppTooltip>
     <Tooltip><Popover v-model:open="themeOpen"><PopoverAnchor as-child><TooltipTrigger as-child><Button variant="ghost" size="icon" aria-label="外观模式" aria-haspopup="dialog" :aria-expanded="themeOpen" @click.stop="themeOpen = !themeOpen"><MonitorIcon v-if="settings.theme === 'system'" /><MoonIcon v-else-if="settings.isDark" /><SunIcon v-else /></Button></TooltipTrigger></PopoverAnchor><TooltipContent side="top">外观模式</TooltipContent><PopoverContent side="top" align="start" :side-offset="6" class="w-40 rounded-md p-1"><div role="menu" aria-label="外观模式" class="flex flex-col gap-0.5"><Button v-for="option in themeOptions" :key="option.value" variant="ghost" role="menuitemradio" :aria-checked="settings.theme === option.value" class="h-8 w-full justify-start gap-2 rounded-sm px-2 font-normal" @click="settings.theme = option.value; themeOpen = false"><component :is="option.icon" data-icon="inline-start" />{{ option.label }}<CheckIcon v-if="settings.theme === option.value" class="ml-auto" /></Button></div></PopoverContent></Popover></Tooltip>
    </template>
    <AppTooltip :text="collapsed ? '展开侧栏' : '收起侧栏'"><Button variant="ghost" size="icon" :aria-label="collapsed ? '展开侧栏' : '收起侧栏'" @click="collapsed = !collapsed"><PanelLeftOpenIcon v-if="collapsed" /><PanelLeftCloseIcon v-else /></Button></AppTooltip>
   </div>
  </Card>
 </ResizablePanel>
 <ResizableHandle :disabled="collapsed" class="shell-resize-handle" aria-label="调整侧边栏宽度" />
 <ResizablePanel :min-size="360" size-unit="px">
  <Card :class="['h-full min-h-0 min-w-0 gap-0 overflow-hidden py-0 shadow-sm', route.name !== 'home' && 'rounded-none border-0 bg-transparent shadow-none']">
   <ScrollArea v-if="route.name === 'home'" class="h-full"><div class="app-content mx-auto max-w-[1600px] p-5"><slot /></div></ScrollArea>
   <div v-else class="app-content h-full"><slot /></div>
  </Card>
 </ResizablePanel>
 </ResizablePanelGroup>
 <Dialog :open="pendingRemoval !== null" @update:open="value => { if (!value && !removing) pendingRemoval = null }"><DialogContent><DialogHeader><DialogTitle>确认移除{{ pendingRemoval && workspace.roots.find(root => root.path === pendingRemoval)?.kind === 'workspace' ? '工作区' : '目录' }}？</DialogTitle><DialogDescription>仅从 RepoDeck 中取消登记，磁盘上的仓库、文件和 Git 历史都会保留。移除工作区会同时隐藏其下的仓库</DialogDescription></DialogHeader><p class="break-all text-sm">{{ pendingRemoval ? displayPath(pendingRemoval) : '' }}</p><DialogFooter><Button variant="outline" :disabled="removing" @click="pendingRemoval = null">取消</Button><Button variant="destructive" :disabled="removing || workspace.operationBusy" @click="confirmRemoval">{{ removing ? '正在移除…' : '确认移除' }}</Button></DialogFooter></DialogContent></Dialog>

 </div>
</template>
