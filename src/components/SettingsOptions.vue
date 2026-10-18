<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { isTauri, invoke } from '@tauri-apps/api/core'
import { toast } from 'vue-sonner'
import { useSettingsStore } from '@/stores/settings'
import { gitActivities, clearGitActivities } from '@/lib/git-client'
import ScanDirectories from '@/components/ScanDirectories.vue'
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '@/components/ui/card'
import { Field, FieldGroup, FieldContent, FieldLabel, FieldDescription } from '@/components/ui/field'
import { Select, SelectTrigger, SelectValue, SelectContent, SelectGroup, SelectItem } from '@/components/ui/select'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { RefreshCwIcon } from '@lucide/vue'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '@/components/ui/dialog'
const props = defineProps<{ category: string }>()
const settings = useSettingsStore()
type OptionKey = keyof typeof settings.options
type Row = { key: OptionKey; label: string; description: string; values: [string, string][] }
const rows = computed<Row[]>(() => props.category === '通用' ? [
 {key:'startup',label:'启动页面',description:'下次启动应用时生效',values:[['last','上次打开的仓库'],['home','仓库首页']]},
 {key:'repositoryLayout',label:'仓库默认布局',description:'记住上次选择，或指定每次打开时的布局',values:[['remember','记住上次选择'],['grid','网格'],['list','列表']]},
 {key:'repositoryTab',label:'仓库默认标签页',description:'进入仓库时默认展示的页面',values:[['overview','总览'],['branches','分支管理'],['commits','提交历史'],['worktree','工作区'],['compare','分支对比'],['stash','暂存区'],['tags','标签管理'],['activity','操作记录']]},
 {key:'dateFormat',label:'日期显示',description:'用于提交时间、仓库日期和操作记录；筛选日期保持完整格式',values:[['full','完整日期（包含年份）'],['relative','相对时间']]},
] : props.category === '外观' ? [
 {key:'density',label:'列表密度',description:'调整列表条目的上下留白',values:[['compact','紧凑'],['standard','标准'],['comfortable','宽松']]},
] : props.category === '仓库' ? [
 {key:'startupScan',label:'启动时扫描固定目录',description:'关闭后只加载已登记仓库；手动扫描仍可发现新仓库',values:[['on','开启'],['off','关闭']]},
 {key:'repositorySort',label:'仓库排序',description:'工作区状态优先显示冲突、未提交改动和待同步仓库',values:[['name','名称'],['recent','最近使用'],['status','工作区状态']]},
] : props.category === '数据' ? [
 {key:'activityLimit',label:'操作记录保留条数',description:'按全部仓库合计保留，降低数量会立即移除超出部分',values:[['100','100 条'],['300','300 条'],['500','500 条'],['1000','1000 条']]},
] : [])
const pending = ref<'clear' | 'trim' | null>(null)
const pendingLimit = ref(300)
const info = ref({version:'读取中…', gitVersion:'读取中…'})
const checking = ref(false)
const updateMessage = ref('')
const releaseUrl = ref('')
async function checkUpdate() {
 if (checking.value) return
 checking.value = true; updateMessage.value = ''; releaseUrl.value = ''
 try {
  const response = await fetch('https://api.github.com/repos/yeyongzhi/RepoDeck/releases/latest', {headers: {Accept: 'application/vnd.github+json'}, signal: AbortSignal.timeout(15000)})
  if (response.status === 404) { updateMessage.value = '暂未找到公开发布的正式版本'; return }
  if (!response.ok) throw new Error(response.status === 403 || response.status === 429 ? '查询次数受限，请稍后重试' : `检查更新失败（${response.status}）`)
  const release = await response.json() as {tag_name?: string}
  const latest = /^v?(\d+)\.(\d+)\.(\d+)$/.exec(release.tag_name ?? '')
  const current = /^v?(\d+)\.(\d+)\.(\d+)(?:-.*)?$/.exec(info.value.version)
  if (!latest || !current) throw new Error('无法识别版本号，请到项目主页查看发布版本')
  let newer = false
  for (let index = 1; index <= 3; index++) {
   if (Number(latest[index]) !== Number(current[index])) { newer = Number(latest[index]) > Number(current[index]); break }
   if (index === 3 && info.value.version.includes('-')) newer = true
  }
  updateMessage.value = newer ? `发现新版本 ${release.tag_name}，当前版本 ${info.value.version}` : `当前已是最新版本（${info.value.version}）`
  if (newer) releaseUrl.value = `https://github.com/yeyongzhi/RepoDeck/releases/tag/${encodeURIComponent(release.tag_name!)}`
 } catch (error) { updateMessage.value = error instanceof Error && error.name === 'TimeoutError' ? '请求超时，请检查网络后重试' : error instanceof Error ? error.message : '检查更新失败，请稍后重试' }
 finally { checking.value = false }
}
function update(key: OptionKey, value: unknown) {
 if (key === 'activityLimit') {
  const limit = Number(value)
  if (gitActivities.value.length > limit) { pendingLimit.value = limit; pending.value = 'trim' }
  else settings.options.activityLimit = limit
 } else (settings.options[key] as string | number) = String(value)
}
function confirm() { if (pending.value === 'clear') clearGitActivities(); else settings.options.activityLimit = pendingLimit.value; pending.value = null; toast.success('操作记录已更新') }
async function openLink(target: string) {
 try { if (isTauri()) await invoke('open_repository_target', {target, local:false}); else window.open(target, '_blank', 'noopener,noreferrer') } catch (error) { toast.error(String(error)) }
}
onMounted(async () => {
 if (!isTauri()) { const {version} = await import('../../package.json'); info.value = {version,gitVersion:'网页预览无法读取本机 Git'}; return }
 try { info.value = await invoke('get_runtime_info') } catch (error) { info.value = {version:'读取失败', gitVersion:String(error)} }
})
</script>
<template>
 <Card v-if="rows.length" class="gap-4 py-4"><CardHeader class="px-4"><CardTitle>{{ {通用:'使用偏好',外观:'列表显示',仓库:'仓库列表',数据:'操作记录'}[category] }}</CardTitle><CardDescription>修改立即生效并自动保存</CardDescription></CardHeader><CardContent class="px-4"><FieldGroup><Field v-for="row in rows" :key="row.key" orientation="horizontal" class="flex-wrap justify-between"><FieldContent><FieldLabel :for="`option-${row.key}`">{{ row.label }}</FieldLabel><FieldDescription>{{ row.description }}</FieldDescription></FieldContent><Select :model-value="String(settings.options[row.key])" @update:model-value="update(row.key, $event)"><SelectTrigger :id="`option-${row.key}`" class="w-52"><SelectValue /></SelectTrigger><SelectContent><SelectGroup><SelectItem v-for="[value, label] in row.values" :key="value" :value="value">{{ label }}</SelectItem></SelectGroup></SelectContent></Select></Field><Field v-if="category === '数据'" orientation="horizontal"><FieldContent><FieldLabel>清空操作记录</FieldLabel><FieldDescription>当前 {{ gitActivities.length }} 条；执行中的记录会保留</FieldDescription></FieldContent><Button variant="destructive-ghost" :disabled="!gitActivities.some(item => item.status !== 'running')" @click="pending = 'clear'">清空记录</Button></Field></FieldGroup></CardContent></Card>
 <ScanDirectories v-if="category === '仓库'" />
 <Card v-if="category === '关于'" class="gap-4 py-4"><CardHeader class="flex flex-row flex-wrap items-start justify-between gap-4 px-4"><div class="flex min-w-0 flex-1 flex-col gap-2"><CardTitle>RepoDeck</CardTitle><CardDescription>开发者的 Git 仓库管理工具 · 仓库井然，开发从容</CardDescription></div><div class="flex shrink-0 gap-2"><Button variant="outline" size="sm" @click="openLink('https://github.com/yeyongzhi/RepoDeck')">项目主页</Button><Button variant="outline" size="sm" @click="openLink('https://github.com/yeyongzhi/RepoDeck/issues/new')">反馈问题</Button></div></CardHeader><CardContent class="flex flex-col items-start gap-4 px-4"><Badge variant="secondary" class="max-w-full whitespace-normal break-all">{{ info.gitVersion }}</Badge><Button size="sm" :disabled="checking || info.version === '读取中…' || info.version === '读取失败'" @click="checkUpdate"><RefreshCwIcon data-icon="inline-start" :class="{'animate-spin': checking}" />{{ checking ? '检查中…' : '检查更新' }}</Button><p v-if="updateMessage" role="status" class="text-sm text-muted-foreground">{{ updateMessage }}</p><Button v-if="releaseUrl" variant="outline" size="sm" @click="openLink(releaseUrl)">查看新版本</Button></CardContent></Card>
 <Dialog :open="pending !== null" @update:open="value => { if (!value) pending = null }"><DialogContent><DialogHeader><DialogTitle>{{ pending === 'clear' ? '清空操作记录？' : '减少记录保留数量？' }}</DialogTitle><DialogDescription>{{ pending === 'clear' ? '移除本机已完成的操作记录，Git 仓库和提交历史会保留。' : `只保留最近 ${pendingLimit} 条已完成记录，超出部分将被移除。` }}</DialogDescription></DialogHeader><DialogFooter><Button variant="outline" @click="pending = null">取消</Button><Button variant="destructive" @click="confirm">确认</Button></DialogFooter></DialogContent></Dialog>
</template>
