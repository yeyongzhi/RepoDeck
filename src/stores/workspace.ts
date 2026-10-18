import { invoke } from '@tauri-apps/api/core'
import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { AddRootResult, RepositorySummary, ScanIssue, ScanRoot, ScanSnapshot } from '@/types/git'

export const useWorkspaceStore = defineStore('workspace', () => {
  const roots = ref<ScanRoot[]>([])
  const repositories = ref<RepositorySummary[]>([])
  const issues = ref<ScanIssue[]>([])
  const isLoading = ref(false)
  const operationBusy = ref(false)
  const lastUpdated = ref<Date | null>(null)
  const error = ref('')

  async function refresh(discover = false, scanPath?: string) {
    if (isLoading.value) return
    isLoading.value = true
    error.value = ''
    try {
      const snapshot = await invoke<ScanSnapshot>('scan_repositories', {discover, scanPath: scanPath ?? null})
      roots.value = snapshot.roots
      repositories.value = snapshot.repositories
      issues.value = snapshot.issues
      lastUpdated.value = new Date()
    } catch (reason) {
      error.value = String(reason)
    } finally {
      isLoading.value = false
    }
  }

  async function addRoot(path: string, kind?: 'workspace' | 'repository') {
    const result = await invoke<AddRootResult>('add_scan_root', { path, kind })
    roots.value = result.roots
    await refresh(true, kind === 'workspace' ? path : undefined)
    return result.added
  }

  async function removeRepository(path: string) {
    await invoke('unregister_repository', {path})
    await refresh()
  }

  async function removeRoot(path: string) {
    roots.value = await invoke<ScanRoot[]>('remove_scan_root', { path })
    await refresh()
  }

  return { removeRepository, roots, repositories, issues, isLoading, operationBusy, lastUpdated, error, refresh, addRoot, removeRoot }
})
