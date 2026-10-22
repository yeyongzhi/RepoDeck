<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { isTauri, invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { toast } from 'vue-sonner'
import { DownloadIcon, FolderOpenIcon, LoaderCircleIcon, TerminalIcon, CodeIcon } from '@lucide/vue'
import { useCloneStore } from '@/stores/clone'
import { useWorkspaceStore } from '@/stores/workspace'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Switch } from '@/components/ui/switch'
import { Progress } from '@/components/ui/progress'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '@/components/ui/dialog'
import { useGroupsStore } from '@/stores/groups'
import { explainGitError } from '@/lib/git-errors'
import { Select, SelectTrigger, SelectValue, SelectContent, SelectGroup, SelectItem } from '@/components/ui/select'
import { displayPath } from '@/lib/path'
import { Field, FieldGroup, FieldLabel, FieldDescription } from '@/components/ui/field'
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '@/components/ui/card'
import { Alert, AlertTitle, AlertDescription } from '@/components/ui/alert'
import { Badge } from '@/components/ui/badge'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Empty, EmptyHeader, EmptyTitle, EmptyDescription } from '@/components/ui/empty'
const clone = useCloneStore()
const groups = useGroupsStore()
async function copyError(value: string) { try { await navigator.clipboard.writeText(value); toast.success('错误日志已复制') } catch { toast.error('复制失败') } }
async function openDirectory(path: string) { try { await invoke('open_repository_target',{target:path,local:true}); toast.success('已请求打开目录') } catch(e) { toast.error(String(e)) } }
const stopId = ref<string | null>(null)
const logValue = ref<string | null>(null)
async function confirmStop(deleteFiles: boolean) {
  if (!stopId.value) return
  try { await clone.stop(stopId.value, deleteFiles); stopId.value = null; toast.info(deleteFiles ? '已请求停止克隆并删除文件' : '已请求停止克隆并保留文件') } catch (e) { toast.error(String(e)); stopId.value = null }
}
const workspace = useWorkspaceStore()
const router = useRouter()
const suggestion = (url: string) => url.trim().replace(/\/+$/, '').split(/[/:]/).at(-1)?.replace(/\.git$/, '') ?? ''
watch(() => clone.url, (value, old) => { if (!clone.name || clone.name === suggestion(old)) clone.name = suggestion(value) })
const valid = computed(() => !!clone.url.trim() && !!clone.parent.trim() && !!clone.name.trim() && !/[<>:"/\\|?*]/.test(clone.name))
const target = computed(() => `${clone.parent.replace(/[\\/]+$/, '')}${clone.parent.includes('\\') ? '\\' : '/'}${clone.name}`)
async function choose() {
  if (!isTauri()) return toast.info('请在桌面应用中选择本机目录')
  try { const path = await open({ directory: true, multiple: false, title: '选择代码存放目录' }); if (typeof path === 'string') { clone.parent = path; await clone.rememberDirectory(path) } } catch (e) { toast.error(String(e)) }
}
async function start() {
  if (!isTauri()) return toast.info('请在桌面应用中克隆仓库')
  if (valid.value) await clone.start()
}
async function view(path: string) {
  await workspace.refresh()
  const repo = workspace.repositories.find(repo => repo.path.replace(/^\\\\\?\\/, '').replaceAll('\\', '/').toLowerCase() === path.replace(/^\\\\\?\\/, '').replaceAll('\\', '/').toLowerCase())
  if (repo) await router.push({ name: 'repository', params: { id: repo.id } })
  else toast.error(workspace.error || '仓库暂未加载，请刷新仓库列表')
}
async function openApp(path: string, kind: string) { try { await invoke('open_repository_app', { path, kind }); toast.success(kind === 'editor' ? '已请求在编辑器打开' : '已请求打开终端') } catch (e) { toast.error(String(e)) } }
</script>

<template>
  <Dialog v-model:open="clone.dialogOpen"><DialogContent class="flex max-h-[90dvh] flex-col overflow-hidden sm:max-w-3xl"><DialogHeader><DialogTitle>{{ clone.quick ? '快速克隆仓库' : '从远程添加仓库' }}</DialogTitle><DialogDescription>{{ clone.register ? '克隆完成后自动加入仓库列表。' : '仅克隆代码到本机。' }}关闭窗口后任务继续执行。</DialogDescription></DialogHeader><ScrollArea class="h-[60dvh] min-h-0"><div class="flex flex-col gap-5 p-1">
    <Card><CardHeader><CardTitle>克隆仓库</CardTitle><CardDescription>输入 Git 地址，将代码下载到本机。</CardDescription></CardHeader>
      <CardContent><form class="flex flex-col gap-5" @submit.prevent="start"><FieldGroup>
        <Field><FieldLabel for="clone-url">Git 仓库地址</FieldLabel><Input id="clone-url" v-model="clone.url" placeholder="https://github.com/owner/repository.git" :disabled="clone.running" required /><FieldDescription>支持 HTTPS 和 SSH；私有仓库使用本机已配置的 Git 凭据或 SSH 密钥。</FieldDescription></Field>
        <Field><FieldLabel for="clone-parent">存放目录</FieldLabel><div class="flex gap-2"><Input id="clone-parent" v-model="clone.parent" placeholder="选择或输入已有目录" :disabled="clone.running" required /><Button type="button" variant="outline" :disabled="clone.running" @click="choose"><FolderOpenIcon data-icon="inline-start" />选择目录</Button></div><template v-if="clone.directories.length"><FieldDescription>历史选择目录：点击即可快速选择</FieldDescription><div class="flex gap-2"><Button type="button" size="sm" variant="ghost" :disabled="clone.running" @click="clone.removeDirectory().catch(e => toast.error(String(e)))">清空历史</Button></div><div class="flex flex-wrap gap-2"><div v-for="directory in clone.directories" :key="directory" class="flex max-w-full items-center gap-1"><Button type="button" variant="outline" size="sm" class="h-auto max-w-full whitespace-normal break-all text-left" :disabled="clone.running" @click="clone.rememberDirectory(directory).then(() => { clone.parent = directory }).catch(e => toast.error(String(e)))">{{ displayPath(directory) }}</Button><Button type="button" size="icon-sm" variant="ghost" :aria-label="`移除历史目录 ${displayPath(directory)}`" :disabled="clone.running" @click="clone.removeDirectory(directory).catch(e => toast.error(String(e)))" :title="`移除历史目录 ${displayPath(directory)}`">×</Button></div></div></template></Field>
        <Field v-if="clone.quick" orientation="horizontal"><Switch id="quick-clone-register" v-model="clone.register" :disabled="clone.running" /><FieldLabel for="quick-clone-register">克隆后加入仓库列表</FieldLabel></Field>
        <Field><FieldLabel for="clone-name">项目目录名</FieldLabel><Input id="clone-name" v-model="clone.name" :disabled="clone.running" placeholder="repository" required /><FieldDescription v-if="clone.parent && clone.name" class="break-all">将克隆到：{{ target }}</FieldDescription></Field>
        <Field v-if="clone.register"><FieldLabel>加入分组（可选）</FieldLabel><Select v-model="clone.groupId" :disabled="clone.running"><SelectTrigger aria-label="克隆后加入分组"><SelectValue placeholder="不加入分组" /></SelectTrigger><SelectContent><SelectGroup><SelectItem value="none">不加入分组</SelectItem><SelectItem v-for="group in groups.groups" :key="group.id" :value="group.id">{{ group.name }}</SelectItem></SelectGroup></SelectContent></Select></Field>
      </FieldGroup><div><Button type="submit" :disabled="!valid || clone.running"><LoaderCircleIcon v-if="clone.running" data-icon="inline-start" class="animate-spin" /><DownloadIcon v-else data-icon="inline-start" />{{ clone.running ? '正在克隆…' : clone.register ? '克隆并添加' : '克隆' }}</Button></div></form></CardContent>
    </Card>
    <Card :class="{ 'order-first': clone.running }"><CardHeader><CardTitle>克隆任务 <Badge variant="secondary">{{ clone.tasks.length }}</Badge></CardTitle><CardDescription>本次启动期间的任务；切换页面后任务会继续执行。</CardDescription></CardHeader>
      <CardContent class="flex flex-col gap-4"><Empty v-if="!clone.tasks.length"><EmptyHeader><EmptyTitle>暂无克隆任务</EmptyTitle><EmptyDescription>从上方添加第一个项目。</EmptyDescription></EmptyHeader></Empty>
        <Card v-for="task in clone.tasks" :key="task.id"><CardHeader><div class="flex items-center justify-between gap-2"><CardTitle class="truncate">{{ task.name }}</CardTitle><Badge :variant="task.state === 'error' ? 'destructive' : 'secondary'"><LoaderCircleIcon v-if="task.state === 'running'" class="animate-spin" />{{ { running: '克隆中', stopping: '停止中', stopped: '已停止', success: '已完成', error: '失败' }[task.state] }} · {{ task.percent }}%</Badge></div><CardDescription v-if="task.path" class="break-all">{{ displayPath(task.path) }}</CardDescription></CardHeader><CardContent class="flex flex-col gap-3">
          <div class="flex items-center justify-between gap-2"><p class="text-xs text-muted-foreground">总体进度{{ task.state === 'success' ? '' : '（按 Git 阶段估算）' }} · {{ task.percent }}%</p><Button v-if="task.state === 'running' || task.state === 'stopping'" size="sm" variant="outline" :disabled="task.state === 'stopping'" @click="stopId = task.id">{{ task.state === 'stopping' ? '停止中…' : '停止' }}</Button></div>
          <Progress :model-value="task.percent" :aria-label="`${task.name} 克隆总体进度`" />
          <p v-if="task.state === 'stopped'" class="text-sm text-muted-foreground">{{ task.deleted ? '已停止，已删除本次克隆的文件。' : '已停止，已保留下载文件，未加入仓库列表；文件可能不完整。' }}</p>
          <Alert v-if="task.error" :variant="task.state === 'error' ? 'destructive' : 'default'"><AlertTitle>{{ task.state === 'error' ? explainGitError(task.error).title : '需要确认' }}</AlertTitle><AlertDescription class="whitespace-pre-wrap break-all">{{ explainGitError(task.error).action }}</AlertDescription></Alert>
          <div v-if="task.state === 'error' || task.state === 'stopped'" class="flex flex-wrap gap-2"><Button v-if="task.error.includes('代码已克隆')" size="sm" variant="outline" :disabled="clone.running" @click="clone.registerExisting(task)">登记已克隆仓库</Button><Button size="sm" variant="outline" :disabled="clone.running" @click="clone.retry(task)">重试到新目录</Button><Button size="sm" variant="outline" @click="openDirectory(task.path)">打开目录</Button><Button size="sm" variant="outline" @click="copyError(task.error || task.output)">复制日志</Button></div>
          <p v-if="task.state === 'success'" class="text-sm text-muted-foreground">{{ task.register ? '克隆完成，已加入仓库列表。' : '克隆完成，仅下载代码，未加入仓库列表。' }}</p>
          <Button v-if="task.error" size="sm" variant="ghost" @click="logValue = task.error">查看详细日志</Button>
          <ScrollArea v-if="task.output" class="h-40 rounded-md bg-muted p-3"><pre class="whitespace-pre-wrap break-all text-xs" aria-live="off">{{ task.output }}</pre></ScrollArea><p v-else-if="task.state === 'running'" role="status" class="text-sm text-muted-foreground">正在连接远程仓库…</p>
          <div v-if="task.state === 'success'" class="flex flex-wrap gap-2"><Button v-if="task.register" size="sm" @click="view(task.path)">查看仓库</Button><Button size="sm" variant="outline" @click="openApp(task.path, 'editor')"><CodeIcon data-icon="inline-start" />在默认编辑器打开</Button><Button size="sm" variant="outline" @click="openApp(task.path, 'terminal')"><TerminalIcon data-icon="inline-start" />打开终端</Button></div>
        </CardContent></Card>
      </CardContent>
    </Card>
  </div></ScrollArea><DialogFooter><Button variant="outline" @click="clone.dialogOpen = false">{{ clone.running ? '后台继续' : '关闭' }}</Button></DialogFooter></DialogContent></Dialog>
  <Dialog :open="stopId !== null" @update:open="value => { if (!value) stopId = null }"><DialogContent><DialogHeader><DialogTitle>停止克隆？</DialogTitle><DialogDescription>终止当前克隆进程，选择如何处理本次已下载的文件。保留的文件可能不完整；删除会永久移除本次克隆创建的项目目录。</DialogDescription></DialogHeader><DialogFooter class="flex-wrap"><Button variant="outline" @click="stopId = null">继续克隆</Button><Button variant="outline" @click="confirmStop(false)">停止并保留文件</Button><Button variant="destructive" @click="confirmStop(true)">停止并删除文件</Button></DialogFooter></DialogContent></Dialog>
<Dialog :open="logValue !== null" @update:open="value => { if (!value) logValue = null }"><DialogContent><DialogHeader><DialogTitle>详细日志</DialogTitle><DialogDescription>可复制日志用于排查问题。</DialogDescription></DialogHeader><ScrollArea class="h-64"><pre class="whitespace-pre-wrap break-all text-xs">{{ logValue }}</pre></ScrollArea></DialogContent></Dialog>
</template>
