<script setup lang="ts">
import { ClipboardListIcon } from '@lucide/vue'
import AppTooltip from '@/components/AppTooltip.vue'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { isTauri } from '@tauri-apps/api/core'
import { useRouter } from 'vue-router'
import { toast } from 'vue-sonner'
import { gitActivities, invoke } from '@/lib/git-client'
import { flushBusiness } from '@/lib/business'
import { explainGitError } from '@/lib/git-errors'
import { useSettingsStore } from '@/stores/settings'
import { useCloneStore } from '@/stores/clone'
import { useWorkspaceStore } from '@/stores/workspace'
import { displayPath } from '@/lib/path'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '@/components/ui/dialog'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
defineProps<{compact?:boolean}>()
const clone = useCloneStore(); const workspace = useWorkspaceStore(); const router = useRouter()
const open = ref(false); const closing = ref(false); const waiting = ref(false); const filter = ref('all')
const active = computed(() => clone.running || workspace.operationBusy || gitActivities.value.some(t => t.status === 'running'))
const count = computed(() => Number(clone.running) + gitActivities.value.filter(t => t.status === 'running').length)
const rows = computed(() => [
  ...clone.tasks.map(task => ({id:task.id,path:task.path,label:`克隆 ${task.name}`,state:task.state === 'stopping' ? 'running' : task.state,detail:task.error || task.output,clone:true, at:''})),
  ...gitActivities.value.map(task => ({id:task.id,path:task.path,label:task.label,state:task.status,detail:task.detail,clone:false, at:task.at})),
].filter(task => filter.value === 'all' || task.state === filter.value))
async function locate(path: string) { await workspace.refresh(); const repo = workspace.repositories.find(r => r.path === path); if (repo) await router.push({name:'repository',params:{id:repo.id}}); else await router.push('/'); open.value = false }
async function retryFetch(path: string) { if (active.value) return; workspace.operationBusy = true; try { await invoke('fetch_repository',{path}); await workspace.refresh() } catch(e) { toast.error(String(e)) } finally { workspace.operationBusy = false } }
async function copy(text: string) { try { await navigator.clipboard.writeText(text); toast.success('日志已复制') } catch { toast.error('复制失败') } }
async function exit() { try { await clone.flushTasks(); await flushBusiness(); await useSettingsStore().flushSettings(); await getCurrentWindow().destroy() } catch(e) { waiting.value = false; toast.error(String(e)) } }
async function stopAndExit() {
  waiting.value = true
  try { const task = clone.tasks.find(t => t.state === 'running'); if (task) await clone.stop(task.id,false); if (!active.value) await exit() } catch(e) { waiting.value = false; toast.error(String(e)) }
}
watch(active, value => { if (!value && waiting.value) void exit() })
let unlisten: (() => void) | undefined
onMounted(async () => {
  if (!isTauri()) return
  unlisten = await getCurrentWindow().onCloseRequested(async event => {
    event.preventDefault()
    if (active.value) { closing.value = true; return }
    await exit()
  })
})
onUnmounted(() => unlisten?.())
</script>
<template>
  <AppTooltip text="任务中心"><Button variant="ghost" class="m-2 shrink-0" aria-label="任务中心" @click="open = true"><ClipboardListIcon data-icon="inline-start" /><template v-if="!compact">任务中心 <Badge variant="secondary">{{ count }}</Badge></template></Button></AppTooltip>
  <Dialog v-model:open="open"><DialogContent class="flex max-h-[85dvh] flex-col sm:max-w-2xl"><DialogHeader><DialogTitle>任务中心</DialogTitle><DialogDescription>查看后台任务、执行结果和上次启动时中断的任务。Git 写入失败后请先检查仓库状态。</DialogDescription></DialogHeader>
    <Tabs v-model="filter"><TabsList aria-label="任务筛选"><TabsTrigger value="all">全部</TabsTrigger><TabsTrigger value="running">执行中</TabsTrigger><TabsTrigger value="error">失败</TabsTrigger><TabsTrigger value="stopped">已停止</TabsTrigger><TabsTrigger value="success">成功</TabsTrigger></TabsList></Tabs>
    <ScrollArea class="h-[50dvh] min-h-0"><div class="flex flex-col gap-3 pr-3"><p v-if="!rows.length" class="py-6 text-sm text-muted-foreground">暂无此类任务</p><div v-for="task in rows" :key="task.id" class="flex flex-col gap-2 rounded-lg border p-3"><div class="flex items-center justify-between gap-2"><p class="text-sm font-medium">{{ task.label }}</p><Badge variant="secondary">{{ {running:'执行中',success:'成功',error:'失败',stopped:'已停止'}[task.state] }}</Badge></div><p class="break-all text-xs text-muted-foreground">{{ displayPath(task.path) }} · {{ task.at }}</p><p v-if="task.state === 'error'" class="text-sm">{{ explainGitError(task.detail).title }}：{{ explainGitError(task.detail).action }}</p><div class="flex flex-wrap gap-2"><Button v-if="task.clone" size="sm" variant="outline" @click="open = false; clone.dialogOpen = true">查看克隆详情</Button><Button v-else size="sm" variant="outline" @click="locate(task.path)">查看仓库</Button><Button v-if="task.state === 'error' && task.label === 'Fetch 获取远端'" size="sm" variant="outline" :disabled="active" @click="retryFetch(task.path)">重试 Fetch</Button><Button v-if="task.detail" size="sm" variant="ghost" @click="copy(task.detail)">复制详细日志</Button></div></div></div></ScrollArea>
  </DialogContent></Dialog>
  <Dialog v-model:open="closing"><DialogContent><DialogHeader><DialogTitle>还有任务正在执行</DialogTitle><DialogDescription>{{ waiting ? '任务结束后将自动退出。' : '关闭应用会中断任务。可以返回继续使用，或等待任务完成后退出；停止克隆会保留已下载文件。' }}</DialogDescription></DialogHeader><DialogFooter class="flex-wrap"><Button variant="outline" @click="waiting = false; closing = false">返回应用</Button><Button v-if="!waiting" variant="outline" @click="waiting = true">完成后退出</Button><Button v-if="clone.running && !workspace.operationBusy && !gitActivities.some(t => t.status === 'running')" :disabled="waiting" @click="stopAndExit">停止克隆并退出</Button></DialogFooter></DialogContent></Dialog>
</template>
