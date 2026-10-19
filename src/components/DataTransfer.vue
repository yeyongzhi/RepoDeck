<script setup lang="ts">
import { ref, computed } from 'vue'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import { toast } from 'vue-sonner'
import { useWorkspaceStore } from '@/stores/workspace'
import { useCloneStore } from '@/stores/clone'
import { useGroupsStore } from '@/stores/groups'
import { useSettingsStore } from '@/stores/settings'
import { initializeBusiness, flushBusiness } from '@/lib/business'
import { gitActivities, initializeActivities, setActivityLimit } from '@/lib/git-client'
import { displayPath } from '@/lib/path'
import { Button } from '@/components/ui/button'
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '@/components/ui/card'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '@/components/ui/dialog'
import { ScrollArea } from '@/components/ui/scroll-area'
const workspace = useWorkspaceStore(); const clone = useCloneStore(); const groups = useGroupsStore(); const settings = useSettingsStore()
const preferenceFile = ref(''); const preferencePreview = ref<Record<string,unknown> | null>(null)
const loading = ref(false); const file = ref(''); const preview = ref<{repositories:number;groups:number;missing:string[]} | null>(null)
const unavailable = computed(() => loading.value || clone.running || workspace.operationBusy || workspace.isLoading || gitActivities.value.some(t => t.status === 'running'))
async function exportData() {
  if (!isTauri()) return toast.info('请在桌面应用中导出本机数据')
  try {
    const path = await save({defaultPath:'repodeck-data.json',filters:[{name:'RepoDeck 数据包',extensions:['json']}]})
    if (!path) return
    loading.value = true; await clone.flushTasks(); await flushBusiness(); await invoke('export_app_data',{path}); toast.success('业务数据已导出；不包含仓库代码、Git 凭据和界面偏好')
  } catch(e) { toast.error(String(e)) } finally { loading.value = false }
}
async function chooseImport() {
  try { const path = await open({multiple:false,filters:[{name:'RepoDeck 数据包',extensions:['json']}]}); if (typeof path !== 'string') return; file.value = path; preview.value = await invoke('preview_app_import',{path}) } catch(e) { toast.error(String(e)) }
}
async function confirmImport() {
  if (unavailable.value) return
  loading.value = true
  try {
    await flushBusiness(); await invoke('import_app_data',{path:file.value}); await initializeBusiness(); await initializeActivities(); setActivityLimit(settings.options.activityLimit); clone.reloadSavedTasks(); await clone.loadDirectories(); await groups.initialize(true); await workspace.refresh(); preview.value = null; toast.success('已合并导入，原数据已自动备份；界面偏好保留')
  } catch(e) { toast.error(String(e)) } finally { loading.value = false }
}
async function exportPreferences() {
  try { await settings.flushSettings(); const path = await save({defaultPath:'repodeck-preferences.json',filters:[{name:'偏好 JSON',extensions:['json']}]}); if(path) {await invoke('export_preferences',{path}); toast.success('偏好已导出')} } catch(e) {toast.error(String(e))}
}
async function choosePreferences() {
  try { const path = await open({multiple:false,filters:[{name:'偏好 JSON',extensions:['json']}]}); if(typeof path === 'string') {preferenceFile.value=path; preferencePreview.value=await invoke('import_preferences',{path,apply:false})} } catch(e) {toast.error(String(e))}
}
async function applyPreferences() {
  try {await invoke('import_preferences',{path:preferenceFile.value,apply:true}); await settings.initialize(); preferencePreview.value=null; toast.success('偏好已导入并生效')} catch(e) {toast.error(String(e))}
}
</script>
<template><Card><CardHeader><CardTitle>本地数据库与迁移</CardTitle><CardDescription>业务数据保存在 SQLite；外观、默认工具和窗口设置保存在 JSON。导入仅合并登记信息，不复制或改动仓库代码。</CardDescription></CardHeader><CardContent class="flex flex-col gap-3"><p class="break-all text-sm">数据库：{{ settings.dataLocation ? `${displayPath(settings.dataLocation)}/data/repodeck.sqlite` : '网页预览不访问本机数据' }}</p><p class="break-all text-sm">偏好：{{ settings.dataLocation ? `${displayPath(settings.dataLocation)}/setting/preferences.json` : '—' }}</p><div class="flex flex-wrap gap-2"><Button :disabled="unavailable" @click="exportData">导出业务数据</Button><Button variant="outline" :disabled="unavailable" @click="chooseImport">导入业务数据</Button><Button variant="outline" :disabled="unavailable" @click="exportPreferences">导出偏好 JSON</Button><Button variant="outline" :disabled="unavailable" @click="choosePreferences">导入偏好 JSON</Button></div><p class="text-xs text-muted-foreground">包含仓库路径、分组、置顶、目录历史、最近访问和任务记录。导入前自动备份至安装目录 backup，重复项目合并；路径失效后可重新定位。</p></CardContent></Card>
<Dialog :open="preview !== null" @update:open="value => { if (!value && !loading) preview = null }"><DialogContent><DialogHeader><DialogTitle>确认合并导入？</DialogTitle><DialogDescription>保留已有数据，合并同名分组与仓库。不会执行任何 Git 操作或启动外部工具。</DialogDescription></DialogHeader><p>仓库 {{ preview?.repositories }} 个 · 分组 {{ preview?.groups }} 个</p><ScrollArea v-if="preview?.missing.length" class="h-40"><p class="text-sm">以下路径当前不可访问，导入后可重新定位：</p><p v-for="path in preview.missing" :key="path" class="break-all text-xs">{{ displayPath(path) }}</p></ScrollArea><DialogFooter><Button variant="outline" :disabled="loading" @click="preview = null">取消</Button><Button :disabled="unavailable" @click="confirmImport">{{ loading ? '导入中…' : '合并导入' }}</Button></DialogFooter></DialogContent></Dialog><Dialog :open="preferencePreview !== null" @update:open="value => {if(!value) preferencePreview=null}"><DialogContent><DialogHeader><DialogTitle>导入界面偏好？</DialogTitle><DialogDescription>替换外观、布局和默认开发工具等偏好。原偏好将自动备份，仓库登记保持不变。</DialogDescription></DialogHeader><ScrollArea class="h-44"><pre class="whitespace-pre-wrap break-all text-xs">{{ JSON.stringify(preferencePreview,null,2) }}</pre></ScrollArea><DialogFooter><Button variant="outline" @click="preferencePreview=null">取消</Button><Button @click="applyPreferences">导入并应用</Button></DialogFooter></DialogContent></Dialog></template>
