<script setup lang="ts">
import { isTauri } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { ChevronDownIcon, ChevronRightIcon, GitBranchIcon, PinIcon, SearchIcon, XIcon, SettingsIcon, MonitorIcon, CheckIcon, RefreshCwIcon, Trash2Icon, PanelLeftCloseIcon, PanelLeftOpenIcon, MoonIcon, SunIcon } from '@lucide/vue'
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { toast } from 'vue-sonner'
import { useWorkspaceStore } from '@/stores/workspace'
import { useSettingsStore } from '@/stores/settings'
import { business } from '@/lib/business'
import { displayPath } from '@/lib/path'
import GroupsSidebar from '@/components/GroupsSidebar.vue'
import GroupManager from '@/components/GroupManager.vue'
import AppTooltip from '@/components/AppTooltip.vue'
import { Tooltip, TooltipTrigger, TooltipContent } from '@/components/ui/tooltip'
import { Popover, PopoverContent, PopoverAnchor } from '@/components/ui/popover'
import { InputGroup, InputGroupAddon, InputGroupInput, InputGroupButton } from '@/components/ui/input-group'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '@/components/ui/dialog'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Separator } from '@/components/ui/separator'
import AddRepositoryMenu from '@/components/AddRepositoryMenu.vue'
import TaskCenter from '@/components/TaskCenter.vue'
import CloneRepositoryDialog from '@/components/CloneRepositoryDialog.vue'
import { useCloneStore } from '@/stores/clone'
import { useDebounceFn } from '@vueuse/core'
import { Card } from '@/components/ui/card'
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@/components/ui/resizable'
import { ScrollArea } from '@/components/ui/scroll-area'
const workspace = useWorkspaceStore()
const clone = useCloneStore()
const settings = useSettingsStore()
const route = useRoute()
const router = useRouter()
const collapsed = ref(false)
const repositoriesExpanded = ref(true)
const repositoryModule = computed(() => route.name === 'home' || route.name === 'repository')
const sidebarPanel = ref<InstanceType<typeof ResizablePanel>>()
const saveWidth = useDebounceFn(() => {
 if (collapsed.value || route.name === 'settings') return
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
const equal = (a: string, b: string) => displayPath(a).replaceAll('\\', '/').toLowerCase() === displayPath(b).replaceAll('\\', '/').toLowerCase()
const singles = computed(() => workspace.repositories.filter(repo => matches(repo.path) || repo.name.toLowerCase().includes(directorySearch.value.trim().toLowerCase())).sort((a,b) => { const ai=business.pins.indexOf(a.id), bi=business.pins.indexOf(b.id); return (ai<0?99999:ai)-(bi<0?99999:bi) || a.name.localeCompare(b.name) }))
async function add(kind: 'repository') {
 if (!isTauri()) return toast.info('请在桌面应用中添加本机目录')
 try {
  const path = await open({directory: true, multiple: false, title: '选择 Git 仓库根目录'})
  if (typeof path !== 'string') return
  await workspace.addRoot(path, kind)
  if (workspace.error) return toast.error(workspace.error)
  { const repo = workspace.repositories.find(repo => equal(repo.path, path)); if (repo) void router.push({name: 'repository', params: {id: repo.id}}) }
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
  const belongs = selected && equal(selected.path, path)
  await workspace.removeRepository(path)
  if (route.query.root === path || belongs) await router.push('/')
  pendingRemoval.value = null
  toast.success('已从列表移除，磁盘文件保留')
 } catch (reason) { toast.error(String(reason)) }
 finally { removing.value = false }
}
onMounted(() => { if (isTauri() && !workspace.lastUpdated) void workspace.refresh(settings.options.startupScan === 'on') })
</script>
<template>
 <GroupManager /><CloneRepositoryDialog />
 <div class="h-dvh min-w-0 overflow-hidden bg-muted/50 p-3">
 <ResizablePanelGroup direction="horizontal" class="min-h-0">
 <ResizablePanel v-if="route.name !== 'settings'" ref="sidebarPanel" size-unit="px" :default-size="settings.sidebarWidth" :min-size="collapsed ? 64 : 200" :max-size="collapsed ? 64 : 480" @resize="saveWidth">
  <Card class="flex h-full min-h-0 flex-col gap-0 overflow-hidden bg-card py-0 shadow-sm">
   <RouterLink to="/" class="flex h-16 shrink-0 items-center gap-2 px-4 outline-none focus-visible:ring-3 focus-visible:ring-inset focus-visible:ring-ring/50" aria-label="RepoDeck 首页"><span class="flex size-8 shrink-0 items-center justify-center rounded-lg bg-primary text-primary-foreground"><GitBranchIcon class="size-[1.125rem]" /></span><span v-if="!collapsed" class="text-base font-semibold">RepoDeck</span></RouterLink>

   <div v-if="!collapsed && repositoryModule" class="flex shrink-0 items-center gap-1 p-2">
    <InputGroup class="min-w-0 flex-1"><InputGroupAddon><SearchIcon /></InputGroupAddon><InputGroupInput v-model="directorySearch" placeholder="搜索仓库" aria-label="搜索侧栏目录" /><InputGroupAddon v-if="directorySearch" align="inline-end"><InputGroupButton size="icon-sm" aria-label="清空搜索" @click="directorySearch = ''"><XIcon /></InputGroupButton></InputGroupAddon></InputGroup>
    <AppTooltip text="刷新全部"><Button variant="ghost" size="icon" aria-label="刷新全部" :disabled="workspace.isLoading || workspace.operationBusy" @click="workspace.refresh(true)"><RefreshCwIcon :class="{'animate-spin': workspace.isLoading}" /></Button></AppTooltip>
   </div>
   <ScrollArea class="min-h-0 flex-1">
    <nav v-if="repositoryModule" class="flex flex-col gap-4 p-2" aria-label="仓库导航">
     <section class="flex flex-col gap-1">
      <div class="flex items-center justify-between py-2"><Button v-if="!collapsed" variant="plain" size="sm" class="justify-start px-0" @click="router.push('/')">全部仓库 <Badge variant="secondary">{{ singles.length }}</Badge></Button><div class="flex items-center gap-1"><AppTooltip v-if="!collapsed" :text="repositoriesExpanded ? '收起仓库列表' : '展开仓库列表'"><Button size="icon-sm" variant="ghost" :aria-label="repositoriesExpanded ? '收起仓库列表' : '展开仓库列表'" :aria-expanded="repositoriesExpanded" aria-controls="sidebar-repositories" @click="repositoriesExpanded = !repositoriesExpanded"><ChevronDownIcon v-if="repositoriesExpanded" /><ChevronRightIcon v-else /></Button></AppTooltip><AddRepositoryMenu compact :disabled="workspace.isLoading || workspace.operationBusy" @local="add('repository')" /></div></div>
      <div id="sidebar-repositories" v-show="collapsed || repositoriesExpanded" class="flex min-w-0 flex-col gap-1"><p v-if="!collapsed && !singles.length" class="flex min-h-20 items-center justify-center px-2 py-4 text-center text-sm text-muted-foreground">{{ directorySearch ? '无匹配结果' : '暂无数据' }}</p>
      <div v-for="repo in singles" :key="repo.id" class="flex min-w-0 items-center gap-1">
       <AppTooltip :text="displayPath(repo.path)"><Button :variant="repo.id === route.params.id ? 'secondary' : 'ghost'" class="min-w-0 flex-1 justify-start" @click="router.push({name: 'repository', params: {id: repo.id}})"><GitBranchIcon data-icon="inline-start" /><span v-if="!collapsed" class="truncate">{{ repo.name }}</span><PinIcon v-if="business.pins.includes(repo.id) && !collapsed" class="ml-auto" /></Button></AppTooltip>
       <AppTooltip text="移除仓库" v-if="!collapsed"><Button variant="destructive-ghost" size="icon-sm" aria-label="移除仓库" :disabled="workspace.operationBusy || workspace.isLoading" @click="remove(repo.path)"><Trash2Icon /></Button></AppTooltip>
      </div>
      </div>
     </section>
     <GroupsSidebar :collapsed="collapsed" :search="directorySearch" />
    </nav>
   </ScrollArea>
   <TaskCenter :compact="collapsed" /><Button v-if="clone.tasks.length" variant="ghost" class="m-2 shrink-0" @click="clone.dialogOpen = true">{{ collapsed ? '克隆' : clone.running ? '克隆进行中 · 查看任务' : '查看克隆任务' }}</Button>
   <Separator />
   <div class="flex shrink-0 items-center justify-center gap-1 p-2">
    <AppTooltip :text="collapsed ? '展开侧栏' : '收起侧栏'"><Button variant="ghost" size="icon" :aria-label="collapsed ? '展开侧栏' : '收起侧栏'" @click="collapsed = !collapsed"><PanelLeftOpenIcon v-if="collapsed" /><PanelLeftCloseIcon v-else /></Button></AppTooltip>
    <template v-if="!collapsed">
     <Tooltip><Popover v-model:open="themeOpen"><PopoverAnchor as-child><TooltipTrigger as-child><Button variant="ghost" size="icon" aria-label="外观模式" aria-haspopup="dialog" :aria-expanded="themeOpen" @click.stop="themeOpen = !themeOpen"><MonitorIcon v-if="settings.theme === 'system'" /><MoonIcon v-else-if="settings.isDark" /><SunIcon v-else /></Button></TooltipTrigger></PopoverAnchor><TooltipContent side="top">外观模式</TooltipContent><PopoverContent side="top" align="start" :side-offset="6" class="w-40 rounded-md p-1"><div role="menu" aria-label="外观模式" class="flex flex-col gap-0.5"><Button v-for="option in themeOptions" :key="option.value" variant="ghost" role="menuitemradio" :aria-checked="settings.theme === option.value" class="h-8 w-full justify-start gap-2 rounded-sm px-2 font-normal" @click="settings.theme = option.value; themeOpen = false"><component :is="option.icon" data-icon="inline-start" />{{ option.label }}<CheckIcon v-if="settings.theme === option.value" class="ml-auto" /></Button></div></PopoverContent></Popover></Tooltip>
     <AppTooltip text="设置"><Button as-child variant="ghost" size="icon" aria-label="设置"><RouterLink to="/settings"><SettingsIcon /></RouterLink></Button></AppTooltip>
    </template>
   </div>
  </Card>
 </ResizablePanel>
 <ResizableHandle v-if="route.name !== 'settings'" :disabled="collapsed" class="shell-resize-handle" aria-label="调整侧边栏宽度" />
 <ResizablePanel :min-size="360" size-unit="px">
  <Card :class="['h-full min-h-0 min-w-0 gap-0 overflow-hidden py-0 shadow-sm', route.name !== 'home' && 'rounded-none border-0 shadow-none']">
   <ScrollArea v-if="route.name === 'home'" class="h-full"><div class="app-content mx-auto max-w-[1600px] p-5"><slot /></div></ScrollArea>
   <div v-else class="app-content h-full"><slot /></div>
  </Card>
 </ResizablePanel>
 </ResizablePanelGroup>
 <Dialog :open="pendingRemoval !== null" @update:open="value => { if (!value && !removing) pendingRemoval = null }"><DialogContent><DialogHeader><DialogTitle>确认移除仓库？</DialogTitle><DialogDescription>仅从 RepoDeck 中取消登记，磁盘上的仓库、文件和 Git 历史都会保留。固定扫描会跳过此仓库；再次手动添加即可恢复登记</DialogDescription></DialogHeader><p class="break-all text-sm">{{ pendingRemoval ? displayPath(pendingRemoval) : '' }}</p><DialogFooter><Button variant="outline" :disabled="removing" @click="pendingRemoval = null">取消</Button><Button variant="destructive" :disabled="removing || workspace.operationBusy" @click="confirmRemoval">{{ removing ? '正在移除…' : '确认移除' }}</Button></DialogFooter></DialogContent></Dialog>

 </div>
</template>
