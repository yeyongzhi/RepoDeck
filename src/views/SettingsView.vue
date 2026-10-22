<script setup lang="ts">
import { ArrowLeftIcon, CheckIcon, GitBranchIcon, MonitorIcon, MoonIcon, SunIcon } from '@lucide/vue'
import { version as packageVersion } from '../../package.json'
import { storeToRefs } from 'pinia'
import { ref } from 'vue'
import { toast } from 'vue-sonner'
import { watchDebounced } from '@vueuse/core'
import { useRouter } from 'vue-router'
import { useSettingsStore } from '@/stores/settings'
import type { ThemeMode } from '@/lib/preferences'
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { ScrollArea } from '@/components/ui/scroll-area'
import ToolSettings from '@/components/ToolSettings.vue'
import DataTransfer from '@/components/DataTransfer.vue'
import SettingsOptions from '@/components/SettingsOptions.vue'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '@/components/ui/dialog'
import ThemePalette from '@/components/ThemePalette.vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Field, FieldContent, FieldDescription, FieldGroup, FieldLabel, FieldTitle } from '@/components/ui/field'
import { Separator } from '@/components/ui/separator'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'

const router = useRouter()
function goBack() {
 if (router.options.history.state.back) router.back()
 else void router.push('/')
}
const category = ref('通用')
const resetOpen = ref(false)
const settings = useSettingsStore()
const { theme, sidebarWidth, borderRadius, fontSize } = storeToRefs(settings)
watchDebounced(() => [settings.theme, settings.accent, settings.borderRadius, settings.fontSize, settings.sidebarWidth, settings.options.density], () => { toast.success('外观设置已应用', { id: 'appearance-updated' }) }, { debounce: 500 })
const themeOptions: { value: ThemeMode; label: string; icon: typeof SunIcon }[] = [
  { value: 'light', label: '亮色', icon: SunIcon },
  { value: 'dark', label: '暗色', icon: MoonIcon },
  { value: 'system', label: '跟随系统', icon: MonitorIcon },
]
function setTheme(value: unknown) { if (themeOptions.some((option) => option.value === value)) theme.value = value as ThemeMode }

</script>

<template>
 <div class="flex h-full min-h-0 min-w-0 flex-col gap-3">
  <Card class="shrink-0 flex-row flex-wrap items-center justify-between gap-2 p-2"><div class="flex items-center gap-2"><Button variant="outline" size="icon-sm" aria-label="返回上一页" @click="goBack" title="返回上一页"><ArrowLeftIcon /></Button><nav aria-label="设置分类" class="flex flex-wrap items-center gap-1"><Button v-for="item in ['通用', '外观', '仓库', '数据', '关于']" :key="item" :variant="category === item ? 'default' : 'ghost'" :aria-current="category === item ? 'page' : undefined" @click="category = item">{{ item }}</Button></nav></div><p v-if="category !== '关于'" class="ml-auto px-2 text-right text-sm text-muted-foreground">{{ ({通用:'设置启动与浏览偏好', 外观:'调整界面外观与阅读体验', 仓库:'管理扫描目录与仓库排序', 数据:'管理本机数据与操作记录'})[category] }}</p></Card>
  <div class="flex min-h-0 min-w-0 flex-1 flex-col gap-3">

   <ScrollArea class="min-h-0 min-w-0 flex-1"><div class="flex flex-col gap-4 pr-2 pb-2">
    <ToolSettings v-if="category === '通用'" /><DataTransfer v-if="category === '数据'" /><SettingsOptions :key="category" :category="category" /><Card v-if="category === '数据'"><CardHeader><CardTitle>本地数据</CardTitle><CardDescription>开发版使用独立数据目录；安装版使用安装目录下的 setting 和 data</CardDescription></CardHeader><CardContent><p class="break-all text-sm">{{ settings.dataLocation || '网页预览不访问本机数据' }}</p></CardContent></Card>

    <template v-if="category === '外观'">
<Card class="gap-4 py-4"><CardHeader class="px-4"><CardTitle>主题样式</CardTitle><CardDescription>统一设置界面模式、主题色与布局显示</CardDescription></CardHeader><CardContent class="px-4"><FieldGroup><Field orientation="horizontal" class="settings-row">
                <FieldContent ><FieldTitle id="theme-label">界面模式</FieldTitle><FieldDescription>默认跟随系统，支持跟随系统外观</FieldDescription></FieldContent>
                <Tabs :model-value="theme" class="justify-self-end" @update:model-value="setTheme"><TabsList aria-labelledby="theme-label">
                  <TabsTrigger v-for="option in themeOptions" :key="option.value" :value="option.value"><component :is="option.icon" />{{ option.label }}</TabsTrigger>
                </TabsList></Tabs>
              </Field><Separator /><Field orientation="horizontal" class="settings-row settings-row-top">
                <FieldContent ><FieldTitle id="accent-label">主题色</FieldTitle><FieldDescription>用于主要按钮、选中状态和界面重点</FieldDescription></FieldContent>
<ThemePalette class="justify-end" />
              </Field><Separator /><Field orientation="horizontal" class="settings-row" v-for="option in [
                {key: 'sidebar', label: '侧栏宽度', values: [...new Set([200,220,240,260,280,320,sidebarWidth])].sort((a,b) => a-b), value: sidebarWidth},
                {key: 'radius', label: '全局圆角', values: [0,4,8,12,16,24], value: borderRadius},
                {key: 'font', label: '界面字号', values: [12,14,16,18,20,24], value: fontSize}
              ]" :key="option.key"><FieldLabel :for="option.key">{{ option.label }}</FieldLabel><Select :model-value="String(option.value)" @update:model-value="value => { if (option.key === 'sidebar') sidebarWidth = Number(value); else if (option.key === 'radius') borderRadius = Number(value); else fontSize = Number(value) }"><SelectTrigger :id="option.key" class="w-48 max-w-full justify-self-end"><SelectValue /></SelectTrigger><SelectContent><SelectGroup><SelectItem v-for="value in option.values" :key="value" :value="String(value)">{{ value }} px</SelectItem></SelectGroup></SelectContent></Select></Field></FieldGroup></CardContent></Card>
      <Card class="h-fit gap-5 py-5">
        <CardHeader class="px-5"><CardTitle>效果预览</CardTitle><CardDescription>当前主题会应用到整个工作台</CardDescription></CardHeader>
        <CardContent class="flex flex-col gap-4 px-5">
          <div class="rounded-xl border bg-background p-4">
            <div class="flex items-center gap-3"><span class="flex size-9 items-center justify-center rounded-lg bg-primary text-primary-foreground"><GitBranchIcon class="size-5" /></span><div><p class="text-sm font-medium">示例仓库</p><p class="mt-0.5 text-xs text-muted-foreground">main · 工作区干净</p></div></div>
            <Separator class="my-4" /><div class="flex items-center justify-between gap-2"><Badge variant="secondary"><CheckIcon />已同步</Badge><Button size="sm">主要操作</Button></div>
          </div>
          <p class="flex items-center gap-1.5 text-xs text-muted-foreground"><CheckIcon class="size-3.5 text-success" />外观设置已自动保存</p>
        </CardContent>
        <Separator />
        <CardContent class="flex flex-col items-start gap-1 px-5"><p class="text-sm font-medium">RepoDeck <span class="text-muted-foreground">{{ packageVersion }}</span></p><p class="text-xs leading-5 text-muted-foreground">偏好设置与扫描目录均保存在本机</p></CardContent>
      </Card>

</template><div v-if="category !== '关于'" class="flex justify-end"><Button variant="outline" @click="resetOpen = true">恢复本页默认设置</Button></div></div></ScrollArea></div></div>
<Dialog v-model:open="resetOpen"><DialogContent><DialogHeader><DialogTitle>恢复“{{ category }}”默认设置？</DialogTitle><DialogDescription>仅恢复当前分类的偏好，其他分类及仓库数据保留。</DialogDescription></DialogHeader><DialogFooter><Button variant="outline" @click="resetOpen = false">取消</Button><Button @click="settings.resetCategory(category); resetOpen = false; toast.success('已恢复当前分类的默认设置')">恢复默认</Button></DialogFooter></DialogContent></Dialog>
</template>

<style scoped>
.settings-row {
  display: grid;
  grid-template-columns: minmax(7rem, 1fr) minmax(0, 3fr);
  align-items: center;
  gap: 1.5rem;
}
</style>

<style scoped>
.settings-row-top { align-items: start; }
</style>
