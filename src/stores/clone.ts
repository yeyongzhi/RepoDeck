import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { Channel, invoke, isTauri } from '@tauri-apps/api/core'
import { useWorkspaceStore } from './workspace'
import { toast } from 'vue-sonner'
import { businessValues, saveBusiness } from '@/lib/business'
import { useGroupsStore } from './groups'
import { cloneProgress } from '@/lib/clone-progress'

export interface CloneTask { id: string; url: string; parent: string; groupId?: string; name: string; path: string; register: boolean; percent: number; state: 'running' | 'stopping' | 'stopped' | 'success' | 'error'; output: string; error: string; deleted: boolean }
export const useCloneStore = defineStore('clone', () => {
  const recoverTasks = () => ((businessValues.clones as CloneTask[]) ?? []).map(task => ['running','stopping'].includes(task.state) ? {...task,state:'error' as const,error:'应用退出时克隆未完成，请检查目录后重试。'} : task)
  const tasks = ref<CloneTask[]>(recoverTasks())
  function reloadSavedTasks() { tasks.value = recoverTasks() }
  let persistTimer: ReturnType<typeof setTimeout> | undefined
  function flushTasks() { clearTimeout(persistTimer); return saveBusiness('clones',tasks.value.slice(0,100)) }
  watch(tasks, () => { clearTimeout(persistTimer); persistTimer = setTimeout(() => { void flushTasks().catch(e => toast.error(String(e))) },250) },{deep:true})
  const running = ref(false)
  const dialogOpen = ref(false)
  const url = ref('')
  const parent = ref('')
  const name = ref('')
  const quick = ref(false)
  const register = ref(true)
  const groupId = ref('none')
  const directories = ref<string[]>([])
  async function loadDirectories() {
    try {
      directories.value = isTauri() ? await invoke<string[]>('read_clone_directories') : JSON.parse(localStorage.getItem('repodeck.cloneDirectories') || '[]')
      if (!parent.value && directories.value.length) parent.value = directories.value[0]!
    } catch (e) { toast.error(`读取历史目录失败：${e}`) }
  }
  async function rememberDirectory(path: string) {
    if (isTauri()) directories.value = await invoke<string[]>('remember_clone_directory', { path })
    else {
      directories.value = [path, ...directories.value.filter(item => item.toLowerCase() !== path.toLowerCase())].slice(0, 8)
      localStorage.setItem('repodeck.cloneDirectories', JSON.stringify(directories.value))
    }
  }
  function openDialog(quickClone = false) {
    if (!running.value) { quick.value = quickClone; register.value = !quickClone }
    dialogOpen.value = true
    void loadDirectories()
  }
  async function start() {
    if (running.value) return
    const task = ref<CloneTask>({ id: crypto.randomUUID(), url: url.value.trim().replace(/(https?:\/\/)[^/@]+@/,'$1'), parent: parent.value.trim(), groupId: groupId.value, name: name.value.trim(), path: `${parent.value.replace(/[\\/]+$/, '')}/${name.value.trim()}`, register: register.value, percent: 0, state: 'running', output: '', error: '', deleted: false })
    tasks.value.unshift(task.value)
    running.value = true
    try { await rememberDirectory(parent.value.trim()) } catch (e) { toast.error(`保存历史目录失败：${e}`) }
    const progress = new Channel<string>()
    progress.onmessage = chunk => {
      task.value.output = (task.value.output + chunk.replaceAll('\r', '\n')).slice(-16000)
      task.value.percent = cloneProgress(task.value.output, task.value.percent)
    }
    try {
      await flushTasks()
      const result = await invoke<{path: string; stopped: boolean; deleted: boolean}>('clone_repository', { id: task.value.id, url: url.value.trim(), parent: parent.value.trim(), name: name.value.trim(), register: task.value.register, progress })
      task.value.path = result.path
      task.value.deleted = result.deleted
      task.value.state = result.stopped ? 'stopped' : 'success'
      if (!result.stopped) {
        task.value.percent = 100
        if (task.value.register) await useWorkspaceStore().refresh()
        if (task.value.register && task.value.groupId && task.value.groupId !== 'none') {
          const groups = useGroupsStore(); const repo = useWorkspaceStore().repositories.find(r => r.path.replaceAll('\\','/').toLowerCase() === result.path.replaceAll('\\','/').toLowerCase())
          if (repo) await groups.save(groups.groups.map(g => g.id === task.value.groupId ? {...g,members:[...new Set([...g.members,repo.id])]} : g)).catch(e => { task.value.error = `克隆和登记已完成，但加入分组失败：${e}。请在仓库列表管理分组。` })
        }
        if (task.value.register && useWorkspaceStore().error) task.value.error = `克隆已完成，列表刷新失败：${useWorkspaceStore().error}`
      }
      if (task.value.error) toast.warning(task.value.error)
      else toast.success(result.stopped ? `克隆已停止${result.deleted ? '，本次文件已删除' : '，已下载文件保留'}` : `${task.value.name} 克隆完成`)
    } catch (reason) { task.value.state = 'error'; task.value.error = String(reason); toast.error(task.value.error) }
    finally { running.value = false; await flushTasks().catch(e => toast.error(String(e))) }
  }
  async function stop(id: string, deleteFiles: boolean) {
    const task = tasks.value.find(task => task.id === id)
    if (!task || task.state !== 'running') return
    task.state = 'stopping'
    try { await invoke('stop_clone', { id, deleteFiles }) }
    catch (e) { if (task.state === 'stopping') task.state = 'running'; throw e }
  }
  async function retry(task: CloneTask) {
    if (running.value) return
    url.value = task.url ?? ''; parent.value = task.parent ?? ''; name.value = `${task.name}-retry-${Date.now().toString().slice(-6)}`
    register.value = task.register; quick.value = !task.register; groupId.value = task.groupId ?? 'none'; dialogOpen.value = true
    if (!url.value) return toast.info('旧任务未保存远程地址，请重新输入后克隆')
    await start()
  }
  async function removeDirectory(path?: string) {
    const next = path ? directories.value.filter(item => item !== path) : []
    await saveBusiness('directories',next); directories.value = next; toast.success(path ? '历史目录已移除' : '历史目录已清空')
  }
  async function registerExisting(task: CloneTask) {
    if (running.value) return
    try { await useWorkspaceStore().addRoot(task.path,'repository'); task.register = true; task.state = 'success'; task.percent = 100; task.error = ''; await flushTasks(); toast.success('已登记克隆仓库') } catch(e) { toast.error(String(e)) }
  }
  return { registerExisting, reloadSavedTasks, loadDirectories, groupId, retry, removeDirectory, flushTasks, tasks, running, dialogOpen, url, parent, name, quick, register, directories, openDialog, rememberDirectory, start, stop }
})
