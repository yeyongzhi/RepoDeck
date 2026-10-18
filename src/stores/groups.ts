import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke, isTauri } from '@tauri-apps/api/core'
export interface RepositoryGroup { id: string; name: string; color: string; members: string[] }
export const useGroupsStore = defineStore('groups', () => {
 const groups = ref<RepositoryGroup[]>([])
 const saving = ref(false)
 const error = ref('')
 const editing = ref<string | null>(null)
 const repository = ref<string | null>(null)
 const dialogOpen = ref(false)
 let ready = false
 async function initialize() {
  if (ready) return
  try {
   const value = isTauri() ? await invoke<RepositoryGroup[]>('read_repository_groups') : JSON.parse(localStorage.getItem('repodeck.groups') || '[]')
   if (!Array.isArray(value) || value.some(item => !item || typeof item.id !== 'string' || typeof item.name !== 'string' || !Array.isArray(item.members) || item.members.some((member: unknown) => typeof member !== 'string'))) throw new Error('分组数据格式无效，原数据已保留')
   groups.value = value; ready = true
  } catch (reason) { error.value = String(reason) }
 }
 async function save(value: RepositoryGroup[]) {
  if (saving.value) throw new Error('分组正在保存，请稍后重试')
  if (!ready) throw new Error('分组数据尚未成功读取，请重启应用检查，避免覆盖原数据')
  saving.value = true
  try {
   const snapshot = JSON.parse(JSON.stringify(value)) as RepositoryGroup[]
   if (isTauri()) await invoke('save_repository_groups', {groups: snapshot})
   else localStorage.setItem('repodeck.groups', JSON.stringify(snapshot))
   groups.value = snapshot; error.value = ''
  } finally { saving.value = false }
 }
 function edit(id: string | null = null) { editing.value = id; repository.value = null; dialogOpen.value = true }
 function assign(id: string) { repository.value = id; editing.value = null; dialogOpen.value = true }
 return { groups, saving, error, editing, repository, dialogOpen, initialize, save, edit, assign }
})
