<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue'
import { toast } from 'vue-sonner'
import { invoke, gitActivities } from '@/lib/git-client'
import { useSettingsStore } from '@/stores/settings'
import { formatAppDate } from '@/lib/app-options'
import { useWorkspaceStore } from '@/stores/workspace'
import type { RepositorySummary } from '@/types/git'
import { Button } from '@/components/ui/button'
import { LoaderCircleIcon, FolderOpenIcon, GitPullRequestIcon } from '@lucide/vue'
import { Popover, PopoverTrigger, PopoverContent } from '@/components/ui/popover'
const props = defineProps<{ repo: RepositorySummary; disabled: boolean }>()
const emit = defineEmits<{ updated: [] }>()
const workspace = useWorkspaceStore()
const settings = useSettingsStore()
const running = ref('')
const result = ref('')
const failed = ref(false)
const activePanel = ref<'shortcuts' | 'sync' | null>(null)
let keyboardInteraction = false
function keyboardOpen(event: KeyboardEvent) {
 if (['Enter', ' ', 'ArrowDown'].includes(event.key)) keyboardInteraction = true
}
function autoFocus(event: Event) {
 if (!keyboardInteraction) event.preventDefault()
}
let closeTimer: ReturnType<typeof setTimeout> | undefined
function showPanel(panel: 'shortcuts' | 'sync') {
 keyboardInteraction = false
 clearTimeout(closeTimer)
 activePanel.value = panel
}
function hidePanel() {
 clearTimeout(closeTimer)
 closeTimer = setTimeout(() => { activePanel.value = null }, 200)
}
function updatePanel(panel: 'shortcuts' | 'sync', value: boolean) {
 clearTimeout(closeTimer)
 activePanel.value = value ? panel : activePanel.value === panel ? null : activePanel.value
}
onBeforeUnmount(() => clearTimeout(closeTimer))
const disabled = computed(() => props.disabled || workspace.operationBusy || workspace.isLoading || !!running.value)
const fetched = computed(() => gitActivities.value.find(item => item.path === props.repo.path && item.label === 'Fetch 获取远端' && item.status === 'success')?.at)
async function open(kind: string) { try { await invoke('open_repository_app', {path: props.repo.path, kind}) } catch (reason) { toast.error(String(reason)) } }
async function copy() { try { await navigator.clipboard.writeText(props.repo.path); toast.success('路径已复制') } catch { toast.error('复制失败') } }
async function sync(command: string, label: string) {
 if (disabled.value) return
 running.value = label; result.value = ''; failed.value = false; workspace.operationBusy = true
 try {
  if (command === 'pull_fast_forward') await invoke('fetch_repository', {path: props.repo.path})
  await invoke(command, {path: props.repo.path}); result.value = `${label}完成`
  toast.success(result.value)
 } catch (reason) { failed.value = true; result.value = String(reason); toast.error(result.value) }
 finally { await workspace.refresh(); workspace.operationBusy = false; running.value = ''; emit('updated') }
}
</script>
<template>
 <div class="flex items-center gap-3" role="group" aria-label="仓库操作">
  <slot />
  <Popover :open="activePanel === 'shortcuts'" @update:open="updatePanel('shortcuts', $event)"><span class="inline-flex" @mouseenter="showPanel('shortcuts')" @mouseleave="hidePanel"><PopoverTrigger as-child><Button variant="outline" size="icon-sm" aria-label="仓库快捷入口" @keydown="keyboardOpen"><FolderOpenIcon /></Button></PopoverTrigger></span><PopoverContent align="end" class="w-[150px] max-w-[calc(100vw-2rem)] p-[10px]" @mouseenter="showPanel('shortcuts')" @mouseleave="hidePanel" @open-auto-focus="autoFocus" @close-auto-focus="autoFocus"><div class="flex flex-col gap-2" aria-label="仓库快捷操作"><Button variant="outline" size="sm" class="w-full justify-start" @click="open('editor')">在 VS Code 打开</Button><Button variant="outline" size="sm" class="w-full justify-start" @click="open('terminal')">打开终端</Button><Button variant="outline" size="sm" class="w-full justify-start" @click="copy">复制路径</Button></div></PopoverContent></Popover>
  <Popover :open="activePanel === 'sync'" @update:open="updatePanel('sync', $event)"><span class="inline-flex" @mouseenter="showPanel('sync')" @mouseleave="hidePanel"><PopoverTrigger as-child><Button variant="outline" size="icon-sm" aria-label="远程同步" @keydown="keyboardOpen"><LoaderCircleIcon v-if="running" class="animate-spin" /><GitPullRequestIcon v-else /></Button></PopoverTrigger></span><PopoverContent align="end" class="w-[150px] max-w-[calc(100vw-2rem)] p-[10px]" @mouseenter="showPanel('sync')" @mouseleave="hidePanel" @open-auto-focus="autoFocus" @close-auto-focus="autoFocus"><div class="flex flex-col gap-3"><div class="flex flex-col gap-2" aria-label="远程同步操作"><Button variant="outline" size="sm" class="w-full justify-start" :disabled="disabled" @click="sync('fetch_repository', 'Fetch')">Fetch</Button><Button variant="outline" size="sm" class="w-full justify-start" :disabled="disabled || !repo.upstream || repo.isDirty || repo.isDetached" @click="sync('pull_fast_forward', 'Pull')">Pull</Button><Button variant="outline" size="sm" class="w-full justify-start" :disabled="disabled || !repo.upstream || repo.isDetached" @click="sync('push_current_branch', 'Push')">Push</Button></div><p class="text-xs text-muted-foreground">{{ fetched ? `最近 Fetch：${formatAppDate(fetched, settings.options.dateFormat)}` : '尚无 Fetch 记录' }}</p><p v-if="running || result" class="max-h-28 overflow-auto whitespace-pre-wrap break-all text-xs" :class="failed ? 'text-destructive' : 'text-muted-foreground'" role="status">{{ running ? `${running} 执行中…` : result }}</p></div></PopoverContent></Popover>
 </div>
</template>
