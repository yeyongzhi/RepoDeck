<script setup lang="ts">
import { computed, ref } from 'vue'
import { invoke } from '@/lib/git-client'
import type { RepositorySummary } from '@/types/git'
import { useWorkspaceStore } from '@/stores/workspace'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '@/components/ui/dialog'
import { LoaderCircleIcon } from '@lucide/vue'
const props = defineProps<{ members: RepositorySummary[]; selected: string[]; disabled: boolean }>()
const workspace = useWorkspaceStore()
const results = ref<{id:string; name:string; path:string; status:string; message:string}[]>([])
const open = ref(false); const running = ref(false); const stopped = ref(false); const completed = ref(0); const total = ref(0)
const failures = computed(() => results.value.filter(item => item.status === 'failed'))
const targets = computed(() => props.selected.length ? props.members.filter(item => props.selected.includes(item.id)) : props.members)
async function fetchAll(retry = false) {
 if (running.value || workspace.operationBusy || props.disabled) return
 const list = retry ? failures.value.map(item => ({id:item.id,name:item.name,path:item.path})) : targets.value.map(item => ({id:item.id,name:item.name,path:item.path}))
 if (!list.length) return
 const unique = [...new Map(list.map(item => [item.path.toLowerCase(), item])).values()]
 if (!retry) results.value = []
 open.value = true; running.value = true; stopped.value = false; completed.value = 0; total.value = unique.length; workspace.operationBusy = true
 try {
  for (const item of unique) {
   if (stopped.value) break
   const row = { ...item,status:'running',message:'正在获取远端…' }
   const index = results.value.findIndex(result => result.id === item.id)
   if (index >= 0) results.value[index] = row; else results.value.push(row)
   try { await invoke('fetch_repository', {path:item.path}); row.status = 'success'; row.message = 'Fetch 完成' }
   catch (reason) { row.status = 'failed'; row.message = String(reason) }
   results.value = results.value.map(result => result.id === row.id ? {...row} : result)
   completed.value++
  }
 } finally { await workspace.refresh(); workspace.operationBusy = false; running.value = false }
}
</script>
<template><div class="flex items-center gap-2"><Button size="sm" :disabled="disabled || workspace.operationBusy || !targets.length" @click="fetchAll()"><LoaderCircleIcon v-if="running" data-icon="inline-start" class="animate-spin" />{{ running ? `${completed}/${total}` : selected.length ? `Fetch 所选 · ${targets.length}` : `Fetch 全组 · ${members.length}` }}</Button><Button v-if="results.length" size="sm" variant="outline" @click="open = true">查看结果</Button></div><Dialog v-model:open="open"><DialogContent class="flex max-h-[85dvh] flex-col"><DialogHeader><DialogTitle>批量 Fetch · {{ completed }}/{{ total }}</DialogTitle><DialogDescription>{{ running ? '逐个获取远端引用，单项失败后继续处理其余仓库' : stopped ? '已停止后续任务，当前任务已结束' : '批量获取结束' }}</DialogDescription></DialogHeader><ScrollArea class="h-80 min-h-0 shrink"><div class="flex flex-col gap-2 pr-2"><div v-for="item in results" :key="item.id" class="rounded-md border p-3"><p class="text-sm font-medium">{{ item.name }} · {{ {running:'执行中',success:'成功',failed:'失败'}[item.status] }}</p><p class="mt-1 whitespace-pre-wrap break-all text-xs" :class="item.status === 'failed' ? 'text-destructive' : 'text-muted-foreground'">{{ item.message }}</p></div></div></ScrollArea><DialogFooter><Button v-if="running" variant="outline" :disabled="stopped" @click="stopped = true">{{ stopped ? '等待当前任务结束…' : '停止后续任务' }}</Button><Button v-if="!running && failures.length" @click="fetchAll(true)">仅重试失败项 · {{ failures.length }}</Button><Button variant="outline" @click="open = false">{{ running ? '后台继续' : '关闭' }}</Button></DialogFooter></DialogContent></Dialog></template>
