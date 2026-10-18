<script setup lang="ts">
import { ref, watch } from 'vue'
import { invoke } from '@/lib/git-client'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from '@/components/ui/dialog'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Button } from '@/components/ui/button'
import { toast } from 'vue-sonner'
const props = defineProps<{ path: string; kind: string; first: string; second?: string; title: string }>()
const open = defineModel<boolean>('open', { default: false })
const data = ref<{ message: string; files: string; diff: string } | null>(null)
const loading = ref(false)
const error = ref('')
let request = 0
watch([open, () => props.path, () => props.first, () => props.second], async () => {
 const id = ++request
 if (!open.value) return
 loading.value = true; error.value = ''; data.value = null
 try { const result = await invoke<typeof data.value>('repository_read', { path: props.path, kind: props.kind, first: props.first, second: props.second }); if (id === request) data.value = result }
 catch (reason) { if (id === request) error.value = String(reason) }
 finally { if (id === request) loading.value = false }
})
async function copy() { try { await navigator.clipboard.writeText(error.value || data.value?.diff || ''); toast.success('已复制') } catch { toast.error('复制失败') } }
</script>
<template>
 <Dialog v-model:open="open"><DialogContent class="top-0 right-0 left-auto flex h-dvh w-full max-w-full translate-x-0 translate-y-0 flex-col rounded-none sm:max-w-3xl"><DialogHeader class="shrink-0 pr-6"><DialogTitle>{{ title }}</DialogTitle><DialogDescription class="break-all">{{ first }}<span v-if="second"> → {{ second }}</span></DialogDescription></DialogHeader><div class="flex shrink-0 justify-end"><Button variant="outline" size="xs" :disabled="loading" @click="copy">{{ error ? '复制错误' : '复制差异' }}</Button></div><ScrollArea class="min-h-0 flex-1"><div class="pr-3"><p v-if="loading" class="text-sm text-muted-foreground">正在读取…</p><p v-else-if="error" class="whitespace-pre-wrap text-sm text-destructive">{{ error }}</p><template v-else-if="data"><pre class="mb-4 whitespace-pre-wrap break-all text-sm">{{ data.message }}</pre><pre v-if="data.files" class="mb-4 whitespace-pre-wrap break-all rounded-md bg-muted p-3 text-xs">{{ data.files }}</pre><div class="overflow-x-auto rounded-md border p-3 font-mono text-xs leading-5"><div v-for="(line, index) in (data.diff || '没有文本差异').split('\n')" :key="index" :class="line.startsWith('+') ? 'text-emerald-700 dark:text-emerald-400' : line.startsWith('-') ? 'text-destructive' : line.startsWith('@@') ? 'text-primary' : ''" class="min-h-5 whitespace-pre">{{ line }}</div></div></template></div></ScrollArea></DialogContent></Dialog>
</template>
