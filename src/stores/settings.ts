import { invoke, isTauri } from '@tauri-apps/api/core'
import { toast } from 'vue-sonner'
import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { usePreferredDark } from '@vueuse/core'
import { defaultAppOptions, normalizeAppOptions } from '@/lib/app-options'
import { readPreferences, normalizeCommitLimit } from '@/lib/preferences'
import type { AccentColor, ThemeMode } from '@/lib/preferences'

export const useSettingsStore = defineStore('settings', () => {
  let saved = readPreferences(null)
  try { saved = readPreferences((localStorage.getItem('repodeck.preferences') ?? localStorage.getItem('gitscope.preferences')), (localStorage.getItem('repodeck.commitLimit') ?? localStorage.getItem('gitscope.commitLimit'))) } catch { /* Storage may be unavailable. */ }
  let storedOptions: unknown = null
  try { storedOptions = JSON.parse(localStorage.getItem('repodeck.preferences') || '{}').options } catch {}
  const options = ref(normalizeAppOptions(storedOptions))
  const theme = ref<ThemeMode>(saved.theme)
  const accent = ref<AccentColor>(saved.accent)
  const accentSource = ref<'palette' | 'custom'>('palette')
  const commitLimit = ref(saved.commitLimit)
  const sidebarWidth = ref(240)
  const borderRadius = ref(16)
  const fontSize = ref(14)
  const dataLocation = ref('')
  const rememberedLayout = ref<'grid' | 'list'>(localStorage.getItem('repodeck.repository-layout') === 'list' ? 'list' : 'grid')
  const editorPath = ref('')
  const terminalPath = ref('')
  let ready = !isTauri()
  let saveQueue = Promise.resolve()
  async function initialize() {
    if (!isTauri()) return
    ready = false
    try {
      dataLocation.value = await invoke<string>('get_data_location')
      const value = await invoke<Record<string, unknown> | null>('read_preferences')
      if (value) {
        rememberedLayout.value = value.rememberedLayout === 'list' ? 'list' : 'grid'
        editorPath.value = typeof value.editorPath === 'string' ? value.editorPath : ''
        terminalPath.value = typeof value.terminalPath === 'string' ? value.terminalPath : ''
        options.value = normalizeAppOptions(value.options)
        accentSource.value = value.accentSource === 'custom' ? 'custom' : 'palette'
        const normalized = readPreferences(JSON.stringify(value))
        theme.value = normalized.theme; accent.value = normalized.accent; commitLimit.value = normalized.commitLimit
        if (typeof value.sidebarWidth === 'number' && Number.isFinite(value.sidebarWidth)) sidebarWidth.value = Math.round(Math.min(480, Math.max(200, value.sidebarWidth)))
        if ([0,4,8,12,16,24].includes(Number(value.borderRadius))) borderRadius.value = Number(value.borderRadius)
        if ([12,14,16,18,20,24].includes(Number(value.fontSize))) fontSize.value = Number(value.fontSize)
      }
      ready = true
      if (!value) await invoke('save_preferences', { preferences: { options: options.value, theme: theme.value, accent: accent.value, accentSource: accentSource.value, commitLimit: commitLimit.value, sidebarWidth: sidebarWidth.value, borderRadius: borderRadius.value, fontSize: fontSize.value, rememberedLayout: rememberedLayout.value, editorPath: editorPath.value, terminalPath: terminalPath.value } })
    } catch (reason) { toast.error(String(reason)) }
  }
  const prefersDark = usePreferredDark()
  const isDark = computed(() => theme.value === 'dark' || (theme.value === 'system' && prefersDark.value))

  watch([isDark, accent], () => {
    document.documentElement.classList.toggle('dark', isDark.value)
    document.documentElement.dataset.accent = accent.value
    const custom = /^#[0-9a-f]{6}$/i.test(accent.value)
    for (const token of ['--primary','--accent','--ring']) document.documentElement.style.removeProperty(token)
    if (custom) {
      document.documentElement.style.setProperty('--primary', `color-mix(in srgb, ${accent.value} ${isDark.value ? '65%, white' : '75%, black'})`)
      document.documentElement.style.setProperty('--accent', `color-mix(in srgb, ${accent.value} 14%, ${isDark.value ? '#20232b' : 'white'})`)
      document.documentElement.style.setProperty('--ring', 'var(--primary)')
    }
    document.documentElement.style.colorScheme = isDark.value ? 'dark' : 'light'
  }, { immediate: true })

  watch([borderRadius, fontSize], () => {
    document.documentElement.style.setProperty('--radius', `${borderRadius.value}px`)
    document.documentElement.style.fontSize = `${fontSize.value}px`
  }, { immediate: true })
  watch([options, theme, accent, accentSource, commitLimit, sidebarWidth, borderRadius, fontSize, rememberedLayout, editorPath, terminalPath], () => {
    if (!ready) return
    const preferences = { options: normalizeAppOptions(options.value), theme: theme.value, accent: accent.value, accentSource: accentSource.value, commitLimit: normalizeCommitLimit(commitLimit.value), sidebarWidth: sidebarWidth.value, borderRadius: borderRadius.value, fontSize: fontSize.value, rememberedLayout: rememberedLayout.value, editorPath: editorPath.value, terminalPath: terminalPath.value }
    if (isTauri()) {
      saveQueue = saveQueue.then(() => invoke('save_preferences', { preferences })).then(() => {}).catch(reason => { toast.error(String(reason)) })
      return
    }
    try {
      localStorage.setItem('repodeck.preferences', JSON.stringify(preferences))
    } catch { /* Keep in-memory preferences working when storage is disabled. */ }
  }, { flush: 'sync', deep: true })

  function resetCategory(category: string) {
    if (category === '通用') { editorPath.value = ''; terminalPath.value = ''; rememberedLayout.value = 'grid' }
    if (category === '通用') for (const key of ['startup', 'repositoryLayout', 'repositoryTab', 'dateFormat'] as const) options.value[key] = defaultAppOptions[key]
    if (category === '外观') { theme.value = 'system'; accent.value = 'blue'; accentSource.value = 'palette'; sidebarWidth.value = 240; borderRadius.value = 16; fontSize.value = 14; options.value.density = 'standard' }
    if (category === '仓库') { options.value.repositorySort = 'name'; options.value.startupScan = 'on' }
    if (category === '数据') options.value.activityLimit = 300
  }
  watch(() => options.value.density, value => { document.documentElement.dataset.density = value }, { immediate: true })
  function flushSettings() { return saveQueue }
  return { flushSettings, rememberedLayout, editorPath, terminalPath, options, resetCategory, theme, accent, accentSource, commitLimit, isDark, sidebarWidth, borderRadius, fontSize, dataLocation, initialize }
})
